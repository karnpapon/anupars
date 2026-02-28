use arrayvec::ArrayVec;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;
// #[cfg(debug_assertions)]
// use std::time::Instant;

use cursive::views::Canvas;
use cursive::views::TextView;
use cursive::Vec2;
use cursive::XY;

use crate::app::AppMode;
use crate::app::Movement;
use crate::core::regex;
use crate::core::{consts, midi, playback_modes, rect::Rect, regex::Match, utils};
use crate::view::common::grid_editor::GridEditor;
use crate::view::common::playhead_controller::Direction;
// #[cfg(debug_assertions)]
// use crate::view::common::timing_diagnostic::TimingStats;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QueueOperator {
  Push,      // P
  Swap,      // S
  Pop,       // O
  Duplicate, // D
}

impl fmt::Display for QueueOperator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      QueueOperator::Push => write!(f, "P"),
      QueueOperator::Swap => write!(f, "S"),
      QueueOperator::Pop => write!(f, "O"),
      QueueOperator::Duplicate => write!(f, "D"),
    }
  }
}

pub const QUEUE_OPERATORS: [QueueOperator; 4] = [
  QueueOperator::Push,
  QueueOperator::Swap,
  QueueOperator::Pop,
  QueueOperator::Duplicate,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventOperator {
  R,
  C,
  X,
}

impl fmt::Display for EventOperator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EventOperator::R => write!(f, "r"),
      EventOperator::C => write!(f, "c"),
      EventOperator::X => write!(f, "x"),
    }
  }
}

impl EventOperator {
  pub fn get_event_name(&self) -> &'static str {
    match self {
      EventOperator::R => ">rrrrr",
      EventOperator::C => ">chord",
      EventOperator::X => ">hold",
    }
  }
}

pub const EVENT_OPERATORS: [EventOperator; 3] =
  [EventOperator::R, EventOperator::C, EventOperator::X];

// Queue item that can be either a position or an event
#[derive(Clone, Debug, PartialEq)]
pub enum QueueItem {
  Position(usize, usize),
  Event(EventOperator),
}

impl fmt::Display for QueueItem {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      QueueItem::Position(x, y) => write!(f, "{},{}", x, y),
      QueueItem::Event(op) => write!(f, "{}", op.get_event_name()),
    }
  }
}

// UI update types for batching
#[derive(Clone, Debug)]
pub enum UIUpdate {
  ActivePos(Vec2),
  AccumulationCounter(usize, usize), // (count, total)
  // OpQueueDisplay(String),
  // EvQueueDisplay(String),
  PlayheadPosAndArea(Vec2, Rect),
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
  pub text_matcher: Option<HashMap<usize, Match>>,
  pub regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  pub arpeggiator_mode: bool,
  pub sweep_mode: bool,
  pub operator_queue: Arc<Mutex<ArrayVec<QueueItem, { consts::OP_QUEUE_CAPACITY }>>>,
  pub event_queue:
    Arc<Mutex<ConstGenericRingBuffer<EventOperator, { consts::EVENT_QUEUE_CAPACITY }>>>,
}

impl PlayheadUI {
  pub fn new() -> Self {
    PlayheadUI {
      playhead_area: Rect::from_point(Vec2::zero()),
      playhead_pos: Vec2::zero(),
      actived_pos: Vec2::zero(),
      text_matcher: None,
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      arpeggiator_mode: false,
      sweep_mode: false,
      operator_queue: Arc::new(Mutex::new(ArrayVec::new())),
      event_queue: Arc::new(Mutex::new(ConstGenericRingBuffer::new())),
    }
  }
}

#[derive(Clone, Debug)]
pub enum Message {
  Move(Direction, XY<usize>, cursive::CbSink),
  SetCurrentPos(XY<usize>, XY<usize>, cursive::CbSink),
  UpdateInfoStatusView(cursive::CbSink),
  SetGridArea(XY<usize>, cursive::CbSink),
  SetActivePos(usize, cursive::CbSink),
  Scale((i32, i32), cursive::CbSink),
  SetMatcher(Option<HashMap<usize, Match>>, cursive::CbSink),
  SetGridSize(usize, usize, cursive::CbSink),
  SetScaleModeLeft(crate::core::scale::ScaleMode),
  SetScaleModeTop(crate::core::scale::ScaleMode),
  SetScaleRootTop(crate::core::scale::ScaleRoot),
  ToggleAccumulationMode(cursive::CbSink),
  ToggleForwardMode(cursive::CbSink),
  ToggleReverseMode(cursive::CbSink),
  TogglePendulumMode(cursive::CbSink),
  ToggleArpeggiatorMode(cursive::CbSink),
  ToggleRandomMode(cursive::CbSink),
  ToggleEventOperatorMode(cursive::CbSink),
  ToggleDrainQueueMode(cursive::CbSink),
  ToggleSweepMode(cursive::CbSink),
  CycleScaleRootTop(cursive::CbSink, crate::core::command::Adjustment),
  CycleScaleMode(cursive::CbSink, crate::core::command::Adjustment),
  SetTempo(usize),
  SetRatio((usize, usize), cursive::CbSink),
}

