#[cfg(feature = "symspell")]
use ringbuffer::AllocRingBuffer;
#[cfg(feature = "symspell")]
use ringbuffer::RingBuffer;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

#[cfg(feature = "symspell")]
use symspell_rs::SymSpell;
#[cfg(feature = "symspell")]
use symspell_rs::Verbosity;

// #[cfg(debug_assertions)]
// use std::time::Instant;

use cursive::views::Canvas;
#[cfg(feature = "symspell")]
use cursive::views::EditView;
use cursive::views::TextView;
use cursive::Vec2;
use cursive::XY;

use crate::app::AppMode;
use crate::core::command::types;
use crate::core::playhead::midi::MidiTriggerHandler;
use crate::core::playhead::movement::Movement;
use crate::core::playhead::position::PositionCalculator;
use crate::core::playhead::queue::EventOperator;
use crate::core::playhead::queue::PendingJumpPosition;
use crate::core::playhead::queue::QueueItem;
use crate::core::playhead::queue::QueueManager;
use crate::core::playhead::queue::QueueOperator;
use crate::core::playhead::queue::QUEUE_OPERATORS;
use crate::core::playhead::tilt::TiltMode;
use crate::core::tonal::scale;

use crate::core::engine::regex;
use crate::core::{consts, engine::regex::Match, io::midi, utils};
use crate::view::grid_editor::GridEditor;
use crate::view::playhead::Direction;
use crate::view::rect::Rect;
// #[cfg(debug_assertions)]
// use crate::view::timing_diagnostic::TimingStats;

// UI update types for batching
#[derive(Clone, Debug)]
pub enum UIUpdate {
  ActivePos(Vec2),
  AccumulationCounter(usize, usize), // (count, total)
  // OpQueueDisplay(String),
  // EvQueueDisplay(String),
  PlayheadPosAndArea(Vec2, Rect),
  ChnStatus(String),
  GridSplits(usize, usize),
  AimedArea(Option<Rect>),

  #[cfg(feature = "symspell")]
  TmpAppend(usize),
  #[cfg(feature = "symspell")]
  TmpAppendSpace,
  #[cfg(feature = "symspell")]
  RplCycle(Rect),
}

#[cfg(feature = "symspell")]
#[derive(Clone, Debug)]
pub enum RplPendingState {
  Empty,
  Waiting(String, Rect),
  Armed(String, Rect),
}

struct GridParams<'a, R: rand::Rng> {
  pub playhead_width: usize,
  pub playhead_height: usize,
  pub grid_width: usize,
  pub grid_height: usize,
  pub rng: &'a mut R,
}

pub struct PlayheadUI {
  pub playhead_area: Rect,
  pub playhead_pos: Vec2,
  pub actived_pos: Vec2,
  pub aimed_area: Option<Rect>,
  pub text_matcher: Option<HashMap<usize, Match>>,
  pub regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  pub arpeggiator_mode: bool,
  pub accumulation_mode: bool,
  pub sweep_mode: bool,
  pub tilt_mode: TiltMode,
  pub queue_manager: Arc<QueueManager>,
  pub grid_v_splits: usize,
  pub grid_h_splits: usize,
  pub focus_mode: bool,
}

impl PlayheadUI {
  pub fn new() -> Self {
    PlayheadUI {
      playhead_area: Rect::from_point(Vec2::zero()),
      playhead_pos: Vec2::zero(),
      actived_pos: Vec2::zero(),
      aimed_area: None,
      text_matcher: None,
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      arpeggiator_mode: false,
      accumulation_mode: false,
      sweep_mode: false,
      tilt_mode: TiltMode::default(),
      queue_manager: Arc::new(QueueManager::new()),
      grid_v_splits: 1,
      grid_h_splits: 1,
      focus_mode: false,
    }
  }
}

#[derive(Clone, Debug)]
pub enum Message {
  Move(Direction, XY<usize>, cursive::CbSink),
  Leap(Direction, usize, XY<usize>, cursive::CbSink),
  SetCurrentPos(XY<usize>, XY<usize>, cursive::CbSink),
  UpdateInfoStatusView(cursive::CbSink),
  SetGridArea(XY<usize>, cursive::CbSink),
  SetActivePos(usize, cursive::CbSink),
  Scale((i32, i32), cursive::CbSink),
  SetMatcher(Option<HashMap<usize, Match>>, cursive::CbSink),
  SetGridSize(usize, usize, cursive::CbSink),
  SetScaleModeLeft(scale::ScaleMode),
  SetScaleModeTop(scale::ScaleMode),
  SetScaleRootTop(scale::ScaleRoot),
  ToggleAccumulationMode(cursive::CbSink),
  ToggleForwardMode(cursive::CbSink),
  ToggleReverseMode(cursive::CbSink),
  TogglePendulumMode(cursive::CbSink),
  ToggleArpeggiatorMode(cursive::CbSink),
  ToggleRandomMode(cursive::CbSink),
  ToggleEventOperatorMode(cursive::CbSink),
  ToggleDrainQueueMode(cursive::CbSink),
  ToggleSweepMode(cursive::CbSink),
  ToggleDynLengthMode(cursive::CbSink),
  ToggleFreezeMode(cursive::CbSink),
  CycleScaleRootTop(cursive::CbSink, types::Adjustment),
  CycleScaleMode(cursive::CbSink, types::Adjustment),
  SetTempo(usize),
  SetRatio((usize, usize), cursive::CbSink),
  ClearQueue(cursive::CbSink),
  SetGridSplits(usize, usize),
  CycleTiltMode(cursive::CbSink),
  StartAim(),
  UpdateAim(Direction, XY<usize>, usize),
  CommitAim(cursive::CbSink),
  CancelAim(),
}

/// Grid and layout state shared across playhead subsystems.
/// Contains grid dimensions and split configuration for MIDI channel mapping.
pub struct GridState {
  pub width: Arc<AtomicUsize>,
  pub height: Arc<AtomicUsize>,
  pub v_splits: Arc<AtomicUsize>, // vertical splits for channel mapping
  pub h_splits: Arc<AtomicUsize>, // horizontal splits for channel mapping
}

impl GridState {
  pub fn new() -> Self {
    GridState {
      width: Arc::new(AtomicUsize::new(0)),
      height: Arc::new(AtomicUsize::new(0)),
      v_splits: Arc::new(AtomicUsize::new(1)),
      h_splits: Arc::new(AtomicUsize::new(1)),
    }
  }
}

/// Musical parameters and scale configuration shared across playhead subsystems.
/// Contains tempo, scale modes, and timing ratios for MIDI generation.
pub struct MusicState {
  pub tempo: Arc<AtomicUsize>,
  pub scale_mode_left: Arc<Mutex<scale::ScaleMode>>,
  pub scale_mode_top: Arc<Mutex<scale::ScaleMode>>,
  pub scale_root_top: Arc<Mutex<scale::ScaleRoot>>,
  pub ratio: Arc<Mutex<(usize, usize)>>,
}

impl MusicState {
  pub fn new() -> Self {
    MusicState {
      tempo: Arc::new(AtomicUsize::new(consts::DEFAULT_TEMPO)),
      scale_mode_left: Arc::new(Mutex::new(scale::ScaleMode::default())),
      scale_mode_top: Arc::new(Mutex::new(scale::ScaleMode::default())),
      scale_root_top: Arc::new(Mutex::new(scale::ScaleRoot::default())),
      ratio: Arc::new(Mutex::new(consts::DEFAULT_RATIO)),
    }
  }
}

/// Boolean operation mode flags shared across playhead subsystems.
/// Contains toggleable modes for accumulation, arpeggiator, sweep, and MIDI operations.
pub struct ModeFlags {
  pub accumulation_mode: Arc<AtomicBool>,
  pub arpeggiator_mode: Arc<AtomicBool>,
  pub event_operator_mode: Arc<AtomicBool>,
  pub sweep_mode: Arc<AtomicBool>,
  pub dyn_length_mode: Arc<AtomicBool>,
  pub freeze_mode: Arc<AtomicBool>,
  pub hold_next_note: Arc<AtomicBool>,
  pub is_ratcheting: Arc<AtomicBool>,
}

