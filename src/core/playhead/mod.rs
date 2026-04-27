use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};

#[cfg(target_arch = "wasm32")]
use std::sync::mpsc::Receiver;

use std::sync::Arc;
use std::sync::Mutex;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

use crate::core::geom::Vec2;

use crate::core::engine::symspell::SymSpellState;
use crate::core::io::midi as io_midi;
use crate::core::{consts, engine::regex::Match, utils};
use crate::view::rect::Rect;

// Existing core submodules
pub mod midi;
pub mod movement;
pub mod position;
pub mod queue;
pub mod tilt;

// Moved from view/playhead_handler
pub mod accumulation;
pub mod aim;
pub mod modes;
pub mod music;
pub mod transform;
pub mod types;

#[cfg(test)]
pub mod test_helpers;

pub use types::{Direction, GridState, Message, ModeFlags, MusicState, PlayheadUI, UIUpdate};

use self::midi::MidiHandlerConfig;
use self::midi::MidiTriggerHandler;
use self::movement::Movement;
use self::position::PositionCalculator;
use self::queue::QueueManager;
use self::tilt::TiltMode;
use super::playhead::types::SweepRowMode;

/// The main playhead controller managing movement, MIDI triggering, and queue operations.
///
/// Architecture:
/// - **Subsystems**: Delegates queue operations (QueueManager), position calculations
///   (PositionCalculator), and MIDI triggering (MidiTriggerHandler) to specialized modules
/// - **State Organization**: Groups related fields into GridState, MusicState, and ModeFlags
///   for clarity and shared access across subsystems
/// - **Threading**: Uses Arc/Mutex for shared state and AtomicBool/AtomicUsize for
///   performance-critical flags (accumulation, ratcheting, step tracking)
/// - **UI Integration**: Batches UI updates via ui_update_queue to reduce UI thread pressure
pub struct Playhead {
  /// position of the top-left corner of the playhead area in the grid
  pos: Arc<Mutex<Vec2>>,
  /// area of the playhead, defined by the top-left corner (pos) and its width and height
  area: Arc<Mutex<Rect>>,
  drag_start_x: AtomicUsize,
  drag_start_y: AtomicUsize,
  /// current running position of the playhead within, relative to the playhead area (0,0)
  actived_pos: Arc<Mutex<Vec2>>,
  /// position captured when freeze mode was enabled; MIDI retriggers here while frozen
  frozen_active_pos: Arc<Mutex<Vec2>>,
  /// aimed area for Ctrl+hjkl aiming feature
  aimed_area: Arc<Mutex<Option<Rect>>>,

  // Consolidated state
  grid: GridState,
  music: MusicState,
  pub modes: ModeFlags,

  // Pattern matching state
  regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,

  // Subsystems
  queue_manager: Arc<QueueManager>,
  position_calc: Arc<PositionCalculator>,
  midi_handler: Arc<MidiTriggerHandler>,

  // Accumulation mode state
  accumulation_counter: Arc<Mutex<usize>>,
  movement: Arc<Mutex<Movement>>,
  tilt_mode: Arc<Mutex<TiltMode>>,
  sweep_row_mode: Arc<Mutex<SweepRowMode>>,

  // Threading/sync state
  pub ui_tx: Sender<UIUpdate>,
  step_index: Arc<Mutex<usize>>,
  ratchet_generation: Arc<AtomicUsize>,
  /// Remaining fast-division steps within the current regex match span.
  match_span_remaining: Arc<AtomicUsize>,

  pub sym_state: Arc<SymSpellState>,

  /// WASM only: receiver for the UI update channel, drained each frame in wasm.rs.  
  #[cfg(target_arch = "wasm32")]
  pub ui_rx: Mutex<Option<Receiver<UIUpdate>>>,

  /// WASM only: the receiver for the playhead message channel, stored here
  /// because `run()` would normally give it to a background thread.
  #[cfg(target_arch = "wasm32")]
  pub wasm_rx: std::sync::Mutex<Option<std::sync::mpsc::Receiver<Message>>>,
}