pub struct PlayheadArea {
  /// position of the top-left corner of the playhead area in the grid
  pos: Arc<Mutex<Vec2>>,
  /// area of the playhead, defined by the top-left corner (pos) and its width and height
  area: Arc<Mutex<Rect>>,
  drag_start_x: AtomicUsize,
  drag_start_y: AtomicUsize,
  /// current running position of the playhead within, relative to the playhead area (0,0)
  actived_pos: Arc<Mutex<Vec2>>,
  regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,
  midi_tx: Sender<midi::Message>,
  grid_width: AtomicUsize,
  grid_height: AtomicUsize,
  tempo: AtomicUsize,
  prev_active_pos: Arc<Mutex<Vec2>>,
  scale_mode_left: Arc<Mutex<crate::core::scale::ScaleMode>>,
  scale_mode_top: Arc<Mutex<crate::core::scale::ScaleMode>>,
  scale_root_top: Arc<Mutex<crate::core::scale::ScaleRoot>>,
  accumulation_counter: Arc<Mutex<usize>>,
  movement: Arc<Mutex<Movement>>,
  accumulation_mode: AtomicBool,
  arpeggiator_mode: AtomicBool,
  event_operator_mode: AtomicBool,
  drain_queue_mode: AtomicBool,
  sweep_mode: AtomicBool,
  ratio: Arc<Mutex<(usize, usize)>>,
  operator_queue: Arc<Mutex<ArrayVec<QueueItem, { consts::OP_QUEUE_CAPACITY }>>>,
  event_queue: Arc<Mutex<ConstGenericRingBuffer<EventOperator, { consts::EVENT_QUEUE_CAPACITY }>>>,
  pushed_positions: Arc<Mutex<HashMap<(usize, usize), bool>>>,
  pub ui_update_queue: Arc<Mutex<VecDeque<UIUpdate>>>,
  step_index: Arc<Mutex<usize>>,
  hold_next_note: AtomicBool,
  // #[cfg(debug_assertions)]
  // timing_stats: Arc<TimingStats>,
}

impl PlayheadArea {
  pub fn new(midi_tx: Sender<midi::Message>) -> Self {
    PlayheadArea {
      pos: Arc::new(Mutex::new(Vec2::zero())),
      area: Arc::new(Mutex::new(Rect::from_point(Vec2::zero()))),
      drag_start_x: AtomicUsize::new(0),
      drag_start_y: AtomicUsize::new(0),
      actived_pos: Arc::new(Mutex::new(Vec2::zero())),
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      text_matcher: Arc::new(Mutex::new(None)),
      midi_tx,
      grid_width: AtomicUsize::new(0),
      grid_height: AtomicUsize::new(0),
      tempo: AtomicUsize::new(consts::DEFAULT_TEMPO),
      prev_active_pos: Arc::new(Mutex::new(Vec2::zero())),
      scale_mode_left: Arc::new(Mutex::new(crate::core::scale::ScaleMode::default())),
      scale_mode_top: Arc::new(Mutex::new(crate::core::scale::ScaleMode::default())),
      scale_root_top: Arc::new(Mutex::new(crate::core::scale::ScaleRoot::default())),
      accumulation_counter: Arc::new(Mutex::new(0)),
      movement: Arc::new(Mutex::new(Movement::Forward)),
      // reverse_mode: AtomicBool::new(false),
      // random_mode: AtomicBool::new(false),
      accumulation_mode: AtomicBool::new(false),
      arpeggiator_mode: AtomicBool::new(false),
      event_operator_mode: AtomicBool::new(false),
      drain_queue_mode: AtomicBool::new(false),
      sweep_mode: AtomicBool::new(false),
      ratio: Arc::new(Mutex::new(consts::DEFAULT_RATIO)),
      operator_queue: Arc::new(Mutex::new(ArrayVec::new())),
      event_queue: Arc::new(Mutex::new(ConstGenericRingBuffer::new())),
      pushed_positions: Arc::new(Mutex::new(HashMap::new())),
      ui_update_queue: Arc::new(Mutex::new(VecDeque::new())),
      step_index: Arc::new(Mutex::new(0)),
      hold_next_note: AtomicBool::new(false),
      // #[cfg(debug_assertions)]
      // timing_stats: Arc::new(TimingStats::new()),
    }
  }