impl ModeFlags {
  pub fn new() -> Self {
    ModeFlags {
      accumulation_mode: Arc::new(AtomicBool::new(false)),
      arpeggiator_mode: Arc::new(AtomicBool::new(false)),
      event_operator_mode: Arc::new(AtomicBool::new(false)),
      sweep_mode: Arc::new(AtomicBool::new(false)),
      dyn_length_mode: Arc::new(AtomicBool::new(false)),
      freeze_mode: Arc::new(AtomicBool::new(false)),
      hold_next_note: Arc::new(AtomicBool::new(false)),
      is_ratcheting: Arc::new(AtomicBool::new(false)),
    }
  }
}

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
pub struct PlayheadArea {
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
  prev_active_pos: Arc<Mutex<Vec2>>,
  /// aimed area for Ctrl+hjkl aiming feature
  aimed_area: Arc<Mutex<Option<Rect>>>,

  // Consolidated state
  grid: GridState,
  music: MusicState,
  modes: ModeFlags,

  // Pattern matching state
  regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,

  // MIDI communication
  // midi_tx: Sender<midi::Message>,

  // Subsystems
  queue_manager: Arc<QueueManager>,
  position_calc: Arc<PositionCalculator>,
  midi_handler: Arc<MidiTriggerHandler>,

  // Accumulation mode state
  accumulation_counter: Arc<Mutex<usize>>,
  movement: Arc<Mutex<Movement>>,
  tilt_mode: Arc<Mutex<TiltMode>>,

  // Threading/sync state
  pub ui_update_queue: Arc<Mutex<VecDeque<UIUpdate>>>,
  step_index: Arc<Mutex<usize>>,
  ratchet_generation: Arc<AtomicUsize>,
  /// Remaining fast-division steps within the current regex match span.
  /// Non-zero → advance at 2× rate (halved divider) until it reaches 0.
  match_span_remaining: Arc<AtomicUsize>,

  #[cfg(feature = "symspell")]
  pub tmp_buf: Arc<Mutex<AllocRingBuffer<char>>>,
  #[cfg(feature = "symspell")]
  pub rpl_state: Arc<Mutex<RplPendingState>>,
  // #[cfg(debug_assertions)]
  // timing_stats: Arc<TimingStats>,
}

impl PlayheadArea {
  pub fn new(midi_tx: Sender<midi::Message>) -> Self {
    let position_calc = Arc::new(PositionCalculator::new());

    // Create consolidated state structures
    let grid = GridState::new();
    let music = MusicState::new();
    let modes = ModeFlags::new();
    let prev_active_pos = Arc::new(Mutex::new(Vec2::zero()));
    let ratchet_generation = Arc::new(AtomicUsize::new(0));
    let pos = Arc::new(Mutex::new(Vec2::zero()));
    let tilt_mode = Arc::new(Mutex::new(TiltMode::default()));
    let aimed_area = Arc::new(Mutex::new(None));

    // Create MIDI handler with shared state
    let midi_handler = Arc::new(MidiTriggerHandler::new(
      midi_tx.clone(),
      Arc::clone(&grid.width),
      Arc::clone(&grid.height),
      Arc::clone(&grid.v_splits),
      Arc::clone(&grid.h_splits),
      Arc::clone(&music.tempo),
      Arc::clone(&music.scale_mode_top),
      Arc::clone(&music.scale_mode_left),
      Arc::clone(&music.scale_root_top),
      Arc::clone(&prev_active_pos),
      Arc::clone(&modes.hold_next_note),
      Arc::clone(&modes.is_ratcheting),
      Arc::clone(&ratchet_generation),
      Arc::clone(&position_calc.text_matcher),
      Arc::clone(&modes.sweep_mode),
      Arc::clone(&modes.dyn_length_mode),
      Arc::clone(&modes.arpeggiator_mode),
      Arc::clone(&tilt_mode),
      Arc::clone(&pos),
      Arc::clone(&music.ratio),
    ));

    PlayheadArea {
      pos,
      area: Arc::new(Mutex::new(Rect::from_point(Vec2::zero()))),
      drag_start_x: AtomicUsize::new(0),
      drag_start_y: AtomicUsize::new(0),
      actived_pos: Arc::new(Mutex::new(Vec2::zero())),
      frozen_active_pos: Arc::new(Mutex::new(Vec2::zero())),
      prev_active_pos,
      aimed_area,
      grid,
      music,
      modes,
      regex_indexes: Arc::clone(&position_calc.regex_indexes),
      text_matcher: Arc::clone(&position_calc.text_matcher),
      movement: Arc::clone(&position_calc.movement),
      tilt_mode,
      // midi_tx,
      queue_manager: Arc::new(QueueManager::new()),
      position_calc,
      midi_handler,
      accumulation_counter: Arc::new(Mutex::new(0)),
      ui_update_queue: Arc::new(Mutex::new(VecDeque::new())),
      step_index: Arc::new(Mutex::new(0)),
      ratchet_generation,
      match_span_remaining: Arc::new(AtomicUsize::new(0)),
      #[cfg(feature = "symspell")]
      tmp_buf: Arc::new(Mutex::new(AllocRingBuffer::new(consts::TMP_BUF_SIZE))),
      #[cfg(feature = "symspell")]
      rpl_state: Arc::new(Mutex::new(RplPendingState::Empty)),
      // #[cfg(debug_assertions)]
      // timing_stats: Arc::new(TimingStats::new()),
    }
  }