impl Playhead {
  pub fn new(midi_tx: Sender<io_midi::Message>, ui_tx: Sender<UIUpdate>) -> Self {
    let position_calc = Arc::new(PositionCalculator::new());

    let grid = GridState::new();
    let music = MusicState::new();
    let modes = ModeFlags::new();
    let prev_active_pos = Arc::new(Mutex::new(Vec2::zero()));
    let ratchet_generation = Arc::new(AtomicUsize::new(0));
    let pos = Arc::new(Mutex::new(Vec2::zero()));
    let tilt_mode = Arc::new(Mutex::new(TiltMode::default()));
    let sweep_row_mode = Arc::new(Mutex::new(SweepRowMode::default()));
    let aimed_area = Arc::new(Mutex::new(None));

    let midi_handler = Arc::new(MidiTriggerHandler::new(
      MidiHandlerConfig {
        midi_tx: midi_tx.clone(),
        prev_active_pos: Arc::clone(&prev_active_pos),
        ratchet_generation: Arc::clone(&ratchet_generation),
        text_matcher: Arc::clone(&position_calc.text_matcher),
        tilt_mode: Arc::clone(&tilt_mode),
        playhead_pos: Arc::clone(&pos),
        sweep_row_mode: Arc::clone(&sweep_row_mode),
      },
      &grid,
      &music,
      &modes,
    ));

    Playhead {
      pos,
      area: Arc::new(Mutex::new(Rect::from_size(
        Vec2::zero(),
        Vec2::new(
          consts::PLAYHEAD_INIT_AREA_WIDTH,
          consts::PLAYHEAD_INIT_AREA_HEIGHT,
        ),
      ))),
      drag_start_x: AtomicUsize::new(0),
      drag_start_y: AtomicUsize::new(0),
      actived_pos: Arc::new(Mutex::new(Vec2::zero())),
      frozen_active_pos: Arc::new(Mutex::new(Vec2::zero())),
      aimed_area,
      grid,
      music,
      modes,
      regex_indexes: Arc::clone(&position_calc.regex_indexes),
      text_matcher: Arc::clone(&position_calc.text_matcher),
      movement: Arc::clone(&position_calc.movement),
      tilt_mode,
      sweep_row_mode,
      queue_manager: Arc::new(QueueManager::new()),
      position_calc,
      midi_handler,
      accumulation_counter: Arc::new(Mutex::new(0)),
      ui_tx,
      step_index: Arc::new(Mutex::new(0)),
      ratchet_generation,
      match_span_remaining: Arc::new(AtomicUsize::new(0)),
      sym_state: Arc::new(SymSpellState::new()),
      #[cfg(target_arch = "wasm32")]
      wasm_rx: std::sync::Mutex::new(None),
      #[cfg(target_arch = "wasm32")]
      ui_rx: std::sync::Mutex::new(None),
    }
  }

  pub(super) fn compute_chn_str(&self, pos: Vec2) -> String {
    let v = self.grid.v_splits.load(Ordering::Relaxed).max(1);
    let h = self.grid.h_splits.load(Ordering::Relaxed).max(1);
    let gw = self.grid.width.load(Ordering::Relaxed).max(1);
    let gh = self.grid.height.load(Ordering::Relaxed).max(1);
    let col_w = if v > 1 { (gw / v).max(1) } else { gw };
    let row_h = if h > 1 { (gh / h).max(1) } else { gh };
    let col_idx = (pos.x / col_w).min(v.saturating_sub(1));
    let row_idx = (pos.y / row_h).min(h.saturating_sub(1));
    let ch = row_idx * v + col_idx;
    format!("{}/{}", ch + 1, v * h)
  }

  pub fn set_text_matcher(&self, text_matcher: Option<HashMap<usize, Match>>) {
    let mut tm = self.text_matcher.lock().unwrap();
    *tm = text_matcher
  }

