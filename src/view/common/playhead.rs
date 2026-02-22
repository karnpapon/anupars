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

use crate::core::{consts, midi, playback_modes, rect::Rect, regex::Match, utils};
use crate::view::common::grid_editor::CanvasEditor;
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
      QueueItem::Position(x, y) => write!(f, "({},{})", x, y),
      QueueItem::Event(op) => write!(f, "{}", op),
    }
  }
}

// UI update types for batching
#[derive(Clone, Debug)]
pub enum UIUpdate {
  ActivePos(Vec2),
  AccumulationCounter(usize, usize), // (count, total)
  OpQueueDisplay(String),
  EvQueueDisplay(String),
  MarkerPosAndArea(Vec2, Rect),
}

struct GridParams<'a, R: rand::Rng> {
  pub marker_width: usize,
  pub marker_height: usize,
  pub grid_width: usize,
  pub grid_height: usize,
  pub rng: &'a mut R,
}

pub struct MarkerUI {
  pub marker_area: Rect,
  pub marker_pos: Vec2,
  pub actived_pos: Vec2,
  pub text_matcher: Option<HashMap<usize, Match>>,
  pub regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  pub reverse_mode: bool,
  pub arpeggiator_mode: bool,
  pub random_mode: bool,
}

impl MarkerUI {
  pub fn new() -> Self {
    MarkerUI {
      marker_area: Rect::from_point(Vec2::zero()),
      marker_pos: Vec2::zero(),
      actived_pos: Vec2::zero(),
      text_matcher: None,
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      reverse_mode: false,
      arpeggiator_mode: false,
      random_mode: false,
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
  SetGridSize(usize, usize),
  SetScaleModeLeft(crate::core::scale::ScaleMode),
  SetScaleModeTop(crate::core::scale::ScaleMode),
  ToggleAccumulationMode(cursive::CbSink),
  ToggleReverseMode(cursive::CbSink),
  ToggleArpeggiatorMode(cursive::CbSink),
  ToggleRandomMode(cursive::CbSink),
  SetTempo(usize),
  SetRatio((i64, usize), cursive::CbSink),
}

pub struct MarkerArea {
  pos: Arc<Mutex<Vec2>>,
  area: Arc<Mutex<Rect>>,
  drag_start_x: AtomicUsize,
  drag_start_y: AtomicUsize,
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
  accumulation_counter: Arc<Mutex<usize>>,
  accumulation_mode: AtomicBool,
  reverse_mode: AtomicBool,
  arpeggiator_mode: AtomicBool,
  random_mode: AtomicBool,
  ratio: Arc<Mutex<(i64, usize)>>,
  operator_queue: Arc<Mutex<VecDeque<QueueItem>>>,
  event_queue: Arc<Mutex<VecDeque<EventOperator>>>,
  pushed_positions: Arc<Mutex<HashMap<(usize, usize), bool>>>,
  pub ui_update_queue: Arc<Mutex<VecDeque<UIUpdate>>>,
  // #[cfg(debug_assertions)]
  // timing_stats: Arc<TimingStats>,
}

impl MarkerArea {
  pub fn new(midi_tx: Sender<midi::Message>) -> Self {
    MarkerArea {
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
      tempo: AtomicUsize::new(120),
      prev_active_pos: Arc::new(Mutex::new(Vec2::zero())),
      scale_mode_left: Arc::new(Mutex::new(crate::core::scale::ScaleMode::default())),
      scale_mode_top: Arc::new(Mutex::new(crate::core::scale::ScaleMode::default())),
      accumulation_counter: Arc::new(Mutex::new(0)),
      accumulation_mode: AtomicBool::new(false),
      reverse_mode: AtomicBool::new(false),
      arpeggiator_mode: AtomicBool::new(false),
      random_mode: AtomicBool::new(false),
      ratio: Arc::new(Mutex::new((1, 16))),
      operator_queue: Arc::new(Mutex::new(VecDeque::new())),
      event_queue: Arc::new(Mutex::new(VecDeque::new())),
      pushed_positions: Arc::new(Mutex::new(HashMap::new())),
      ui_update_queue: Arc::new(Mutex::new(VecDeque::new())),
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
                    move |canvas: &mut Canvas<CanvasEditor>| {
                      let editor = canvas.state_mut();
                      editor.marker_ui.actived_pos = active_pos;
                    },
                  );
                }
                UIUpdate::AccumulationCounter(count, total) => {
                  siv.call_on_name(
                    consts::input_status_unit_view,
                    move |view: &mut TextView| {
                      view.set_content(format!("acc: {}/{}", count, total));
                    },
                  );
                }
                UIUpdate::OpQueueDisplay(queue_str) => {
                  siv.call_on_name(
                    consts::op_queue_status_unit_view,
                    move |view: &mut TextView| {
                      view.set_content(queue_str);
                    },
                  );
                }
                UIUpdate::EvQueueDisplay(queue_str) => {
                  siv.call_on_name(
                    consts::ev_queue_status_unit_view,
                    move |view: &mut TextView| {
                      view.set_content(queue_str);
                    },
                  );
                }
                UIUpdate::MarkerPosAndArea(pos, area) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<CanvasEditor>| {
                      let editor = canvas.state_mut();
                      editor.marker_ui.marker_pos = pos;
                      editor.marker_ui.marker_area = area;
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
    let reverse = self.reverse_mode.load(Ordering::Relaxed);
    let arpeggiator = self.arpeggiator_mode.load(Ordering::Relaxed);
    let accumulation = self.accumulation_mode.load(Ordering::Relaxed);
    let random = self.random_mode.load(Ordering::Relaxed);

    format!(
      "{}{}{}{}",
      if reverse { "R" } else { "r" },
      if arpeggiator { "A" } else { "a" },
      if accumulation { "U" } else { "u" },
      if random { "D" } else { "d" }
    )
  }

  pub fn toggle_reverse_mode(&self, cb_sink: cursive::CbSink) {
    let is_reversed = !self.reverse_mode.load(Ordering::Relaxed);
    self.reverse_mode.store(is_reversed, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<CanvasEditor>| {
            let editor = canvas.state_mut();
            editor.reverse_mode = is_reversed;
            editor.marker_ui.reverse_mode = is_reversed;
          },
        );

        siv.call_on_name(consts::osc_status_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
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

  fn calculate_adjusted_pos(&self, pos: usize) -> usize {
    let ratio = *self.ratio.lock().unwrap();
    let divisor = std::cmp::max(1, 16 / ratio.1);
    pos / divisor
  }

  pub fn set_actived_pos(&self, pos: usize) {
    let area = self.area.lock().unwrap();
    let mut actived_pos = self.actived_pos.lock().unwrap();
    let reverse = self.reverse_mode.load(Ordering::Relaxed);
    let arpeggiator = self.arpeggiator_mode.load(Ordering::Relaxed);
    let random = self.random_mode.load(Ordering::Relaxed);
    let marker_w = area.width();
    let marker_h = area.height();
    let marker_x = area.left();
    let marker_y = area.top();
    let canvas_w = self.grid_width.load(Ordering::Relaxed);

    let adjusted_pos = self.calculate_adjusted_pos(pos);

    if arpeggiator {
      let regex_indexes = self.regex_indexes.lock().unwrap();
      let matches = playback_modes::get_arpeggiator_matches(
        &regex_indexes,
        marker_x,
        marker_y,
        marker_w,
        marker_h,
        canvas_w,
        reverse,
      );
      drop(regex_indexes);

      if !matches.is_empty() {
        let step = if random {
          playback_modes::get_random_index(adjusted_pos, matches.len())
        } else {
          adjusted_pos % matches.len()
        };
        let (x, y) = matches[step];
        actived_pos.x = x;
        actived_pos.y = y;
      } else {
        // No matches, fallback to normal running
        playback_modes::calculate_position_fallback(
          adjusted_pos,
          marker_w,
          marker_h,
          reverse,
          random,
          &mut actived_pos,
        );
      }
    } else {
      // Normal running without arpeggiator
      playback_modes::calculate_position_fallback(
        adjusted_pos,
        marker_w,
        marker_h,
        reverse,
        random,
        &mut actived_pos,
      );
    }
  }

  fn calculate_absolute_position(&self, active_pos: Vec2) -> (usize, usize, usize) {
    let pos = self.pos.lock().unwrap();
    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let abs_y = pos.y + active_pos.y;
    let abs_x = pos.x + active_pos.x;
    let curr_running_marker = (abs_y * grid_width) + abs_x;
    (abs_x, abs_y, curr_running_marker)
  }

  fn determine_note_position_and_scale(
    &self,
    active_pos: Vec2,
    abs_x: usize,
    abs_y: usize,
  ) -> (usize, crate::core::scale::ScaleMode) {
    let mut prev_active = self.prev_active_pos.lock().unwrap();
    let prev_active_pos = *prev_active;

    let x_diff = active_pos.x.abs_diff(prev_active_pos.x);
    let y_diff = active_pos.y.abs_diff(prev_active_pos.y);

    *prev_active = active_pos;
    drop(prev_active);

    let grid_height = self.grid_height.load(Ordering::Relaxed);

    if x_diff > y_diff {
      // Horizontal movement: use top keyboard mapping
      let pos = if grid_height > 0 {
        abs_x % grid_height
      } else {
        abs_y
      };
      let scale = *self.scale_mode_top.lock().unwrap();
      (pos, scale)
    } else {
      // Vertical movement: use left keyboard mapping
      let scale = *self.scale_mode_left.lock().unwrap();
      (abs_y, scale)
    }
  }

  fn trigger_midi_if_matched(
    &self,
    curr_running_marker: usize,
    note_position: usize,
    scale_mode: crate::core::scale::ScaleMode,
  ) -> bool {
    if let Some(matcher) = self.text_matcher.lock().unwrap().as_ref() {
      if matcher.get(&curr_running_marker).is_some() {
        let grid_width = self.grid_width.load(Ordering::Relaxed);
        let grid_height = self.grid_height.load(Ordering::Relaxed);
        let current_tempo = self.tempo.load(Ordering::Relaxed);

        let _ = self.midi_tx.send(midi::Message::TriggerWithPosition((
          curr_running_marker,
          note_position,
          grid_width,
          grid_height,
          scale_mode,
          current_tempo,
        )));
        return true;
      }
    }
    false
  }

  fn handle_accumulation_mode(&self, abs_x: usize, cb_sink: &cursive::CbSink) -> Option<Vec2> {
    if !self.accumulation_mode.load(Ordering::Relaxed) {
      return None;
    }

    self.check_operators(abs_x);

    let area = self.area.lock().unwrap();
    let marker_area_size = area.width() * area.height();
    drop(area);

    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter += 1;
    let current_count = *counter;

    if *counter >= marker_area_size {
      *counter = 0;
      drop(counter);

      self.update_accumulation_ui(0, marker_area_size, cb_sink);
      Some(self.perform_accumulation_jump())
    } else {
      drop(counter);
      self.update_accumulation_ui(current_count, marker_area_size, cb_sink);
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
    let marker_width = area.width();
    let marker_height = area.height();
    drop(area);

    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let grid_height = self.grid_height.load(Ordering::Relaxed);

    let current_marker_pos = {
      let pos = self.pos.lock().unwrap();
      (pos.x, pos.y)
    };

    // Determine jump position from stack or random
    let params = GridParams {
      marker_width,
      marker_height,
      grid_width,
      grid_height,
      rng: &mut rng,
    };
    let (new_x, new_y) = self.get_jump_position(current_marker_pos, params);

    // Update marker position and area
    let mut pos = self.pos.lock().unwrap();
    pos.x = new_x;
    pos.y = new_y;
    let new_pos = *pos;
    drop(pos);

    let mut area = self.area.lock().unwrap();
    *area = Rect::from_size((new_x, new_y), (marker_width, marker_height));
    let new_area = *area;
    drop(area);

    // Reset active position to start
    let mut actived = self.actived_pos.lock().unwrap();
    *actived = Vec2::zero();
    drop(actived);

    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::MarkerPosAndArea(new_pos, new_area));

    Vec2::zero()
  }

  fn get_jump_position<R: rand::Rng>(
    &self,
    current_marker_pos: (usize, usize),
    params: GridParams<R>,
  ) -> (usize, usize) {
    let mut queue = self.operator_queue.lock().unwrap();

    if let Some(first_item) = queue.front() {
      match first_item {
        QueueItem::Position(x, y) => {
          let first_pos = (*x, *y);
          if first_pos == current_marker_pos {
            // Same position, jump randomly
            drop(queue);
            self.generate_random_position(
              params.marker_width,
              params.marker_height,
              params.grid_width,
              params.grid_height,
              params.rng,
            )
          } else {
            // Use position from queue (pop from front - FIFO)
            let item = queue.pop_front().unwrap();
            let queue_display = format!("{:?}", *queue);
            drop(queue);

            if let QueueItem::Position(x, y) = item {
              let mut pushed = self.pushed_positions.lock().unwrap();
              pushed.remove(&(x, y));
              drop(pushed);

              // Queue UI update (batched processing)
              let mut ui_queue = self.ui_update_queue.lock().unwrap();
              ui_queue.push_back(UIUpdate::OpQueueDisplay(queue_display));
              drop(ui_queue);

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
            params.marker_width,
            params.marker_height,
            params.grid_width,
            params.grid_height,
            params.rng,
          )
        }
      }
    } else {
      drop(queue);
      self.generate_random_position(
        params.marker_width,
        params.marker_height,
        params.grid_width,
        params.grid_height,
        params.rng,
      )
    }
  }

  fn generate_random_position(
    &self,
    marker_width: usize,
    marker_height: usize,
    grid_width: usize,
    grid_height: usize,
    rng: &mut impl rand::Rng,
  ) -> (usize, usize) {
    let max_x = grid_width.saturating_sub(marker_width);
    let max_y = grid_height.saturating_sub(marker_height);

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
          |canvas: &mut Canvas<CanvasEditor>| {
            let editor = canvas.state_mut();
            editor.arpeggiator_mode = is_arp;
            editor.marker_ui.arpeggiator_mode = is_arp;
          },
        );

        siv.call_on_name(consts::osc_status_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_random_mode(&self, cb_sink: cursive::CbSink) {
    let is_rand = !self.random_mode.load(Ordering::Relaxed);
    self.random_mode.store(is_rand, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<CanvasEditor>| {
            let editor = canvas.state_mut();
            editor.random_mode = is_rand;
            editor.marker_ui.random_mode = is_rand;
          },
        );

        siv.call_on_name(consts::osc_status_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
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
  fn check_operators(&self, abs_x: usize) {
    if !self.accumulation_mode.load(Ordering::Relaxed) {
      return;
    }

    if abs_x.is_multiple_of(consts::QUEUE_OP_SPACING) {
      let position_index = abs_x / consts::QUEUE_OP_SPACING;
      self.execute_queue_operator(position_index);
    }

    if abs_x.is_multiple_of(consts::EVENT_OP_SPACING) {
      let position_index = abs_x / consts::EVENT_OP_SPACING;
      self.execute_event_operator(position_index);
    }
  }

  fn execute_queue_operator(&self, position_index: usize) {
    let operator_index = position_index % QUEUE_OPERATORS.len();
    let operator = QUEUE_OPERATORS[operator_index];

    match operator {
      QueueOperator::Push => self.handle_push(),
      QueueOperator::Swap => self.handle_swap(),
      QueueOperator::Pop => self.handle_pop(),
      QueueOperator::Duplicate => self.handle_duplicate(),
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
    let event_op = if !event_queue.is_empty() {
      event_queue.pop_front()
    } else {
      None
    };

    if let Some(event_op) = event_op {
      drop(event_queue);

      // Push the event to the main queue
      let mut queue = self.operator_queue.lock().unwrap();
      queue.push_back(QueueItem::Event(event_op));
      drop(queue);

      self.update_queue_display();
    } else {
      drop(event_queue);

      // No events in queue, push current marker position
      let marker_pos = self.pos.lock().unwrap();
      let push_pos = (marker_pos.x, marker_pos.y);
      drop(marker_pos);

      let mut pushed = self.pushed_positions.lock().unwrap();
      if let Entry::Vacant(e) = pushed.entry(push_pos) {
        e.insert(true);
        drop(pushed);

        let mut queue = self.operator_queue.lock().unwrap();
        queue.push_back(QueueItem::Position(push_pos.0, push_pos.1));
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
    if let Some(item) = queue.pop_back() {
      drop(queue);

      // Only remove from pushed_positions if it was a position
      if let QueueItem::Position(x, y) = item {
        let mut pushed = self.pushed_positions.lock().unwrap();
        pushed.remove(&(x, y));
        drop(pushed);
      }

      self.update_queue_display();
    }
  }

  fn handle_duplicate(&self) {
    let mut queue = self.operator_queue.lock().unwrap();
    if let Some(item) = queue.back().cloned() {
      queue.push_back(item);
    }
    drop(queue);

    self.update_queue_display();
  }

  fn update_queue_display(&self) {
    let queue = self.operator_queue.lock().unwrap();
    let event_queue = self.event_queue.lock().unwrap();

    // Format operator queue with Display trait for clean output
    let queue_display = if queue.is_empty() {
      "[]".to_string()
    } else {
      let items: Vec<String> = queue.iter().map(|item| format!("{}", item)).collect();
      format!("[{}]", items.join(", "))
    };

    // Format event queue with Display trait
    let event_queue_display = if event_queue.is_empty() {
      "[]".to_string()
    } else {
      let items: Vec<String> = event_queue.iter().map(|op| format!("{}", op)).collect();
      format!("[{}]", items.join(", "))
    };

    drop(queue);
    drop(event_queue);

    let mut ui_queue = self.ui_update_queue.lock().unwrap();
    ui_queue.push_back(UIUpdate::OpQueueDisplay(queue_display));
    ui_queue.push_back(UIUpdate::EvQueueDisplay(event_queue_display));
  }

  fn handle_r(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.push_back(EventOperator::R);
    drop(event_queue);

    self.update_queue_display();
  }

  fn handle_c(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.push_back(EventOperator::C);
    drop(event_queue);

    self.update_queue_display();
  }

  fn handle_x(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.push_back(EventOperator::X);
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
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.marker_ui.marker_pos = pos;
                    editor.marker_ui.marker_area = area;
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
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.marker_ui.marker_pos = pos;
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
            let marker_area = *area;

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
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();

                    editor.marker_ui.marker_area = marker_area;
                  },
                );
              }))
              .unwrap();
          }
          Message::SetActivePos(tick, cb_sink) => {
            // #[cfg(debug_assertions)]
            // let start = Instant::now();

            self.set_actived_pos(tick);

            let active_pos_mutex = self.actived_pos.lock().unwrap();
            let mut active_pos = *active_pos_mutex;
            drop(active_pos_mutex);

            let (abs_x, abs_y, curr_running_marker) = self.calculate_absolute_position(active_pos);

            let (note_position, scale_mode) =
              self.determine_note_position_and_scale(active_pos, abs_x, abs_y);

            let matched =
              self.trigger_midi_if_matched(curr_running_marker, note_position, scale_mode);

            if matched {
              if let Some(new_active_pos) = self.handle_accumulation_mode(abs_x, &cb_sink) {
                active_pos = new_active_pos;
              }
            }

            self.update_active_pos_ui(active_pos, &cb_sink);

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
            let marker_area = *area;
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
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.marker_ui.marker_area = marker_area;
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
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.marker_ui.text_matcher = mm;
                    editor.marker_ui.regex_indexes = regex_indexes_cloned;
                  },
                );
              }))
              .unwrap();
          }
          Message::SetGridSize(width, height) => {
            self.grid_width.store(width, Ordering::Relaxed);
            self.grid_height.store(height, Ordering::Relaxed);
          }
          Message::SetScaleModeLeft(scale_mode) => {
            let mut mode = self.scale_mode_left.lock().unwrap();
            *mode = scale_mode;
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

                if !is_enabled {
                  siv.call_on_name(consts::op_queue_status_unit_view, |view: &mut TextView| {
                    view.set_content("[]");
                  });
                }

                siv.call_on_name(consts::osc_status_unit_view, |view: &mut TextView| {
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
                  view.set_content(utils::build_ratio_status_str(new_ratio, ""));
                });
              }))
              .unwrap();
          }
          Message::ToggleReverseMode(cb_sink) => {
            self.toggle_reverse_mode(cb_sink);
          }
          Message::ToggleArpeggiatorMode(cb_sink) => {
            self.toggle_arpeggiator_mode(cb_sink);
          }
          Message::ToggleRandomMode(cb_sink) => {
            self.toggle_random_mode(cb_sink);
          }
        }
      }
    });

    tx
  }
}