  pub fn spawn_ui_processor(ui_queue: Arc<Mutex<VecDeque<UIUpdate>>>, cb_sink: cursive::CbSink) {
    thread::Builder::new()
      .name("ui-batch-processor".to_string())
      .spawn(move || loop {
        thread::sleep(Duration::from_millis(16)); // ~60 FPS

        let mut queue = ui_queue.lock().unwrap();
        if queue.is_empty() {
          drop(queue);
          continue;
        }

        // Drain all pending updates
        let updates: Vec<UIUpdate> = queue.drain(..).collect();
        drop(queue);

        // Process batched updates
        cb_sink
          .send(Box::new(move |siv| {
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
                // UIUpdate::OpQueueDisplay(queue_str) => {
                //   siv.call_on_name(
                //     consts::op_queue_status_unit_view,
                //     move |view: &mut TextView| {
                //       view.set_content(queue_str);
                //     },
                //   );
                // }
                // UIUpdate::EvQueueDisplay(queue_str) => {
                //   siv.call_on_name(
                //     consts::ev_queue_status_unit_view,
                //     move |view: &mut TextView| {
                //       view.set_content(queue_str);
                //     },
                //   );
                // }
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
              }
            }
          }))
          .unwrap();
      })
      .expect("Failed to spawn UI batch processor thread");
  }

  fn build_mode_status_string(&self) -> String {
    let arpeggiator = self.arpeggiator_mode.load(Ordering::Relaxed);
    let accumulation = self.accumulation_mode.load(Ordering::Relaxed);
    let event_op = self.event_operator_mode.load(Ordering::Relaxed);
    let drain_queue = self.drain_queue_mode.load(Ordering::Relaxed);
    let sweep = self.sweep_mode.load(Ordering::Relaxed);

    let a = format!("{}", AppMode::Arpeggiator);
    let u = format!("{}", AppMode::Accumulation);
    let e = format!("{}", AppMode::EventOperator);
    let n = format!("{}", AppMode::DrainQueue);
    let s = format!("{}", AppMode::Sweep);

    format!(
      "{}{}{}{}{}",
      if arpeggiator { "A" } else { &a },
      if accumulation { "U" } else { &u },
      if event_op { "E" } else { &e },
      if drain_queue { "N" } else { &n },
      if sweep { "S" } else { &s }
    )
  }

  fn build_movement_status_string(&self) -> String {
    let movement = self.movement.lock().unwrap();
    movement.print_movements()
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

  fn set_move(&self, direction: Direction, canvas_size: Vec2) {
    let mut pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();
    let (dx, dy) = direction.get_direction();
    let next_pos = Vec2::new(
      pos.x.saturating_add_signed(dx as isize),
      pos.y.saturating_add_signed(dy as isize),
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
    let mut actived_pos = self.actived_pos.lock().unwrap();
    let arpeggiator = self.arpeggiator_mode.load(Ordering::Relaxed);
    let playhead_w = area.width();
    let playhead_h = area.height();
    let playhead_x = area.left();
    let playhead_y = area.top();
    let canvas_w = self.grid_width.load(Ordering::Relaxed);

    if arpeggiator {
      let regex_indexes = self.regex_indexes.lock().unwrap();
      let movement = self.movement.lock().unwrap();
      let movement_random = *movement == Movement::Random;
      let matches = playback_modes::get_arpeggiator_matches(
        &regex_indexes,
        playhead_x,
        playhead_y,
        playhead_w,
        playhead_h,
        canvas_w,
        *movement,
      );
      drop(regex_indexes);
      drop(movement);

      if !matches.is_empty() {
        let step = if movement_random {
          playback_modes::get_random_index(pos, matches.len())
        } else {
          pos % matches.len()
        };
        let (x, y) = matches[step];
        actived_pos.x = x;
        actived_pos.y = y;
      } else {
        let movement = self.movement.lock().unwrap();
        // No matches, fallback to normal running
        playback_modes::calculate_position_fallback(
          pos,
          playhead_w,
          playhead_h,
          *movement,
          &mut actived_pos,
        );
        drop(movement);
      }
    } else {
      let movement = self.movement.lock().unwrap();
      // Normal running without arpeggiator
      playback_modes::calculate_position_fallback(
        pos,
        playhead_w,
        playhead_h,
        *movement,
        &mut actived_pos,
      );
      drop(movement);
    }
  }

  /// get absolute position the same way it being displayed in console "POS: (x,y)" and flattened index
  fn calculate_absolute_position(&self, active_pos: Vec2) -> (usize, usize, usize) {
    let pos = self.pos.lock().unwrap();
    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let abs_y = pos.y + active_pos.y;
    let abs_x = pos.x + active_pos.x;
    let curr_running_playhead = (abs_y * grid_width) + abs_x;
    (abs_x, abs_y, curr_running_playhead)
  }

  // TODO: revisit keyboard-left
  fn determine_note_position_and_scale(
    &self,
    active_pos: Vec2,
    abs_x: usize,
    abs_y: usize,
  ) -> (usize, crate::core::scale::ScaleMode) {
    let mut prev_active = self.prev_active_pos.lock().unwrap();
    // let prev_active_pos = *prev_active;

    // let x_diff = active_pos.x.abs_diff(prev_active_pos.x);
    // let y_diff = active_pos.y.abs_diff(prev_active_pos.y);

    *prev_active = active_pos;
    drop(prev_active);

    let grid_height = self.grid_height.load(Ordering::Relaxed);
    // let is_horizontal =
    //   self.area.lock().unwrap().width() >= 1 && self.area.lock().unwrap().height() == 1;

    // if (x_diff > y_diff) || is_horizontal {
    //   // Horizontal movement: use top keyboard mapping
    //   let pos = if grid_height > 0 {
    //     abs_x % grid_height
    //   } else {
    //     abs_y
    //   };
    //   let scale = *self.scale_mode_top.lock().unwrap();
    //   (pos, scale)
    // } else {
    //   // Vertical movement: use left keyboard mapping
    //   let scale = *self.scale_mode_left.lock().unwrap();
    //   (abs_y, scale)
    // }
    let pos = if grid_height > 0 {
      abs_x % grid_height
    } else {
      abs_y
    };
    let scale = *self.scale_mode_top.lock().unwrap();
    (pos, scale)
  }

  fn trigger_midi_if_matched(
    &self,
    curr_running_playhead: usize,
    note_position: usize,
    scale_mode: crate::core::scale::ScaleMode,
  ) {
    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let grid_height = self.grid_height.load(Ordering::Relaxed);
    let current_tempo = self.tempo.load(Ordering::Relaxed);
    let pos = self.pos.lock().unwrap();

    let hold_next = self.hold_next_note.load(Ordering::Relaxed);
    let _ = self.midi_tx.send(midi::Message::TriggerWithPosition((
      curr_running_playhead,
      note_position,
      grid_width,
      grid_height,
      scale_mode,
      current_tempo,
      pos.y,
      hold_next,
      false, // no sweep mode for normal triggers
      pos.y, // active_pos_y (same as trigger_pos_y for normal triggers)
      self.scale_root_top.lock().unwrap().to_root_offset(),
    )));
  }

  fn trigger_midi_if_matched_sweep(&self, curr_running_playhead: usize, abs_x: usize) {
    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let grid_height = self.grid_height.load(Ordering::Relaxed);
    let current_tempo = self.tempo.load(Ordering::Relaxed);
    let active_pos_y = self.pos.lock().unwrap().y;

    // When sweep_mode is enabled, trigger MIDI for all positions along the vertical crosshair
    if self.sweep_mode.load(Ordering::Relaxed) {
      let x_scale_mode = *self.scale_mode_top.lock().unwrap();

      // Collect all matched y positions for the current x
      let mut matched_y_positions: Vec<usize> = Vec::new();

      // Iterate through all y positions for the current x (vertical crosshair)
      for y in 0..grid_height {
        let crosshair_index = y * grid_width + abs_x;

        // Skip current playhead position to avoid duplicate MIDI trigger
        if crosshair_index == curr_running_playhead {
          continue;
        }

        // Check if this position matches
        if let Some(matcher) = self.text_matcher.lock().unwrap().as_ref() {
          if matcher.contains_key(&crosshair_index) {
            matched_y_positions.push(y);
          }
        }
      }

      // If we have matched positions, send a single MIDI trigger with average velocity
      if !matched_y_positions.is_empty() {
        // Calculate average y position for velocity reference
        let avg_y = matched_y_positions.iter().sum::<usize>() / matched_y_positions.len();

        let x_note_position = if grid_height > 0 {
          abs_x % grid_height
        } else {
          avg_y
        };

        // Use the first matched index for reference (they all share the same x)
        let first_y = matched_y_positions[0];
        let reference_index = first_y * grid_width + abs_x;

        let _ = self.midi_tx.send(midi::Message::TriggerWithPosition((
          reference_index,
          x_note_position,
          grid_width,
          grid_height,
          x_scale_mode,
          current_tempo,
          avg_y,        // Use average y position for velocity calculation
          false,        // sweep mode notes don't use hold
          true,         // is_sweep
          active_pos_y, // reference Y position for velocity calculation
          self.scale_root_top.lock().unwrap().to_root_offset(),
        )));
      }
    }
  }

  fn check_contains(&self, area: &Rect, matcher: &HashMap<usize, regex::Match>) -> bool {
    for dy in 0..area.height() {
      for dx in 0..area.width() {
        let x = area.top_left.x + dx;
        let y = area.top_left.y + dy;
        let pos_index = y * self.grid_width.load(Ordering::Relaxed) + x;
        if matcher.contains_key(&pos_index) {
          return true;
        }
      }
    }
    false
  }

  fn handle_silent_step(&self, matcher: &HashMap<usize, regex::Match>, cb_sink: &cursive::CbSink) {
    if !self.accumulation_mode.load(Ordering::Relaxed) {
      return;
    }
    let area = self.area.lock().unwrap();
    let has_some_pos = self.check_contains(&area, matcher);
    let playhead_area_size = area.width() * area.height();
    drop(area);

    if has_some_pos {
      return;
    };

    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter += 1;
    let current_count = *counter;
    self.update_accumulation_ui(current_count, playhead_area_size, cb_sink);
    if *counter >= playhead_area_size {
      *counter = 0;
      drop(counter);
      self.update_accumulation_ui(0, playhead_area_size, cb_sink);
      self.perform_accumulation_jump();
    } else {
      drop(counter);
      self.update_accumulation_ui(current_count, playhead_area_size, cb_sink);
    }
  }

  fn handle_accumulation_mode(&self, abs_x: usize, cb_sink: &cursive::CbSink) -> Option<Vec2> {
    self.check_n_execute_operators(abs_x);

    let area = self.area.lock().unwrap();
    let playhead_area_size = area.width() * area.height();
    drop(area);

    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter += 1;
    let current_count = *counter;

    if *counter >= playhead_area_size {
      *counter = 0;
      drop(counter);

      self.update_accumulation_ui(0, playhead_area_size, cb_sink);
      Some(self.perform_accumulation_jump())
    } else {
      drop(counter);
      self.update_accumulation_ui(current_count, playhead_area_size, cb_sink);
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

    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let grid_height = self.grid_height.load(Ordering::Relaxed);

    let current_playhead_pos = {
      let pos = self.pos.lock().unwrap();
      (pos.x, pos.y)
    };

    // Determine jump position from stack or random
    let params = GridParams {
      playhead_width,
      playhead_height,
      grid_width,
      grid_height,
      rng: &mut rng,
    };
    let (new_x, new_y) = self.get_jump_position(current_playhead_pos, params);

    // Update playhead position and area
    let mut pos = self.pos.lock().unwrap();
    pos.x = new_x;
    pos.y = new_y;
    let new_pos = *pos;
    drop(pos);

    let mut area = self.area.lock().unwrap();
    *area = Rect::from_size((new_x, new_y), (playhead_width, playhead_height));
    let new_area = *area;
    drop(area);

    // Reset active position to start
    let mut actived = self.actived_pos.lock().unwrap();
    *actived = Vec2::zero();
    drop(actived);

    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::PlayheadPosAndArea(new_pos, new_area));

    Vec2::zero()
  }

  fn get_jump_position<R: rand::Rng>(
    &self,
    current_playhead_pos: (usize, usize),
    params: GridParams<R>,
  ) -> (usize, usize) {
    let mut queue = self.operator_queue.lock().unwrap();

    if let Some(first_item) = queue.first() {
      match first_item {
        QueueItem::Position(x, y) => {
          let first_pos = (*x, *y);
          if first_pos == current_playhead_pos {
            // Same position, jump randomly
            drop(queue);
            self.generate_random_position(
              params.playhead_width,
              params.playhead_height,
              params.grid_width,
              params.grid_height,
              params.rng,
            )
          } else {
            // Use position from queue (pop from front - FIFO)
            let item = queue.remove(0);
            // let is_drain = self.drain_queue_mode.load(Ordering::Relaxed);

            // Format queue using Display trait for consistency
            // let queue_display = if queue.is_empty() {
            //   if is_drain {
            //     format!("{}[]", consts::SYMBOL_DRAIN)
            //   } else {
            //     "[]".to_string()
            //   }
            // } else {
            //   let items: Vec<String> = queue.iter().map(|item| format!("{}", item)).collect();
            //   if is_drain {
            //     format!("{}[{}]", consts::SYMBOL_DRAIN, items.join(", "))
            //   } else {
            //     format!("[{}]", items.join(", "))
            //   }
            // };
            // drop(queue);

            if let QueueItem::Position(x, y) = item {
              let mut pushed = self.pushed_positions.lock().unwrap();
              pushed.remove(&(x, y));
              drop(pushed);

              // Queue UI update (batched processing)
              // let mut ui_queue = self.ui_update_queue.lock().unwrap();
              // ui_queue.push_back(UIUpdate::OpQueueDisplay(queue_display));
              // drop(ui_queue);

              (x, y)
            } else {
              // This shouldn't happen as we checked first_item was Position
              (0, 0)
            }
          }
        }
        QueueItem::Event(_) => {
          // Event at front of queue, can't jump to it, generate random position
          drop(queue);
          self.generate_random_position(
            params.playhead_width,
            params.playhead_height,
            params.grid_width,
            params.grid_height,
            params.rng,
          )
        }
      }
    } else {
      drop(queue);
      self.generate_random_position(
        params.playhead_width,
        params.playhead_height,
        params.grid_width,
        params.grid_height,
        params.rng,
      )
    }
  }

  fn generate_random_position(
    &self,
    playhead_width: usize,
    playhead_height: usize,
    grid_width: usize,
    grid_height: usize,
    rng: &mut impl rand::Rng,
  ) -> (usize, usize) {
    let max_x = grid_width.saturating_sub(playhead_width);
    let max_y = grid_height.saturating_sub(playhead_height);

    if max_x > 0 && max_y > 0 {
      (rng.gen_range(0..=max_x), rng.gen_range(0..=max_y))
    } else {
      (0, 0)
    }
  }

  fn update_active_pos_ui(&self, active_pos: Vec2, _cb_sink: &cursive::CbSink) {
    // Queue UI update instead of immediate send (batched processing)
    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::ActivePos(active_pos));
  }

  pub fn toggle_arpeggiator_mode(&self, cb_sink: cursive::CbSink) {
    let is_arp = !self.arpeggiator_mode.load(Ordering::Relaxed);
    self.arpeggiator_mode.store(is_arp, Ordering::Relaxed);

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
    let is_event_op = !self.event_operator_mode.load(Ordering::Relaxed);
    self
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
    let is_drain = !self.drain_queue_mode.load(Ordering::Relaxed);
    self.drain_queue_mode.store(is_drain, Ordering::Relaxed);

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
        // siv.call_on_name(consts::op_queue_status_unit_view, |view: &mut TextView| {
        //   let current = view.get_content().source().to_string();
        //   if is_drain {
        //     view.set_content(format!("{}{}", consts::SYMBOL_DRAIN, current));
        //   } else if current.starts_with(*consts::SYMBOL_DRAIN) {
        //     let unlocked = current
        //       .trim_start_matches(*consts::SYMBOL_DRAIN)
        //       .to_string();
        //     view.set_content(unlocked);
        //   } else {
        //     view.set_content(current);
        //   }
        // });
      }))
      .unwrap();
  }

  pub fn toggle_sweep_mode(&self, cb_sink: cursive::CbSink) {
    let is_sweep = !self.sweep_mode.load(Ordering::Relaxed);
    self.sweep_mode.store(is_sweep, Ordering::Relaxed);

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

  pub fn cycle_scale_root(&self, cb_sink: cursive::CbSink, dir: crate::core::command::Adjustment) {
    let mut root = self.scale_root_top.lock().unwrap();
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

  pub fn cycle_scale_mode(&self, cb_sink: cursive::CbSink, dir: crate::core::command::Adjustment) {
    let mut mode = self.scale_mode_top.lock().unwrap();
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

    *area = Rect::from_size(
      *pos,
      ((area.width() as i32) + w, (area.height() as i32) - h),
    );
  }

  pub fn set_text_matcher(&self, text_matcher: Option<HashMap<usize, Match>>) {
    let mut tm = self.text_matcher.lock().unwrap();
    *tm = text_matcher
  }

  // Queue operators: P (Push), S (Swap), O (pOp), D (Duplicate) with narrow spacing
  // Event operators: r, c, x with wider spacing
  fn check_n_execute_operators(&self, abs_x: usize) {
    // Check if this is a "space" position (where event operators are defined)
    let is_space = abs_x.is_multiple_of(consts::EVENT_OP_SPACING);

    // Only execute queue operators if NOT on a space position
    if abs_x.is_multiple_of(consts::QUEUE_OP_SPACING) && !is_space {
      let position_index = abs_x / consts::QUEUE_OP_SPACING;
      self.execute_queue_operator(position_index);
    }

    // Event operators only execute on space positions when event_operator_mode is enabled
    if is_space && self.event_operator_mode.load(Ordering::Relaxed) {
      let position_index = abs_x / consts::EVENT_OP_SPACING;
      self.execute_event_operator(position_index);
    }
  }

  fn execute_front_event_op_in_queue(&self) {
    // if front of queue is an event, we execute it immediately
    let op_front = self.operator_queue.lock().unwrap().first().cloned();
    if let Some(QueueItem::Event(ev_op)) = op_front {
      match ev_op {
        EventOperator::X => {
          self.hold_next_note.store(
            !self.hold_next_note.load(Ordering::Relaxed),
            Ordering::Relaxed,
          );
        }
        EventOperator::R | EventOperator::C => {
          // NO OP for now.
        }
      }
    }
  }

  fn execute_queue_operator(&self, position_index: usize) {
    let operator_index = position_index % QUEUE_OPERATORS.len();
    let operator = QUEUE_OPERATORS[operator_index];

    let is_drain = self.drain_queue_mode.load(Ordering::Relaxed);

    match operator {
      QueueOperator::Push => {
        if !is_drain {
          self.handle_push();
        }
      }
      QueueOperator::Swap => {
        if !is_drain {
          self.handle_swap();
        }
      }
      QueueOperator::Pop => {
        // Pop is always allowed, even when drain mode is active
        self.handle_pop();
      }
      QueueOperator::Duplicate => {
        if !is_drain {
          self.handle_duplicate();
        }
      }
    }
  }

  fn execute_event_operator(&self, position_index: usize) {
    let operator_index = position_index % EVENT_OPERATORS.len();
    let operator = EVENT_OPERATORS[operator_index];

    match operator {
      EventOperator::R => self.handle_r(),
      EventOperator::C => self.handle_c(),
      EventOperator::X => self.handle_x(),
    }
  }

  fn handle_push(&self) {
    // Check event queue first (FIFO)
    let mut event_queue = self.event_queue.lock().unwrap();
    let event_op = event_queue.dequeue();

    if let Some(event_op) = event_op {
      drop(event_queue);

      // Push the event to the main queue
      let mut queue = self.operator_queue.lock().unwrap();
      if queue.len() < queue.capacity() {
        queue.push(QueueItem::Event(event_op));
      }
      drop(queue);

      self.update_queue_display();
    } else {
      drop(event_queue);

      // No events in queue, push current playhead position
      let playhead_pos = self.pos.lock().unwrap();
      let push_pos = (playhead_pos.x, playhead_pos.y);
      drop(playhead_pos);

      let mut pushed = self.pushed_positions.lock().unwrap();
      if let Entry::Vacant(e) = pushed.entry(push_pos) {
        e.insert(true);
        drop(pushed);

        let mut queue = self.operator_queue.lock().unwrap();
        if queue.len() < queue.capacity() {
          queue.push(QueueItem::Position(push_pos.0, push_pos.1));
        }
        drop(queue);

        self.update_queue_display();
      }
    }
  }

  fn handle_swap(&self) {
    let mut queue = self.operator_queue.lock().unwrap();
    let len = queue.len();
    if len >= 2 {
      queue.swap(len - 1, len - 2);
    }
    drop(queue);

    self.update_queue_display();
  }

  fn handle_pop(&self) {
    let mut queue = self.operator_queue.lock().unwrap();
    if !queue.is_empty() {
      let item = queue.remove(0);
      drop(queue);

      // Only remove from pushed_positions if it was a position
      if let QueueItem::Position(x, y) = item {
        let mut pushed = self.pushed_positions.lock().unwrap();
        pushed.remove(&(x, y));
        drop(pushed);
      }

      // self.update_queue_display();
    }
  }

  fn handle_duplicate(&self) {
    let mut queue = self.operator_queue.lock().unwrap();
    if let Some(item) = queue.last().cloned() {
      if queue.len() < queue.capacity() {
        queue.push(item);
      }
    }
    drop(queue);

    // self.update_queue_display();
  }

  // TODO: ? maybe obsolete
  fn update_queue_display(&self) {
    let queue = self.operator_queue.lock().unwrap();
    let event_queue = self.event_queue.lock().unwrap();
    // let drain_queue = self.drain_queue_mode.load(Ordering::Relaxed);

    // Format operator queue with Display trait for clean output
    // let queue_display = if queue.is_empty() {
    //   if drain_queue {
    //     format!("{}[]", consts::SYMBOL_DRAIN)
    //   } else {
    //     "[]".to_string()
    //   }
    // } else {
    //   let items: Vec<String> = queue.iter().map(|item| format!("{}", item)).collect();
    //   if drain_queue {
    //     format!("{}[{}]", consts::SYMBOL_DRAIN, items.join(", "))
    //   } else {
    //     format!("[{}]", items.join(", "))
    //   }
    // };

    // Format event queue with Display trait
    // let event_queue_display = if event_queue.is_empty() {
    //   "[]".to_string()
    // } else {
    //   let items: Vec<String> = event_queue.iter().map(|op| format!("{}", op)).collect();
    //   format!("[{}]", items.join(", "))
    // };

    drop(queue);
    drop(event_queue);

    // let mut ui_queue = self.ui_update_queue.lock().unwrap();
    // ui_queue.push_back(UIUpdate::OpQueueDisplay(queue_display));
    // ui_queue.push_back(UIUpdate::EvQueueDisplay(event_queue_display));
  }

  fn handle_r(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.enqueue(EventOperator::R);
    drop(event_queue);

    self.update_queue_display();
  }

  fn handle_c(&self) {
    let actived_pos = *self.actived_pos.lock().unwrap();
    let playhead_pos = *self.pos.lock().unwrap();
    let abs_x = playhead_pos.x + actived_pos.x;
    let abs_y = playhead_pos.y + actived_pos.y;

    let scale_mode = *self.scale_mode_top.lock().unwrap();
    let scale_root_offset = self.scale_root_top.lock().unwrap().to_root_offset();
    let grid_height = self.grid_height.load(Ordering::Relaxed);

    if grid_height == 0 {
      return;
    }

    let note_position = abs_x % grid_height;

    // Get the base note
    let (_base_note_index, base_octave) = scale_mode.pos_to_scale_note(
      note_position,
      grid_height,
      consts::BASE_OCTAVE,
      scale_root_offset,
    );

    // For a triad: root (0), third (2 scale degrees up), fifth (4 scale degrees up)
    let scale_intervals = scale_mode.intervals();
    let scale_length = scale_intervals.len();

    // Calculate base scale degree
    let inverted_y = grid_height.saturating_sub(1).saturating_sub(note_position);
    let base_scale_degree = inverted_y % scale_length;

    // maybe duplicated code, just dynamic velo based on y-axis
    let max_vel = 100.0;
    let min_vel = 10.0;
    let ref_velocity = max_vel - (abs_y as f32 / grid_height as f32) * (max_vel - min_vel);
    let velocity = ref_velocity.round().max(min_vel) as u8;

    // Calculate dynamic note length based on BPM
    let current_tempo = self.tempo.load(Ordering::Relaxed);
    let base_bpm = consts::DEFAULT_TEMPO;
    let base_length = 4; // Base length for chords
    let calculated_length = if current_tempo > 0 {
      ((base_length * base_bpm) / current_tempo).max(1)
    } else {
      base_length
    };
    let note_length = (calculated_length as u8).min(127);

    // Chord notes: root, third, fifth
    let chord_degrees = [0, 2, 4];
    let channel = 0;

    let mut chord_notes = Vec::new();

    for &degree_offset in &chord_degrees {
      let target_scale_degree = (base_scale_degree + degree_offset) % scale_length;
      let octave_jump = (base_scale_degree + degree_offset) / scale_length;

      let interval = scale_intervals[target_scale_degree];
      let raw_pitch = interval + (scale_root_offset % 12) as f32;
      let note_index = raw_pitch % 12.0;
      let extra_octave = (raw_pitch / 12.0).floor() as u8;
      let final_octave = base_octave + octave_jump as u8 + extra_octave;

      let midi_msg = midi::MidiMsg::from(
        note_index,
        final_octave,
        note_length,
        velocity,
        channel,
        false,
      );

      chord_notes.push(midi_msg);
    }

    for midi_msg in &chord_notes {
      let _ = self
        .midi_tx
        .send(midi::Message::Trigger(midi_msg.clone(), true));
    }

    // Use BPM-aware timing for chord duration (matches Stack behavior)
    let chord_duration_ms = (note_length as u64 * 8).max(50);

    let midi_tx_clone = self.midi_tx.clone();
    thread::spawn(move || {
      thread::sleep(Duration::from_millis(chord_duration_ms));
      for midi_msg in chord_notes {
        let _ = midi_tx_clone.send(midi::Message::Trigger(midi_msg, false));
      }
    });
  }

  fn handle_x(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.enqueue(EventOperator::X);
    drop(event_queue);

    self.update_queue_display();
  }

  // fn is_actived_position(&self, curr_pos: Vec2) -> bool {
  //   let pos = self.pos.lock().unwrap();
  //   let actived_pos = self.actived_pos.lock().unwrap();
  //   pos.saturating_add(*actived_pos).eq(&curr_pos)
  // }

  // #[cfg(debug_assertions)]
  // pub fn spawn_stats_printer(self: &Arc<Self>) {
  //   let stats = Arc::clone(&self.timing_stats);
  //   thread::spawn(move || loop {
  //     thread::sleep(Duration::from_secs(10));
  //     stats.print_stats();
  //     stats.reset();
  //   });
  // }

  pub fn run(self: Arc<Self>) -> Sender<Message> {
    let (tx, rx) = channel();

    // #[cfg(debug_assertions)]
    // self.spawn_stats_printer(); // Start stats printer

    thread::spawn(move || {
      for control_message in &rx {
        match control_message {
          Message::Move(direction, canvas_size, cb_sink) => {
            self.set_move(direction.clone(), canvas_size);

            // Reset accumulation counter on user interaction
            let mut counter = self.accumulation_counter.lock().unwrap();
            *counter = 0;
            drop(counter);

            let pos_mutex = self.pos.lock().unwrap();
            let pos = *pos_mutex;

            let area_mutex = self.area.lock().unwrap();
            let area = *area_mutex;

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_pos_status_str(pos));
                });

                siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
                  view.set_content("-");
                });

                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<GridEditor>| {
                    let editor = canvas.state_mut();
                    editor.playhead_ui.playhead_pos = pos;
                    editor.playhead_ui.playhead_area = area;
                  },
                );
              }))
              .unwrap();
          }
          Message::SetCurrentPos(position, offset, cb_sink) => {
            self.set_current_pos(position, offset);

            // Reset accumulation counter on user interaction
            let mut counter = self.accumulation_counter.lock().unwrap();
            *counter = 0;
            drop(counter);

            let mutex_pos = self.pos.lock().unwrap();
            let pos = *mutex_pos;
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
              }))
              .unwrap();
          }
          Message::UpdateInfoStatusView(cb_sink) => {
            let pos = self.pos.lock().unwrap();
            let area = self.area.lock().unwrap();
            let pos_x = pos.x;
            let pos_y = pos.y;
            let w = area.width();
            let h = area.height();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_pos_status_str((pos_x, pos_y).into()))
                });

                siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_len_status_str((w, h)));
                });
              }))
              .unwrap();
          }
          Message::SetGridArea(current_pos, cb_sink) => {
            self.set_grid_area(current_pos);

            // Reset accumulation counter on user interaction
            let mut counter = self.accumulation_counter.lock().unwrap();
            *counter = 0;
            drop(counter);

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
          Message::SetActivePos(tick, cb_sink) => {
            // #[cfg(debug_assertions)]
            // let start = Instant::now();

            let ratio = self.ratio.lock().unwrap();
            let divider = 16 / ratio.1;
            drop(ratio);

            // Only advance step_index when ratio triggers
            let should_advance = tick % divider == 0;
            if should_advance {
              let mut step_idx = self.step_index.lock().unwrap();
              *step_idx += 1;
              self.set_actived_pos(*step_idx);
              drop(step_idx);

              let active_pos_mutex = self.actived_pos.lock().unwrap();
              let mut active_pos = *active_pos_mutex;
              drop(active_pos_mutex);

              let (abs_x, abs_y, curr_running_playhead) =
                self.calculate_absolute_position(active_pos);

              let (note_position, scale_mode) =
                self.determine_note_position_and_scale(active_pos, abs_x, abs_y);

              // ? should sweep mode effect accumulation value
              // Handle accumulation mode (for position operators)
              if let Some(matcher) = self.text_matcher.lock().unwrap().as_ref() {
                if matcher.get(&curr_running_playhead).is_some() {
                  if self.accumulation_mode.load(Ordering::Relaxed) {
                    if let Some(new_active_pos) = self.handle_accumulation_mode(abs_x, &cb_sink) {
                      active_pos = new_active_pos;
                    }
                    self.execute_front_event_op_in_queue();
                  }
                  self.trigger_midi_if_matched(curr_running_playhead, note_position, scale_mode);
                  if self.hold_next_note.load(Ordering::Relaxed) {
                    self.hold_next_note.store(false, Ordering::Relaxed);
                    self.operator_queue.lock().unwrap().remove(0);
                    self.update_queue_display();
                  }
                }
                self.handle_silent_step(matcher, &cb_sink);
              }
              self.trigger_midi_if_matched_sweep(curr_running_playhead, abs_x);
              self.update_active_pos_ui(active_pos, &cb_sink);
            }

            // #[cfg(debug_assertions)]
            // {
            //   let elapsed = start.elapsed().as_micros() as u64;
            //   self.timing_stats.record(elapsed);

            //   // Immediate warning for slow calls
            //   if elapsed > 1000 {
            //     eprintln!(
            //       "⚠️  SLOW: SetActivePos took {}μs ({:.2}ms)",
            //       elapsed,
            //       elapsed as f64 / 1000.0
            //     );
            //   }
            // }
          }
          Message::Scale(size, cb_sink) => {
            self.scale(size);

            // Reset accumulation counter on user interaction
            let mut counter = self.accumulation_counter.lock().unwrap();
            *counter = 0;
            drop(counter);

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
          Message::SetMatcher(matcher, cb_sink) => {
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
          Message::SetGridSize(width, height, cb_sink) => {
            self.grid_width.store(width, Ordering::Relaxed);
            self.grid_height.store(height, Ordering::Relaxed);

            // Clone queue references to share with PlayheadUI
            let operator_queue_cloned = self.operator_queue.clone();
            let event_queue_cloned = self.event_queue.clone();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<GridEditor>| {
                    let editor = canvas.state_mut();
                    editor.playhead_ui.operator_queue = operator_queue_cloned;
                    editor.playhead_ui.event_queue = event_queue_cloned;
                  },
                );
              }))
              .unwrap();
          }
          Message::SetScaleModeLeft(scale_mode) => {
            let mut mode = self.scale_mode_left.lock().unwrap();
            *mode = scale_mode;
          }
          Message::SetScaleRootTop(scale_root) => {
            let mut root = self.scale_root_top.lock().unwrap();
            *root = scale_root;
          }
          Message::SetScaleModeTop(scale_mode) => {
            let mut mode = self.scale_mode_top.lock().unwrap();
            *mode = scale_mode;
          }
          Message::ToggleAccumulationMode(cb_sink) => {
            let is_enabled = !self.accumulation_mode.load(Ordering::Relaxed);
            self.accumulation_mode.store(is_enabled, Ordering::Relaxed);

            // Reset counter when toggling mode
            let mut counter = self.accumulation_counter.lock().unwrap();
            *counter = 0;
            drop(counter);

            // Clear queue when disabling accumulation mode
            if !is_enabled {
              let mut queue = self.operator_queue.lock().unwrap();
              queue.clear();
              drop(queue);

              let mut pushed = self.pushed_positions.lock().unwrap();
              pushed.clear();
              drop(pushed);
            }

            let mode_status = self.build_mode_status_string();

            // Update UI to clear accumulation display and queue display
            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
                  view.set_content("-");
                });

                // if !is_enabled {
                //   siv.call_on_name(consts::op_queue_status_unit_view, |view: &mut TextView| {
                //     view.set_content("[]");
                //   });
                // }

                siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
                  view.set_content(mode_status);
                });
              }))
              .unwrap();
          }
          Message::SetTempo(bpm) => {
            self.tempo.store(bpm, Ordering::Relaxed);
          }
          Message::SetRatio(new_ratio, cb_sink) => {
            let mut ratio = self.ratio.lock().unwrap();
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
          Message::CycleScaleRootTop(cb_sink, dir) => {
            self.cycle_scale_root(cb_sink, dir);
          }
          Message::CycleScaleMode(cb_sink, dir) => {
            self.cycle_scale_mode(cb_sink, dir);
          }
        }
      }
    });

    tx
  }
}