  pub(super) fn execute_front_op_in_queue(&self) {
    use self::queue::{EventOperator, QueueItem};

    let op_front = self.queue_manager.get_front_item();
    match op_front {
      None => (),
      Some(QueueItem::Position(_, _)) => (),
      Some(QueueItem::Event(ev_op)) => {
        let actived_pos = *self.actived_pos.lock().unwrap();
        let playhead_pos = *self.pos.lock().unwrap();
        let abs_x = playhead_pos.x + actived_pos.x;
        let abs_y = playhead_pos.y + actived_pos.y;

        match ev_op {
          EventOperator::H => {
            self.midi_handler.h_op();
          }
          EventOperator::C => {
            let (_abs_x, _abs_y, curr_running_playhead) =
              self.calculate_absolute_position(actived_pos);
            let distance_to_next = if !self.modes.dyn_length_mode.load(Ordering::Relaxed)
              || self.modes.arpeggiator_mode.load(Ordering::Relaxed)
            {
              4
            } else {
              self.find_distance_to_next_trigger(curr_running_playhead)
            };
            self.midi_handler.c_op(abs_x, abs_y, distance_to_next);
          }
          EventOperator::R => {
            self.midi_handler.r_op(abs_x, abs_y);
          }
        }
      }
    }
  }

  fn handle_update_info_status_view(&self) {
    let pos = self.pos.lock().unwrap();
    let area = self.area.lock().unwrap();
    let pos_x = pos.x;
    let pos_y = pos.y;
    let w = area.width();
    let h = area.height();
    drop(pos);
    drop(area);

    let chn_str = self.compute_chn_str(Vec2::new(pos_x, pos_y));
    let _ = self
      .ui_tx
      .send(UIUpdate::PosStatus(utils::build_pos_status_str(
        (pos_x, pos_y).into(),
      )));
    let _ = self
      .ui_tx
      .send(UIUpdate::LenStatus(utils::build_len_status_str((w, h))));
    let _ = self.ui_tx.send(UIUpdate::ChnStatus(chn_str));
  }

  fn handle_set_matcher(&self, matcher: Option<HashMap<usize, Match>>) {
    self.set_text_matcher(matcher);

    let text_matcher = self.text_matcher.lock().unwrap();
    let mm = text_matcher.clone();
    drop(text_matcher);
    let regex_indexes_cloned = self.regex_indexes.clone();
    let _ = self.ui_tx.send(UIUpdate::TextMatcher {
      matcher: mm,
      regex_indexes: regex_indexes_cloned,
    });
  }

  fn handle_set_grid_size(&self, width: usize, height: usize) {
    self.grid.width.store(width, Ordering::Relaxed);
    self.grid.height.store(height, Ordering::Relaxed);

    self
      .position_calc
      .grid_width
      .store(width, Ordering::Relaxed);
    self
      .position_calc
      .grid_height
      .store(height, Ordering::Relaxed);

    let queue_manager_cloned = self.queue_manager.clone();
    let _ = self
      .ui_tx
      .send(UIUpdate::QueueManagerUpdate(queue_manager_cloned));
  }

  fn handle_set_grid_splits(&self, v: usize, h: usize) {
    self.grid.v_splits.store(v, Ordering::Relaxed);
    self.grid.h_splits.store(h, Ordering::Relaxed);
    let pos = *self.pos.lock().unwrap();
    let chn_str = self.compute_chn_str(pos);
    let _ = self.ui_tx.send(UIUpdate::GridSplits(v, h));
    let _ = self.ui_tx.send(UIUpdate::ChnStatus(chn_str));
  }

  fn handle_clear_queue(&self) {
    self.queue_manager.clear_all();
    self.reset_accumulation_counter();
    let _ = self.ui_tx.send(UIUpdate::InputStatus("-".to_string()));
  }

  /// WASM: set up channels without spawning any threads.
  /// Returns the `Sender` for keybindings; the `Receiver` is stored inside
  /// the `Playhead` and drained by `wasm_tick()` each frame.
  #[cfg(target_arch = "wasm32")]
  pub fn wasm_setup(self: Arc<Self>) -> Sender<Message> {
    let (tx, rx) = channel();
    *self.wasm_rx.lock().unwrap() = Some(rx);
    tx
  }