  pub fn spawn_ui_processor(
    ui_queue: Arc<Mutex<VecDeque<UIUpdate>>>,
    cb_sink: cursive::CbSink,
    #[cfg(feature = "symspell")] tmp_buf: Arc<Mutex<AllocRingBuffer<char>>>,
    #[cfg(feature = "symspell")] symspell: Arc<Mutex<SymSpell>>,
    #[cfg(feature = "symspell")] rpl_state: Arc<Mutex<RplPendingState>>,
    #[cfg(feature = "symspell")] regex_tx: Sender<regex::Message>,
  ) {
    thread::Builder::new()
      .name("ui-batch-processor".to_string())
      .spawn(move || loop {
        thread::sleep(Duration::from_millis(16)); // ~60 FPS

        let mut queue = ui_queue.lock().unwrap();
        if queue.is_empty() {
          drop(queue);
          continue;
        }

        let updates: Vec<UIUpdate> = queue.drain(..).collect();
        drop(queue);

        #[cfg(feature = "symspell")]
        let tmp_buf_cb = Arc::clone(&tmp_buf);
        #[cfg(feature = "symspell")]
        let symspell_cb = Arc::clone(&symspell);
        #[cfg(feature = "symspell")]
        let rpl_state_cb = Arc::clone(&rpl_state);
        #[cfg(feature = "symspell")]
        let regex_tx_cb = regex_tx.clone();
        cb_sink
          .send(Box::new(move |siv| {
            #[cfg(feature = "symspell")]
            let tmp_buf = tmp_buf_cb;
            #[cfg(feature = "symspell")]
            let symspell = symspell_cb;
            #[cfg(feature = "symspell")]
            let rpl_state = rpl_state_cb;
            #[cfg(feature = "symspell")]
            let regex_tx = regex_tx_cb;

            for update in updates {
              match update {
                UIUpdate::ActivePos(active_pos) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.actived_pos = active_pos;
                    },
                  );
                }
                UIUpdate::AccumulationCounter(count, total) => {
                  siv.call_on_name(
                    consts::input_status_unit_view,
                    move |view: &mut TextView| {
                      view.set_content(format!("@ {}/{}", count, total));
                    },
                  );
                }
                UIUpdate::PlayheadPosAndArea(pos, area) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.playhead_pos = pos;
                      editor.playhead_ui.playhead_area = area;
                    },
                  );
                  siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
                    view.set_content(utils::build_pos_status_str(pos));
                  });
                  let area_size = area.size();
                  siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
                    view.set_content(utils::build_len_status_str((area_size.x, area_size.y)));
                  });
                }
                UIUpdate::ChnStatus(chn_str) => {
                  siv.call_on_name(consts::chn_status_unit_view, |view: &mut TextView| {
                    view.set_content(chn_str);
                  });
                }
                UIUpdate::GridSplits(v, h) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.grid_v_splits = v;
                      editor.playhead_ui.grid_h_splits = h;
                    },
                  );
                }
                UIUpdate::AimedArea(aimed_area) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.aimed_area = aimed_area;
                      if aimed_area.is_none() {
                        editor.is_aiming = false;
                      }
                    },
                  );
                }
                #[cfg(feature = "symspell")]
                UIUpdate::TmpAppendSpace => {
                  let display = {
                    let mut buf = tmp_buf.lock().unwrap();
                    let last = buf.back().copied();
                    if last != Some(' ') {
                      buf.push(' ');
                    }
                    let s: String = buf.iter().collect();
                    if s.is_empty() {
                      "-".to_string()
                    } else {
                      s
                    }
                  };
                  siv.call_on_name(consts::tmp_status_unit_view, move |view: &mut TextView| {
                    view.set_content(display);
                  });
                }
                #[cfg(feature = "symspell")]
                UIUpdate::TmpAppend(idx) => {
                  let ch = siv
                    .call_on_name(
                      consts::canvas_editor_section_view,
                      move |canvas: &mut Canvas<GridEditor>| {
                        canvas
                          .state_mut()
                          .grid
                          .data
                          .get(idx)
                          .copied()
                          .unwrap_or('\0')
                      },
                    )
                    .unwrap_or('\0');
                  if ch.is_alphabetic() {
                    let new_tmp = {
                      let mut buf = tmp_buf.lock().unwrap();
                      buf.push(ch);
                      buf.iter().collect::<String>()
                    };
                    siv.call_on_name(consts::tmp_status_unit_view, |view: &mut TextView| {
                      view.set_content(new_tmp.clone());
                    });
                    // Run symspell on the current TMP content
                    let sym_result = {
                      let mut ss = symspell.lock().unwrap();
                      let suggestions = ss.lookup_compound(new_tmp.trim(), 2, &None, false);
                      suggestions
                        .into_iter()
                        .next()
                        .map(|s| s.term)
                        .unwrap_or_default()
                    };
                    if !sym_result.is_empty() {
                      siv.call_on_name(consts::sym_status_unit_view, move |view: &mut TextView| {
                        view.set_content(sym_result);
                      });
                    }
                  }
                }
                #[cfg(feature = "symspell")]
                UIUpdate::RplCycle(old_area) => {
                  let sym = siv
                    .call_on_name(consts::sym_status_unit_view, |v: &mut TextView| {
                      v.get_content().source().to_string()
                    })
                    .unwrap_or_default();
                  let mut state = rpl_state.lock().unwrap();
                  let apply_opt: Option<(String, Rect)> =
                    match std::mem::replace(&mut *state, RplPendingState::Empty) {
                      RplPendingState::Armed(armed_sym, armed_area) => {
                        *state = if !sym.is_empty() && sym != "-" {
                          RplPendingState::Waiting(sym, old_area)
                        } else {
                          RplPendingState::Empty
                        };
                        Some((armed_sym, armed_area))
                      }
                      RplPendingState::Waiting(w_sym, w_area) => {
                        *state = RplPendingState::Armed(w_sym, w_area);
                        None
                      }
                      RplPendingState::Empty => {
                        *state = if !sym.is_empty() && sym != "-" {
                          RplPendingState::Waiting(sym, old_area)
                        } else {
                          RplPendingState::Empty
                        };
                        None
                      }
                    };
                  let rpl_display = match &*state {
                    RplPendingState::Armed(s, _) => format!("[armed] {}", s),
                    RplPendingState::Waiting(s, _) => s.clone(),
                    RplPendingState::Empty => "-".to_string(),
                  };
                  drop(state);
                  siv.call_on_name(consts::rpl_status_unit_view, move |v: &mut TextView| {
                    v.set_content(rpl_display);
                  });
                  if let Some((sym_text, area)) = apply_opt {
                    let result = siv.call_on_name(
                      consts::canvas_editor_section_view,
                      move |canvas: &mut Canvas<GridEditor>| {
                        let editor = canvas.state_mut();
                        let gw = editor.grid.width;
                        let old_text = editor.text_contents();
                        let new_text = splice_text_at_area(&old_text, &area, &sym_text);
                        editor.update_text_contents(&new_text);
                        editor.update_grid_src();
                        (new_text, gw)
                      },
                    );
                    if let Some((new_text, grid_width)) = result {
                      let pattern = siv
                        .call_on_name(consts::regex_input_unit_view, |v: &mut EditView| {
                          v.get_content().as_ref().clone()
                        })
                        .unwrap_or_default();
                      if !pattern.is_empty() {
                        let _ = regex_tx.send(regex::Message::Solve(regex::EventData {
                          text: new_text,
                          pattern,
                          flags: "i".to_string(),
                          grid_width,
                        }));
                      }
                    }
                  }
                }
              }
            }
          }))
          .unwrap();
      })
      .expect("Failed to spawn UI batch processor thread");
  }

  fn compute_chn_str(&self, pos: Vec2) -> String {
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

  fn build_mode_status_string(&self) -> String {
    let arpeggiator = self.modes.arpeggiator_mode.load(Ordering::Relaxed);
    let accumulation = self.modes.accumulation_mode.load(Ordering::Relaxed);
    let event_op = self.modes.event_operator_mode.load(Ordering::Relaxed);
    let drain_queue = self.queue_manager.is_drain_queue_mode();
    let sweep = self.modes.sweep_mode.load(Ordering::Relaxed);
    let dyn_length = self.modes.dyn_length_mode.load(Ordering::Relaxed);

    let mut active_modes = Vec::new();
    if arpeggiator {
      active_modes.push(AppMode::Arpeggiator);
    }
    if drain_queue {
      active_modes.push(AppMode::DrainQueue);
    }
    if accumulation {
      active_modes.push(AppMode::Accumulation);
    }
    if dyn_length {
      active_modes.push(AppMode::DynLength);
    }
    if event_op {
      active_modes.push(AppMode::EventOperator);
    }
    if sweep {
      active_modes.push(AppMode::Sweep);
    }

    AppMode::print_activated_modes_from_vec(&active_modes)
  }

  fn build_movement_status_string(&self) -> String {
    let movement = self.movement.lock().unwrap();
    movement.print_movements()
  }

  /// Returns true when the playhead is currently advancing in the forward direction.
  /// For Pendulum, this depends on which half of the cycle we are in.
  fn is_going_forward(&self) -> bool {
    let movement = *self.movement.lock().unwrap();
    match movement {
      Movement::Forward | Movement::Random => true,
      Movement::Reverse => false,
      Movement::Pendulum => {
        let step_idx = *self.step_index.lock().unwrap();
        let area = self.area.lock().unwrap();
        let total = area.width() * area.height();
        drop(area);
        if total <= 1 {
          return true;
        }
        let cycle_len = total * 2 - 2;
        (step_idx % cycle_len) < total
      }
    }
  }

  pub fn switch_movement(&self, new_movement: Movement, cb_sink: cursive::CbSink) {
    let mut movement = self.movement.lock().unwrap();
    *movement = new_movement;
    drop(movement);

    let movement_status = self.build_movement_status_string();
    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
        siv.call_on_name(consts::movement_unit_view, |view: &mut TextView| {
          view.set_content(movement_status);
        });
      }))
      .unwrap();
  }

  fn handle_movement(
    &self,
    direction: Direction,
    steps: usize,
    canvas_size: Vec2,
    cb_sink: cursive::CbSink,
  ) {
    self.ratchet_generation.fetch_add(1, Ordering::SeqCst);
    self.modes.is_ratcheting.store(false, Ordering::Relaxed);
    self.match_span_remaining.store(0, Ordering::Relaxed);
    self.set_leap(direction, steps, canvas_size);
    self.reset_accumulation_counter();

    let pos_mutex = self.pos.lock().unwrap();
    let pos = *pos_mutex;
    drop(pos_mutex);

    let area_mutex = self.area.lock().unwrap();
    let area = *area_mutex;
    drop(area_mutex);

    let chn_str = self.compute_chn_str(pos);
    #[cfg(feature = "symspell")]
    let tmp_buf_move = Arc::clone(&self.tmp_buf);

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_pos_status_str(pos));
        });

        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(consts::chn_status_unit_view, |view: &mut TextView| {
          view.set_content(chn_str);
        });

        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.playhead_pos = pos;
            editor.playhead_ui.playhead_area = area;
          },
        );
        #[cfg(feature = "symspell")]
        {
          let display = {
            let mut buf = tmp_buf_move.lock().unwrap();
            let last = buf.back().copied();
            if last != Some(' ') {
              buf.push(' ');
            }
            let s: String = buf.iter().collect();
            if s.is_empty() {
              "-".to_string()
            } else {
              s
            }
          };
          siv.call_on_name(consts::tmp_status_unit_view, move |view: &mut TextView| {
            view.set_content(display);
          });
        }
      }))
      .unwrap();
  }

  fn set_move(&self, direction: Direction, canvas_size: Vec2) {
    self.set_leap(direction, 1, canvas_size);
  }

  fn set_leap(&self, direction: Direction, steps: usize, canvas_size: Vec2) {
    let mut pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();
    let (dx, dy) = direction.get_direction();

    let leap_x = dx * steps as i32;
    let leap_y = dy * steps as i32;

    let next_pos = Vec2::new(
      pos.x.saturating_add_signed(leap_x as isize),
      pos.y.saturating_add_signed(leap_y as isize),
    );
    let next_pos_bottom_right: Vec2 = (
      next_pos.x + area.width() - 1,
      next_pos.y + area.height() - 1,
    )
      .into();

    if !next_pos_bottom_right.fits_in_rect(Vec2::ZERO, canvas_size) {
      return;
    }

    *pos = next_pos;

    let w = area.width();
    let h = area.height();

    *area = Rect::from_size(next_pos, (w, h));
  }

  pub fn set_current_pos(&self, pos: XY<usize>, offset: XY<usize>) {
    let mut mutex_pos = self.pos.lock().unwrap();
    let pos_x = pos.x.abs_diff(1);
    let pos_y = pos.y.abs_diff(offset.y);
    *mutex_pos = (pos_x, pos_y).into();
  }

  pub fn move_to(&self, current_pos: XY<usize>) {
    let pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();

    let new_w = current_pos.x.abs_diff(pos.x).clamp(1, usize::MAX);
    let new_h = current_pos.y.abs_diff(pos.y).clamp(1, usize::MAX);
    let new_x = match current_pos.x.saturating_sub(pos.x) == 0 {
      true => current_pos.x,
      false => pos.x,
    };

    let new_y = match current_pos.y.saturating_sub(pos.y) == 0 {
      true => current_pos.y,
      false => pos.y,
    };

    *area = Rect::from_size((new_x, new_y), (new_w, new_h));
  }

  pub fn set_grid_area(&self, current_pos: XY<usize>) {
    self.move_to(current_pos);

    let area = self.area.lock().unwrap();
    let top_left = area.top_left();

    self.drag_start_x.store(top_left.x, Ordering::Relaxed);
    self.drag_start_y.store(top_left.y, Ordering::Relaxed);
  }

  pub fn set_actived_pos(&self, pos: usize) {
    let area = self.area.lock().unwrap();
    let playhead_w = area.width();
    let playhead_h = area.height();
    let playhead_x = area.left();
    let playhead_y = area.top();
    drop(area);

    // Sync state to position calculator
    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);
    self
      .position_calc
      .grid_height
      .store(self.grid.height.load(Ordering::Relaxed), Ordering::Relaxed);
    self.position_calc.arpeggiator_mode.store(
      self.modes.arpeggiator_mode.load(Ordering::Relaxed),
      Ordering::Relaxed,
    );

    let new_pos = self
      .position_calc
      .calculate_actived_pos(pos, playhead_x, playhead_y, playhead_w, playhead_h);

    let mut actived_pos = self.actived_pos.lock().unwrap();
    *actived_pos = new_pos;
  }

  /// get absolute position the same way it being displayed in console "POS: (x,y)" and flattened index
  fn calculate_absolute_position(&self, active_pos: Vec2) -> (usize, usize, usize) {
    let pos = self.pos.lock().unwrap();
    let playhead_pos = *pos;
    drop(pos);

    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);
    self
      .position_calc
      .calculate_absolute_position(playhead_pos, active_pos)
  }

  // TODO: revisit keyboard-left
  /// Find distance to the next closest trigger position within the playhead area
  fn find_distance_to_next_trigger(&self, curr_pos: usize) -> usize {
    let area = self.area.lock().unwrap();
    let playhead_x = area.left();
    let playhead_y = area.top();
    let playhead_width = area.width();
    let playhead_height = area.height();
    drop(area);

    // Sync state to position calculator
    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);

    self.position_calc.find_distance_to_next_trigger(
      curr_pos,
      playhead_x,
      playhead_y,
      playhead_width,
      playhead_height,
    )
  }

  fn check_contains(&self, area: &Rect, matcher: &HashMap<usize, regex::Match>) -> bool {
    for dy in 0..area.height() {
      for dx in 0..area.width() {
        let x = area.top_left.x + dx;
        let y = area.top_left.y + dy;
        let pos_index = y * self.grid.width.load(Ordering::Relaxed) + x;
        if matcher.contains_key(&pos_index) {
          return true;
        }
      }
    }
    false
  }

  fn handle_silent_step(&self, matcher: &HashMap<usize, regex::Match>, cb_sink: &cursive::CbSink) {
    if !self.modes.accumulation_mode.load(Ordering::Relaxed) {
      return;
    }
    let area = self.area.lock().unwrap();
    let has_some_pos = self.check_contains(&area, matcher);
    let playhead_area_size = area.width() * area.height();
    drop(area);

    if has_some_pos {
      return;
    };

    // When clock out is enabled, use fixed length to keep external devices in sync
    let clock_enabled = consts::CLOCK_ENABLED.load(Ordering::Relaxed);
    #[allow(clippy::if_same_then_else)]
    let counter_limit = if clock_enabled {
      playhead_area_size
      // 32
    } else {
      playhead_area_size
    };

    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter += 1;
    let current_count = *counter;
    self.update_accumulation_ui(current_count, counter_limit, cb_sink);
    if *counter >= counter_limit {
      *counter = 0;
      drop(counter);
      self.update_accumulation_ui(0, counter_limit, cb_sink);
      self.perform_accumulation_jump();
    } else {
      drop(counter);
      self.update_accumulation_ui(current_count, counter_limit, cb_sink);
    }
  }

  fn handle_accumulation_mode(&self, abs_x: usize, cb_sink: &cursive::CbSink) -> Option<Vec2> {
    self.queue_manager.check_and_execute_operators(
      abs_x,
      self.modes.event_operator_mode.load(Ordering::Relaxed),
    );

    // Handle Push operator manually since it needs current playhead position
    if abs_x.is_multiple_of(consts::QUEUE_OP_SPACING)
      && !abs_x.is_multiple_of(consts::EVENT_OP_SPACING)
      && !self.queue_manager.is_drain_queue_mode()
    {
      let position_index = abs_x / consts::QUEUE_OP_SPACING;
      let operator_index = position_index % QUEUE_OPERATORS.len();
      if QUEUE_OPERATORS[operator_index] == QueueOperator::Push {
        let playhead_pos = self.pos.lock().unwrap();
        let push_pos = (playhead_pos.x, playhead_pos.y);
        drop(playhead_pos);
        self.queue_manager.handle_push(push_pos);
      }
    }

    // Handle Pop operator - check for pending jump
    if abs_x.is_multiple_of(consts::QUEUE_OP_SPACING) {
      let position_index = abs_x / consts::QUEUE_OP_SPACING;
      let operator_index = position_index % QUEUE_OPERATORS.len();
      if QUEUE_OPERATORS[operator_index] == QueueOperator::Pop {
        let front = self.queue_manager.get_front_item();
        match front {
          Some(QueueItem::Event(_)) => {
            self.execute_front_op_in_queue();
          }
          Some(QueueItem::Position(x, y)) => {
            self
              .queue_manager
              .set_pending_jump(PendingJumpPosition::Waiting(x, y));
          }
          None => {}
        }
      }
    }

    let area = self.area.lock().unwrap();
    let playhead_area_size = area.width() * area.height();
    drop(area);

    // When clock out is enabled, use fixed length to keep external devices in sync
    let clock_enabled = consts::CLOCK_ENABLED.load(Ordering::Relaxed);
    // just fine-tuning which one to use
    #[allow(clippy::if_same_then_else)]
    let counter_limit = if clock_enabled {
      playhead_area_size
      // 32
    } else {
      playhead_area_size
    };

    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter += 1;
    let current_count = *counter;

    if *counter >= counter_limit {
      *counter = 0;
      drop(counter);

      self.update_accumulation_ui(0, counter_limit, cb_sink);
      Some(self.perform_accumulation_jump())
    } else {
      drop(counter);
      self.update_accumulation_ui(current_count, counter_limit, cb_sink);
      None
    }
  }

  fn update_accumulation_ui(&self, count: usize, total: usize, _cb_sink: &cursive::CbSink) {
    // Queue UI update instead of immediate send (batched processing)
    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::AccumulationCounter(count, total));
  }

  fn perform_accumulation_jump(&self) -> Vec2 {
    let mut rng = rand::thread_rng();

    let area = self.area.lock().unwrap();
    let playhead_width = area.width();
    let playhead_height = area.height();
    drop(area);

    let grid_width = self.grid.width.load(Ordering::Relaxed);
    let grid_height = self.grid.height.load(Ordering::Relaxed);

    let current_playhead_pos = {
      let pos = self.pos.lock().unwrap();
      (pos.x, pos.y)
    };

    // Two-stage pending jump:
    //  Waiting → this jump is random, promote to Armed for the next one.
    //  Armed   → use the stored position, then clear.
    //  Empty   → normal random / queue-front logic.
    let (new_x, new_y) = {
      match self.queue_manager.peek_pending_jump() {
        PendingJumpPosition::Armed(x, y) => {
          self
            .queue_manager
            .set_pending_jump(PendingJumpPosition::Empty);
          (x, y)
        }
        PendingJumpPosition::Waiting(_x, _y) => {
          self.queue_manager.promote_waiting_to_armed();
          // This jump falls through to the normal (random) logic.
          let params = GridParams {
            playhead_width,
            playhead_height,
            grid_width,
            grid_height,
            rng: &mut rng,
          };
          self.get_jump_position(current_playhead_pos, params)
        }
        PendingJumpPosition::Empty => {
          let params = GridParams {
            playhead_width,
            playhead_height,
            grid_width,
            grid_height,
            rng: &mut rng,
          };
          self.get_jump_position(current_playhead_pos, params)
        }
      }
    };

    self.ratchet_generation.fetch_add(1, Ordering::SeqCst);
    self.modes.is_ratcheting.store(false, Ordering::Relaxed);

    // Update playhead position and area
    let mut pos = self.pos.lock().unwrap();
    pos.x = new_x;
    pos.y = new_y;
    let new_pos = *pos;
    drop(pos);

    let mut area = self.area.lock().unwrap();
    #[cfg(feature = "symspell")]
    let prev_area = *area;
    *area = Rect::from_size((new_x, new_y), (playhead_width, playhead_height));
    let new_area = *area;
    drop(area);

    // Reset active position to start
    let mut actived = self.actived_pos.lock().unwrap();
    *actived = Vec2::zero();
    drop(actived);

    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::PlayheadPosAndArea(new_pos, new_area));
    queue.push_back(UIUpdate::ChnStatus(self.compute_chn_str(new_pos)));

    #[cfg(feature = "symspell")]
    queue.push_back(UIUpdate::RplCycle(prev_area));
    #[cfg(feature = "symspell")]
    queue.push_back(UIUpdate::TmpAppendSpace);

    Vec2::zero()
  }

  /// determine jump position from queue or generate random position if queue is empty
  fn get_jump_position<R: rand::Rng>(
    &self,
    current_playhead_pos: (usize, usize),
    params: GridParams<R>,
  ) -> (usize, usize) {
    if let Some(first_item) = self.queue_manager.get_front_item() {
      match first_item {
        QueueItem::Position(x, y) => {
          let first_pos = (x, y);
          if first_pos == current_playhead_pos {
            PositionCalculator::generate_random_position(
              params.playhead_width,
              params.playhead_height,
              params.grid_width,
              params.grid_height,
              params.rng,
            )
          } else if let Some(QueueItem::Position(x, y)) = self.queue_manager.remove_front_item() {
            self.execute_front_op_in_queue();
            (x, y)
          } else {
            (0, 0)
          }
        }
        QueueItem::Event(_) => PositionCalculator::generate_random_position(
          params.playhead_width,
          params.playhead_height,
          params.grid_width,
          params.grid_height,
          params.rng,
        ),
      }
    } else {
      PositionCalculator::generate_random_position(
        params.playhead_width,
        params.playhead_height,
        params.grid_width,
        params.grid_height,
        params.rng,
      )
    }
  }

  fn update_active_pos_ui(&self, active_pos: Vec2, _cb_sink: &cursive::CbSink) {
    // Queue UI update instead of immediate send (batched processing)
    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::ActivePos(active_pos));
  }

  pub fn toggle_arpeggiator_mode(&self, cb_sink: cursive::CbSink) {
    let is_arp = !self.modes.arpeggiator_mode.load(Ordering::Relaxed);
    self.modes.arpeggiator_mode.store(is_arp, Ordering::Relaxed);
    self
      .position_calc
      .arpeggiator_mode
      .store(is_arp, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.arpeggiator_mode = is_arp;
            editor.playhead_ui.arpeggiator_mode = is_arp;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_event_operator_mode(&self, cb_sink: cursive::CbSink) {
    let is_event_op = !self.modes.event_operator_mode.load(Ordering::Relaxed);
    self
      .modes
      .event_operator_mode
      .store(is_event_op, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.event_operator_mode = is_event_op;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_drain_queue_mode(&self, cb_sink: cursive::CbSink) {
    let is_drain = !self.queue_manager.is_drain_queue_mode();
    self.queue_manager.set_drain_queue_mode(is_drain);

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.drain_queue_mode = is_drain;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_sweep_mode(&self, cb_sink: cursive::CbSink) {
    let is_sweep = !self.modes.sweep_mode.load(Ordering::Relaxed);
    self.modes.sweep_mode.store(is_sweep, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.sweep_mode = is_sweep;
            editor.playhead_ui.sweep_mode = is_sweep;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn cycle_tilt_mode(&self, cb_sink: cursive::CbSink) {
    let mut tilt = self.tilt_mode.lock().unwrap();
    *tilt = tilt.cycle_next();
    let new_tilt = *tilt;
    drop(tilt);

    let tilt_status = new_tilt.print_tilts();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.tilt_mode = new_tilt;
          },
        );
        siv.call_on_name(consts::tilt_unit_view, |view: &mut TextView| {
          view.set_content(tilt_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_dyn_length_mode(&self, cb_sink: cursive::CbSink) {
    let is_dyn_length = !self.modes.dyn_length_mode.load(Ordering::Relaxed);
    self
      .modes
      .dyn_length_mode
      .store(is_dyn_length, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_freeze_mode(&self, cb_sink: cursive::CbSink) {
    let is_freeze = !self.modes.freeze_mode.load(Ordering::Relaxed);
    self.modes.freeze_mode.store(is_freeze, Ordering::Relaxed);

    if is_freeze {
      // Snap-capture the current active position to freeze at
      let current_pos = *self.actived_pos.lock().unwrap();
      *self.frozen_active_pos.lock().unwrap() = current_pos;
    }

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.freeze_mode = is_freeze;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn cycle_scale_root(&self, cb_sink: cursive::CbSink, dir: types::Adjustment) {
    let mut root = self.music.scale_root_top.lock().unwrap();
    *root = root.cycle(dir);
    let new_root = *root;
    drop(root);

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.scale_root_top = new_root;
          },
        );
      }))
      .unwrap();
  }

  pub fn cycle_scale_mode(&self, cb_sink: cursive::CbSink, dir: types::Adjustment) {
    let mut mode = self.music.scale_mode_top.lock().unwrap();
    *mode = mode.cycle(dir);
    let new_mode = *mode;
    drop(mode);

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.scale_mode_top = new_mode;
          },
        );
      }))
      .unwrap();
  }

  pub fn scale(&self, (w, h): (i32, i32)) {
    let pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();

    // Calculate new dimensions with bounds checking
    let new_width = ((area.width() as i32) + w).max(1);
    let new_height = ((area.height() as i32) - h).max(1);

    *area = Rect::from_size(*pos, (new_width, new_height));
  }

  pub fn set_text_matcher(&self, text_matcher: Option<HashMap<usize, Match>>) {
    let mut tm = self.text_matcher.lock().unwrap();
    *tm = text_matcher
  }

  fn execute_front_op_in_queue(&self) {
    let op_front = self.queue_manager.get_front_item();
    match op_front {
      None => (),
      Some(QueueItem::Position(_, _)) => (),
      Some(QueueItem::Event(ev_op)) => {
        // Calculate absolute position for event operators
        let actived_pos = *self.actived_pos.lock().unwrap();
        let playhead_pos = *self.pos.lock().unwrap();
        let abs_x = playhead_pos.x + actived_pos.x;
        let abs_y = playhead_pos.y + actived_pos.y;

        match ev_op {
          EventOperator::H => {
            self.midi_handler.h_op();
          }
          EventOperator::C => {
            // Calculate distance for chord
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

  // #[cfg(debug_assertions)]
  // pub fn spawn_stats_printer(self: &Arc<Self>) {
  //   let stats = Arc::clone(&self.timing_stats);
  //   thread::spawn(move || loop {
  //     thread::sleep(Duration::from_secs(10));
  //     stats.print_stats();
  //     stats.reset();
  //   });
  // }

  pub fn reset_accumulation_counter(&self) {
    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter = 0;
  }

  fn handle_set_current_pos(
    &self,
    position: XY<usize>,
    offset: XY<usize>,
    cb_sink: cursive::CbSink,
  ) {
    self.ratchet_generation.fetch_add(1, Ordering::SeqCst);
    self.modes.is_ratcheting.store(false, Ordering::Relaxed);
    self.match_span_remaining.store(0, Ordering::Relaxed);
    self.set_current_pos(position, offset);
    self.reset_accumulation_counter();

    let mutex_pos = self.pos.lock().unwrap();
    let pos = *mutex_pos;
    drop(mutex_pos);
    #[cfg(feature = "symspell")]
    let tmp_buf_move = Arc::clone(&self.tmp_buf);
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.playhead_pos = pos;
          },
        );

        #[cfg(feature = "symspell")]
        {
          let display = {
            let mut buf = tmp_buf_move.lock().unwrap();
            let last = buf.back().copied();
            if last != Some(' ') {
              buf.push(' ');
            }
            let s: String = buf.iter().collect();
            if s.is_empty() {
              "-".to_string()
            } else {
              s
            }
          };
          siv.call_on_name(consts::tmp_status_unit_view, move |view: &mut TextView| {
            view.set_content(display);
          });
        }
      }))
      .unwrap();
  }

  fn handle_update_info_status_view(&self, cb_sink: cursive::CbSink) {
    let pos = self.pos.lock().unwrap();
    let area = self.area.lock().unwrap();
    let pos_x = pos.x;
    let pos_y = pos.y;
    let w = area.width();
    let h = area.height();
    drop(pos);
    drop(area);

    let chn_str = self.compute_chn_str(Vec2::new(pos_x, pos_y));

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_pos_status_str((pos_x, pos_y).into()))
        });

        siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str((w, h)));
        });

        siv.call_on_name(consts::chn_status_unit_view, |view: &mut TextView| {
          view.set_content(chn_str);
        });
      }))
      .unwrap();
  }

  fn handle_set_grid_area(&self, current_pos: XY<usize>, cb_sink: cursive::CbSink) {
    self.set_grid_area(current_pos);
    self.reset_accumulation_counter();

    let area = self.area.lock().unwrap();
    let w = area.width();
    let h = area.height();
    let playhead_area = *area;

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str((w, h)));
        });

        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();

            editor.playhead_ui.playhead_area = playhead_area;
          },
        );
      }))
      .unwrap();
  }

  fn handle_set_active_pos(&self, tick: usize, cb_sink: cursive::CbSink) {
    let ratio = self.music.ratio.lock().unwrap();
    // Clock fires 16 ticks/beat (64 per 4-beat bar). Divider table:
    //   ratio.1=1  → every 64 ticks (DIV 1)    ratio.1=16 → every  4 ticks (DIV 16)
    //   ratio.1=32 → every  2 ticks (DIV 32)   ratio.1=64 → every  1 tick  (DIV 64)
    let base_divider = (64 / ratio.1).max(1);
    drop(ratio);

    if self.modes.freeze_mode.load(Ordering::Relaxed) {
      if tick.is_multiple_of(base_divider) && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        let active_pos = *self.frozen_active_pos.lock().unwrap();
        let going_forward = self.is_going_forward();
        let (abs_x, abs_y, curr_running_playhead) = self.calculate_absolute_position(active_pos);
        let (note_position, scale_mode) = self
          .midi_handler
          .determine_note_position_and_scale(active_pos, abs_x, abs_y);

        let matcher_guard = self.text_matcher.lock().unwrap();
        let match_at_pos = if going_forward {
          matcher_guard
            .as_ref()
            .and_then(|m| m.get(&curr_running_playhead))
            .cloned()
        } else {
          matcher_guard.as_ref().and_then(|m| {
            m.values()
              .find(|mv| curr_running_playhead == mv.i + mv.l.max(1) - 1)
              .cloned()
          })
        };
        drop(matcher_guard);
        let has_match = match_at_pos.is_some();
        let match_len = match_at_pos.as_ref().map(|m| m.l).unwrap_or(1).max(1);

        if has_match {
          let playhead_pos = self.pos.lock().unwrap();
          let playhead_pos_x = playhead_pos.x;
          let playhead_pos_y = playhead_pos.y;
          drop(playhead_pos);

          self.midi_handler.trigger_midi_if_matched(
            curr_running_playhead,
            note_position,
            scale_mode,
            playhead_pos_x,
            playhead_pos_y,
            match_len,
          );
        }

        self.update_active_pos_ui(active_pos, &cb_sink);
      }
      return;
    }

    let going_forward = self.is_going_forward();
    // Inside a match span: speed up when going forward (halve divider),
    // slow down when going reverse (double divider).
    let in_match_span = self.match_span_remaining.load(Ordering::Relaxed) > 0;
    let divider = if in_match_span {
      if going_forward {
        (base_divider / 2).max(1)
      } else {
        base_divider * 2
      }
    } else {
      base_divider
    };

    let should_advance =
      tick.is_multiple_of(divider) && !self.modes.is_ratcheting.load(Ordering::Relaxed);
    if should_advance {
      if in_match_span {
        self.match_span_remaining.fetch_sub(1, Ordering::Relaxed);
      }

      let mut step_idx = self.step_index.lock().unwrap();
      *step_idx += 1;
      self.set_actived_pos(*step_idx);
      drop(step_idx);

      let active_pos_mutex = self.actived_pos.lock().unwrap();
      let mut active_pos = *active_pos_mutex;
      drop(active_pos_mutex);

      let (abs_x, abs_y, curr_running_playhead) = self.calculate_absolute_position(active_pos);

      let (note_position, scale_mode) = self
        .midi_handler
        .determine_note_position_and_scale(active_pos, abs_x, abs_y);

      let mut did_jump = false;

      // Capture the full Match for the current cell.
      // Forward: trigger at the first cell (m.i == curr).
      // Reverse/Pendulum-reverse: trigger at the last cell (m.i + m.l - 1 == curr).
      let matcher_guard = self.text_matcher.lock().unwrap();
      let match_at_pos = if going_forward {
        matcher_guard
          .as_ref()
          .and_then(|m| m.get(&curr_running_playhead))
          .cloned()
      } else {
        matcher_guard.as_ref().and_then(|m| {
          m.values()
            .find(|mv| curr_running_playhead == mv.i + mv.l.max(1) - 1)
            .cloned()
        })
      };
      drop(matcher_guard);

      let has_match = match_at_pos.is_some();
      let match_len = match_at_pos.as_ref().map(|m| m.l).unwrap_or(1).max(1);

      // Retrigger on every fast step inside a span (cells 2, 3, … of a match word)
      if in_match_span && !has_match && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        let playhead_pos = self.pos.lock().unwrap();
        let playhead_pos_x = playhead_pos.x;
        let playhead_pos_y = playhead_pos.y;
        drop(playhead_pos);
        self.midi_handler.trigger_midi_if_matched(
          curr_running_playhead,
          note_position,
          scale_mode,
          playhead_pos_x,
          playhead_pos_y,
          1,
        );
        #[cfg(feature = "symspell")]
        let mut queue = self.ui_update_queue.lock().unwrap();

        #[cfg(feature = "symspell")]
        queue.push_back(UIUpdate::TmpAppend(curr_running_playhead));
      }

      if has_match {
        if self.modes.accumulation_mode.load(Ordering::Relaxed) {
          if let Some(new_active_pos) = self.handle_accumulation_mode(abs_x, &cb_sink) {
            active_pos = new_active_pos;
            did_jump = true;
          }
        }
        if !did_jump && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
          let playhead_pos = self.pos.lock().unwrap();
          let playhead_pos_x = playhead_pos.x;
          let playhead_pos_y = playhead_pos.y;
          drop(playhead_pos);

          // Use match length as note duration; dyn_length_mode overrides with distance-based value
          let distance_to_next = if self.modes.dyn_length_mode.load(Ordering::Relaxed)
            && !self.modes.arpeggiator_mode.load(Ordering::Relaxed)
          {
            self.find_distance_to_next_trigger(curr_running_playhead)
          } else {
            match_len
          };

          self.midi_handler.trigger_midi_if_matched(
            curr_running_playhead,
            note_position,
            scale_mode,
            playhead_pos_x,
            playhead_pos_y,
            distance_to_next,
          );
          #[cfg(feature = "symspell")]
          let mut queue = self.ui_update_queue.lock().unwrap();

          #[cfg(feature = "symspell")]
          queue.push_back(UIUpdate::TmpAppend(curr_running_playhead));
        }
        if self.modes.hold_next_note.load(Ordering::Relaxed) {
          self.modes.hold_next_note.store(false, Ordering::Relaxed);
        }
        // Start fast-stepping for remaining cells in the match span
        if match_len > 1 {
          self
            .match_span_remaining
            .store(match_len - 1, Ordering::Relaxed);
        }
      }

      let matcher_guard = self.text_matcher.lock().unwrap();
      if let Some(ref m) = *matcher_guard {
        self.handle_silent_step(m, &cb_sink);
      }
      drop(matcher_guard);

      if !did_jump && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        let active_pos_y = self.pos.lock().unwrap().y;
        self
          .midi_handler
          .trigger_midi_if_matched_sweep(curr_running_playhead, abs_x, active_pos_y);

        #[cfg(feature = "symspell")]
        let sweep_indexes = self
          .midi_handler
          .sweep_matched_indexes(curr_running_playhead, abs_x);
        #[cfg(feature = "symspell")]
        if !sweep_indexes.is_empty() {
          let mut queue = self.ui_update_queue.lock().unwrap();
          for idx in sweep_indexes {
            queue.push_back(UIUpdate::TmpAppend(idx));
          }
        }
      }
      self.update_active_pos_ui(active_pos, &cb_sink);
    }
  }

  fn handle_scale(&self, size: (i32, i32), cb_sink: cursive::CbSink) {
    self.scale(size);
    self.reset_accumulation_counter();

    let area = self.area.lock().unwrap();
    let playhead_area = *area;
    let area_size = area.size();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str((area_size.x, area_size.y)));
        });

        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.playhead_area = playhead_area;
          },
        );
      }))
      .unwrap();
  }

  fn handle_set_matcher(&self, matcher: Option<HashMap<usize, Match>>, cb_sink: cursive::CbSink) {
    self.set_text_matcher(matcher);

    let text_matcher = self.text_matcher.lock().unwrap();
    let mm = text_matcher.clone();

    let regex_indexes_cloned = self.regex_indexes.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.text_matcher = mm;
            editor.playhead_ui.regex_indexes = regex_indexes_cloned;
          },
        );
      }))
      .unwrap();
  }

  fn handle_set_grid_size(&self, width: usize, height: usize, cb_sink: cursive::CbSink) {
    self.grid.width.store(width, Ordering::Relaxed);
    self.grid.height.store(height, Ordering::Relaxed);

    // Sync to position calculator
    self
      .position_calc
      .grid_width
      .store(width, Ordering::Relaxed);
    self
      .position_calc
      .grid_height
      .store(height, Ordering::Relaxed);

    let queue_manager_cloned = self.queue_manager.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.queue_manager = queue_manager_cloned;
          },
        );
      }))
      .unwrap();
  }

  fn handle_set_scale_mode_left(&self, scale_mode: scale::ScaleMode) {
    let mut mode = self.music.scale_mode_left.lock().unwrap();
    *mode = scale_mode;
  }

  fn handle_set_scale_root_top(&self, scale_root: scale::ScaleRoot) {
    let mut root = self.music.scale_root_top.lock().unwrap();
    *root = scale_root;
  }

  fn handle_set_scale_mode_top(&self, scale_mode: scale::ScaleMode) {
    let mut mode = self.music.scale_mode_top.lock().unwrap();
    *mode = scale_mode;
  }

  fn handle_toggle_accumulation_mode(&self, cb_sink: cursive::CbSink) {
    let is_enabled = !self.modes.accumulation_mode.load(Ordering::Relaxed);
    self
      .modes
      .accumulation_mode
      .store(is_enabled, Ordering::Relaxed);
    self.reset_accumulation_counter();

    if !is_enabled {
      self.queue_manager.clear_all();
    }

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.accumulation_mode = is_enabled;
            editor.playhead_ui.accumulation_mode = is_enabled;
          },
        );

        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  fn handle_set_tempo(&self, bpm: usize) {
    self.music.tempo.store(bpm, Ordering::Relaxed);
  }

  fn handle_set_ratio(&self, new_ratio: (usize, usize), cb_sink: cursive::CbSink) {
    let mut ratio = self.music.ratio.lock().unwrap();
    *ratio = new_ratio;
    drop(ratio);

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::ratio_status_unit_view, |view: &mut TextView| {
          view.set_content(utils::build_ratio_status_str(new_ratio));
        });
      }))
      .unwrap();
  }

  fn handle_clear_queue(&self, cb_sink: cursive::CbSink) {
    self.queue_manager.clear_all();
    self.reset_accumulation_counter();
    let _ = cb_sink.send(Box::new(move |siv| {
      siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
        view.set_content("-");
      });
    }));
  }

  fn handle_set_grid_splits(&self, v: usize, h: usize) {
    self.grid.v_splits.store(v, Ordering::Relaxed);
    self.grid.h_splits.store(h, Ordering::Relaxed);
    let pos = *self.pos.lock().unwrap();
    let chn_str = self.compute_chn_str(pos);
    let mut q = self.ui_update_queue.lock().unwrap();
    q.push_back(UIUpdate::GridSplits(v, h));
    q.push_back(UIUpdate::ChnStatus(chn_str));
  }

  fn handle_start_aim(&self) {
    // Use the authoritative playhead `pos` and current area size so the aimed
    // rectangle always starts at the actual playhead position.
    let current_pos = *self.pos.lock().unwrap();
    let area_size = self.area.lock().unwrap().size();
    let start_area = Rect::from_size(current_pos, area_size);

    *self.aimed_area.lock().unwrap() = Some(start_area);
    let mut q = self.ui_update_queue.lock().unwrap();
    q.push_back(UIUpdate::AimedArea(Some(start_area)));
  }

  fn handle_update_aim(&self, direction: Direction, canvas_size: Vec2, step: usize) {
    let mut aimed = self.aimed_area.lock().unwrap();
    let base_area = aimed.unwrap_or_else(|| *self.area.lock().unwrap());

    let (dx, dy) = direction.get_direction();
    let new_top_left = Vec2::new(
      (base_area.left() as i32 + dx * (step as i32)).max(0) as usize,
      (base_area.top() as i32 + dy * (step as i32)).max(0) as usize,
    );

    // ensure within grid bounds
    let size = base_area.size();
    let max_x = canvas_size.x.saturating_sub(size.x);
    let max_y = canvas_size.y.saturating_sub(size.y);

    let clamped_top_left = Vec2::new(new_top_left.x.min(max_x), new_top_left.y.min(max_y));

    let new_aimed_area = Rect::from_size(clamped_top_left, size);

    *aimed = Some(new_aimed_area);
    drop(aimed);

    let mut q = self.ui_update_queue.lock().unwrap();
    q.push_back(UIUpdate::AimedArea(Some(new_aimed_area)));
  }

  fn handle_commit_aim(&self, cb_sink: cursive::CbSink) {
    let aimed = self.aimed_area.lock().unwrap();
    let aimed_area_opt = *aimed;
    drop(aimed);

    if let Some(aimed_area) = aimed_area_opt {
      let target_pos = aimed_area.top_left();

      // Update internal state
      *self.pos.lock().unwrap() = target_pos;
      *self.aimed_area.lock().unwrap() = None;

      // Reset active position
      let mut actived = self.actived_pos.lock().unwrap();
      *actived = Vec2::zero();
      drop(actived);

      // Immediately update UI via cb_sink to avoid delayed queued update
      let chn_str = self.compute_chn_str(target_pos);
      let area_size = aimed_area.size();
      let pos_status = utils::build_pos_status_str(target_pos);
      let len_status = utils::build_len_status_str((area_size.x, area_size.y));

      cb_sink
        .send(Box::new(move |siv| {
          siv.call_on_name(
            consts::canvas_editor_section_view,
            move |canvas: &mut Canvas<GridEditor>| {
              let editor = canvas.state_mut();
              editor.playhead_ui.playhead_pos = target_pos;
              editor.playhead_ui.playhead_area = aimed_area;
              editor.playhead_ui.aimed_area = None;
              editor.is_aiming = false;
            },
          );

          siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
            view.set_content(pos_status.clone());
          });

          siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
            view.set_content(len_status.clone());
          });

          siv.call_on_name(consts::chn_status_unit_view, move |view: &mut TextView| {
            view.set_content(chn_str.clone());
          });
        }))
        .unwrap();
    } else {
      // Ensure UI clears aimed overlay immediately
      cb_sink
        .send(Box::new(move |siv| {
          siv.call_on_name(
            consts::canvas_editor_section_view,
            move |canvas: &mut Canvas<GridEditor>| {
              let editor = canvas.state_mut();
              editor.playhead_ui.aimed_area = None;
              editor.is_aiming = false;
            },
          );
        }))
        .unwrap();
    }
  }

  fn handle_cancel_aim(&self) {
    *self.aimed_area.lock().unwrap() = None;
    let mut q = self.ui_update_queue.lock().unwrap();
    q.push_back(UIUpdate::AimedArea(None));
  }

  pub fn run(self: Arc<Self>) -> Sender<Message> {
    let (tx, rx) = channel();

    // #[cfg(debug_assertions)]
    // self.spawn_stats_printer(); // Start stats printer

    thread::spawn(move || {
      for control_message in &rx {
        match control_message {
          Message::Move(direction, canvas_size, cb_sink) => {
            self.handle_movement(direction, 1, canvas_size, cb_sink);
          }
          Message::Leap(direction, steps, canvas_size, cb_sink) => {
            self.handle_movement(direction, steps, canvas_size, cb_sink);
          }
          Message::SetCurrentPos(position, offset, cb_sink) => {
            self.handle_set_current_pos(position, offset, cb_sink);
          }
          Message::UpdateInfoStatusView(cb_sink) => {
            self.handle_update_info_status_view(cb_sink);
          }
          Message::SetGridArea(current_pos, cb_sink) => {
            self.handle_set_grid_area(current_pos, cb_sink);
          }
          Message::SetActivePos(tick, cb_sink) => {
            self.handle_set_active_pos(tick, cb_sink);
          }
          Message::Scale(size, cb_sink) => {
            self.handle_scale(size, cb_sink);
          }
          Message::SetMatcher(matcher, cb_sink) => {
            self.handle_set_matcher(matcher, cb_sink);
          }
          Message::SetGridSize(width, height, cb_sink) => {
            self.handle_set_grid_size(width, height, cb_sink);
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
          Message::ToggleAccumulationMode(cb_sink) => {
            self.handle_toggle_accumulation_mode(cb_sink);
          }
          Message::SetTempo(bpm) => {
            self.handle_set_tempo(bpm);
          }
          Message::SetRatio(new_ratio, cb_sink) => {
            self.handle_set_ratio(new_ratio, cb_sink);
          }
          Message::ToggleForwardMode(cb_sink) => {
            self.switch_movement(Movement::Forward, cb_sink);
          }
          Message::ToggleReverseMode(cb_sink) => {
            self.switch_movement(Movement::Reverse, cb_sink);
          }
          Message::ToggleArpeggiatorMode(cb_sink) => {
            self.toggle_arpeggiator_mode(cb_sink);
          }
          Message::ToggleRandomMode(cb_sink) => {
            self.switch_movement(Movement::Random, cb_sink);
          }
          Message::TogglePendulumMode(cb_sink) => {
            self.switch_movement(Movement::Pendulum, cb_sink);
          }
          Message::ToggleEventOperatorMode(cb_sink) => {
            self.toggle_event_operator_mode(cb_sink);
          }
          Message::ToggleDrainQueueMode(cb_sink) => {
            self.toggle_drain_queue_mode(cb_sink);
          }
          Message::ToggleSweepMode(cb_sink) => {
            self.toggle_sweep_mode(cb_sink);
          }
          Message::ToggleDynLengthMode(cb_sink) => {
            self.toggle_dyn_length_mode(cb_sink);
          }
          Message::ToggleFreezeMode(cb_sink) => {
            self.toggle_freeze_mode(cb_sink);
          }
          Message::CycleScaleRootTop(cb_sink, dir) => {
            self.cycle_scale_root(cb_sink, dir);
          }
          Message::CycleScaleMode(cb_sink, dir) => {
            self.cycle_scale_mode(cb_sink, dir);
          }
          Message::ClearQueue(cb_sink) => {
            self.handle_clear_queue(cb_sink);
          }
          Message::SetGridSplits(v, h) => {
            self.handle_set_grid_splits(v, h);
          }
          Message::CycleTiltMode(cb_sink) => {
            self.cycle_tilt_mode(cb_sink);
          }
          Message::StartAim() => {
            self.handle_start_aim();
          }
          Message::UpdateAim(direction, canvas_size, step) => {
            self.handle_update_aim(direction, canvas_size, step);
          }
          Message::CommitAim(cb_sink) => {
            self.handle_commit_aim(cb_sink.clone());
          }
          Message::CancelAim() => {
            self.handle_cancel_aim();
          }
        }
      }
    });

    tx
  }
}