  /// WASM: drain pending messages from the channel and process them.
  #[cfg(target_arch = "wasm32")]
  pub fn wasm_tick(self: &Arc<Self>) {
    let guard = self.wasm_rx.lock().unwrap();
    if let Some(rx) = guard.as_ref() {
      while let Ok(control_message) = rx.try_recv() {
        match control_message {
          Message::Move(direction, canvas_size) => {
            self.handle_movement(direction, 1, canvas_size);
          }
          Message::Leap(direction, canvas_size) => {
            let leap_steps = match direction {
              Direction::Up | Direction::Down => 4,
              Direction::Left | Direction::Right => 8,
              Direction::Idle => 0,
            };
            self.handle_movement(direction, leap_steps, canvas_size);
          }
          Message::SetCurrentPos(position, offset) => {
            self.handle_set_current_pos(position, offset);
          }
          Message::UpdateInfoStatusView() => {
            self.handle_update_info_status_view();
          }
          Message::SetGridArea(current_pos) => {
            self.handle_set_grid_area(current_pos);
          }
          Message::SetActivePos(tick) => {
            self.handle_set_active_pos(tick);
          }
          Message::Scale(dir) => {
            self.handle_scale(dir);
          }
          Message::SetMatcher(matcher) => {
            self.handle_set_matcher(matcher);
          }
          Message::SetGridSize(width, height) => {
            self.handle_set_grid_size(width, height);
          }
          Message::SetScaleModeLeft(scale_mode) => {
            self.handle_set_scale_mode_left(scale_mode);
          }
          Message::SetScaleRootTop(scale_root) => {
            self.handle_set_scale_root_top(scale_root);
          }
          Message::SetScaleModeTop(scale_mode) => {
            self.handle_set_scale_mode_top(scale_mode);
          }
          Message::ToggleAccumulationMode() => {
            self.handle_toggle_accumulation_mode();
          }
          Message::SetTempo(bpm) => {
            self.handle_set_tempo(bpm);
          }
          Message::SetRatio(new_ratio) => {
            self.handle_set_ratio(new_ratio);
          }
          Message::ToggleForwardMode() => {
            self.switch_movement(movement::Movement::Forward);
          }
          Message::ToggleReverseMode() => {
            self.switch_movement(movement::Movement::Reverse);
          }
          Message::ToggleArpeggiatorMode() => {
            self.toggle_arpeggiator_mode();
          }
          Message::ToggleRandomMode() => {
            self.switch_movement(movement::Movement::Random);
          }
          Message::TogglePendulumMode() => {
            self.switch_movement(movement::Movement::Pendulum);
          }
          Message::ToggleEventOperatorMode() => {
            self.toggle_event_operator_mode();
          }
          Message::ToggleDrainQueueMode() => {
            self.toggle_drain_queue_mode();
          }
          Message::ToggleSweepMode() => {
            self.toggle_sweep_mode();
          }
          Message::ToggleDroneMode() => {
            self.toggle_drone_mode();
          }
          Message::MoveDrone(dir) => {
            self.move_drone(dir);
          }
          Message::CycleDroneChannel(adj) => {
            self.cycle_drone_channel(adj);
          }
          Message::ToggleDynLengthMode() => {
            self.toggle_dyn_length_mode();
          }
          Message::ToggleFreezeMode() => {
            self.toggle_freeze_mode();
          }
          Message::CycleScaleRootTop(dir) => {
            self.cycle_scale_root(dir);
          }
          Message::CycleScaleMode(dir) => {
            self.cycle_scale_mode(dir);
          }
          Message::CycleScaleRootLeft(dir) => {
            self.cycle_scale_root_left(dir);
          }
          Message::CycleScaleModeLeft(dir) => {
            self.cycle_scale_mode_left(dir);
          }
          Message::ClearQueue() => {
            self.handle_clear_queue();
          }
          Message::SetGridSplits(v, h) => {
            self.handle_set_grid_splits(v, h);
          }
          Message::CycleTiltMode() => {
            self.cycle_tilt_mode();
          }
          Message::CycleSweepRowMode() => {
            self.cycle_sweep_row_mode();
          }
          Message::StartAim() => {
            self.handle_start_aim();
          }
          Message::UpdateAim(direction, canvas_size, step) => {
            self.handle_update_aim(direction, canvas_size, step);
          }
          Message::CommitAim() => {
            self.handle_commit_aim();
          }
          Message::CancelAim() => {
            self.handle_cancel_aim();
          }
          Message::ToggleSweepMovementMode() => {
            self.toggle_sweep_movement_mode();
          }
          Message::SetSweepMovement(mv) => {
            self.set_sweep_movement(mv);
          }
          Message::CycleSweepOutputMode() => {
            self.cycle_sweep_output_mode();
          }
          Message::AdjustSweepCC(adj) => {
            self.adjust_sweep_cc(adj);
          }
          Message::ToggleSpatialKeyboard() => {
            let prev = self.modes.keyboard_top_active.load(Ordering::Relaxed);
            let new_val = !prev;
            self
              .modes
              .keyboard_top_active
              .store(new_val, Ordering::Relaxed);
            let _ = self.ui_tx.send(UIUpdate::CanvasKeyboardTopActive(new_val));
          }
        }
      }
    }
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn run(self: Arc<Self>) -> Sender<Message> {
    let (tx, rx) = channel();

    thread::Builder::new()
      .name(crate::core::consts::THREAD_NAME_PLAYHEAD.to_string())
      .spawn(move || {
        for control_message in &rx {
          match control_message {
            Message::Move(direction, canvas_size) => {
              self.handle_movement(direction, 1, canvas_size);
            }
            Message::Leap(direction, canvas_size) => {
              let leap_steps = match direction {
                Direction::Up | Direction::Down => 4,
                Direction::Left | Direction::Right => 8,
                Direction::Idle => 0,
              };
              self.handle_movement(direction, leap_steps, canvas_size);
            }
            Message::SetCurrentPos(position, offset) => {
              self.handle_set_current_pos(position, offset);
            }
            Message::UpdateInfoStatusView() => {
              self.handle_update_info_status_view();
            }
            Message::SetGridArea(current_pos) => {
              self.handle_set_grid_area(current_pos);
            }
            Message::SetActivePos(tick) => {
              self.handle_set_active_pos(tick);
            }
            Message::Scale(dir) => {
              self.handle_scale(dir);
            }
            Message::SetMatcher(matcher) => {
              self.handle_set_matcher(matcher);
            }
            Message::SetGridSize(width, height) => {
              self.handle_set_grid_size(width, height);
            }
            Message::SetScaleModeLeft(scale_mode) => {
              self.handle_set_scale_mode_left(scale_mode);
            }
            Message::SetScaleRootTop(scale_root) => {
              self.handle_set_scale_root_top(scale_root);
            }
            Message::SetScaleModeTop(scale_mode) => {
              self.handle_set_scale_mode_top(scale_mode);
            }
            Message::ToggleAccumulationMode() => {
              self.handle_toggle_accumulation_mode();
            }
            Message::SetTempo(bpm) => {
              self.handle_set_tempo(bpm);
            }
            Message::SetRatio(new_ratio) => {
              self.handle_set_ratio(new_ratio);
            }
            Message::ToggleForwardMode() => {
              self.switch_movement(Movement::Forward);
            }
            Message::ToggleReverseMode() => {
              self.switch_movement(Movement::Reverse);
            }
            Message::ToggleArpeggiatorMode() => {
              self.toggle_arpeggiator_mode();
            }
            Message::ToggleRandomMode() => {
              self.switch_movement(Movement::Random);
            }
            Message::TogglePendulumMode() => {
              self.switch_movement(Movement::Pendulum);
            }
            Message::ToggleEventOperatorMode() => {
              self.toggle_event_operator_mode();
            }
            Message::ToggleDrainQueueMode() => {
              self.toggle_drain_queue_mode();
            }
            Message::ToggleSweepMode() => {
              self.toggle_sweep_mode();
            }
            Message::ToggleDroneMode() => {
              self.toggle_drone_mode();
            }
            Message::MoveDrone(dir) => {
              self.move_drone(dir);
            }
            Message::CycleDroneChannel(adj) => {
              self.cycle_drone_channel(adj);
            }
            Message::ToggleDynLengthMode() => {
              self.toggle_dyn_length_mode();
            }
            Message::ToggleFreezeMode() => {
              self.toggle_freeze_mode();
            }
            Message::CycleScaleRootTop(dir) => {
              self.cycle_scale_root(dir);
            }
            Message::CycleScaleMode(dir) => {
              self.cycle_scale_mode(dir);
            }
            Message::CycleScaleRootLeft(dir) => {
              self.cycle_scale_root_left(dir);
            }
            Message::CycleScaleModeLeft(dir) => {
              self.cycle_scale_mode_left(dir);
            }
            Message::ClearQueue() => {
              self.handle_clear_queue();
            }
            Message::SetGridSplits(v, h) => {
              self.handle_set_grid_splits(v, h);
            }
            Message::CycleTiltMode() => {
              self.cycle_tilt_mode();
            }
            Message::CycleSweepRowMode() => {
              self.cycle_sweep_row_mode();
            }
            Message::StartAim() => {
              self.handle_start_aim();
            }
            Message::UpdateAim(direction, canvas_size, step) => {
              self.handle_update_aim(direction, canvas_size, step);
            }
            Message::CommitAim() => {
              self.handle_commit_aim();
            }
            Message::CancelAim() => {
              self.handle_cancel_aim();
            }
            Message::ToggleSweepMovementMode() => {
              self.toggle_sweep_movement_mode();
            }
            Message::SetSweepMovement(mv) => {
              self.set_sweep_movement(mv);
            }
            Message::CycleSweepOutputMode() => {
              self.cycle_sweep_output_mode();
            }
            Message::AdjustSweepCC(adj) => {
              self.adjust_sweep_cc(adj);
            }
            Message::ToggleSpatialKeyboard() => {
              let prev = self.modes.keyboard_top_active.load(Ordering::Relaxed);
              let new_val = !prev;
              self
                .modes
                .keyboard_top_active
                .store(new_val, Ordering::Relaxed);
              let _ = self.ui_tx.send(UIUpdate::CanvasKeyboardTopActive(new_val));
            }
          }
        }
      })
      .expect("Failed to spawn playhead thread");

    tx
  }
}

#[cfg(test)]
mod tests {
  use super::test_helpers::make_playhead;
  use crate::core::geom::Vec2;
  use std::sync::atomic::Ordering;