#[cfg(feature = "symspell")]
fn splice_text_at_area(text: &str, area: &Rect, sym: &str) -> String {
  let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
  let sym_chars: Vec<char> = sym.chars().collect();
  let top = area.top_left.y;
  let left = area.top_left.x;
  let h = area.height();
  let w = area.width();
  // Only write sym chars that exist; stop early if sym is exhausted.
  // Cells beyond sym length keep their original content, never overwrite
  // with spaces or write outside the playhead area.
  'outer: for row in 0..h {
    let line_idx = top + row;
    if line_idx >= lines.len() {
      lines.resize(line_idx + 1, String::new());
    }
    let line = &mut lines[line_idx];
    // Only pad the line if sym has chars in this row.
    let row_start = row * w;
    if row_start >= sym_chars.len() {
      break 'outer;
    }
    // Pad with '\0' (not ' ') so cells beyond the original line end remain
    // rendered as '.' rather than being overwritten with a visible space.
    while line.chars().count() < left + w {
      line.push('\0');
    }
    let mut chars: Vec<char> = line.chars().collect();
    for col in 0..w {
      let sym_idx = row_start + col;
      if sym_idx >= sym_chars.len() {
        break;
      }
      chars[left + col] = sym_chars[sym_idx];
    }
    *line = chars.into_iter().collect();
  }
  lines.join("\n")
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod view_tests;