  #[test]
  fn chn_str_no_splits_is_always_1_of_1() {
    let ph = make_playhead();
    ph.grid.width.store(8, Ordering::Relaxed);
    ph.grid.height.store(8, Ordering::Relaxed);
    ph.grid.v_splits.store(1, Ordering::Relaxed);
    ph.grid.h_splits.store(1, Ordering::Relaxed);
    assert_eq!(ph.compute_chn_str(Vec2::new(0, 0)), "1/1");
    assert_eq!(ph.compute_chn_str(Vec2::new(7, 7)), "1/1");
  }

  #[test]
  fn chn_str_2x2_splits_maps_quadrants_correctly() {
    let ph = make_playhead();
    ph.grid.width.store(8, Ordering::Relaxed);
    ph.grid.height.store(8, Ordering::Relaxed);
    ph.grid.v_splits.store(2, Ordering::Relaxed);
    ph.grid.h_splits.store(2, Ordering::Relaxed);
    assert_eq!(ph.compute_chn_str(Vec2::new(0, 0)), "1/4"); // top-left
    assert_eq!(ph.compute_chn_str(Vec2::new(4, 0)), "2/4"); // top-right
    assert_eq!(ph.compute_chn_str(Vec2::new(0, 4)), "3/4"); // bottom-left
    assert_eq!(ph.compute_chn_str(Vec2::new(4, 4)), "4/4"); // bottom-right
  }

  #[test]
  fn chn_str_position_at_boundary_does_not_overflow() {
    let ph = make_playhead();
    ph.grid.width.store(4, Ordering::Relaxed);
    ph.grid.height.store(2, Ordering::Relaxed);
    ph.grid.v_splits.store(2, Ordering::Relaxed);
    ph.grid.h_splits.store(1, Ordering::Relaxed);
    assert_eq!(ph.compute_chn_str(Vec2::new(3, 0)), "2/2");
  }
}
