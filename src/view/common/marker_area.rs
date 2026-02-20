use std::collections::hash_map::DefaultHasher;
use std::collections::hash_map::Entry;
use std::hash::{Hash, Hasher};
use std::{
  collections::{BTreeSet, HashMap},
  sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{channel, Sender},
    Arc, Mutex,
  },
  thread, usize,
};

use cursive::{
  views::{Canvas, TextView},
  Vec2, XY,
};

use crate::core::{consts, midi, rect::Rect, regex::Match, utils};

use super::{canvas_editor::CanvasEditor, marker::Direction};

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
  SetTempo(usize),
  SetRatio((i64, usize), cursive::CbSink),
  ToggleReverseMode(cursive::CbSink),
  ToggleArpeggiatorMode(cursive::CbSink),
  ToggleRandomMode(cursive::CbSink),
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
  grid_width: Arc<Mutex<usize>>,
  grid_height: Arc<Mutex<usize>>,
  prev_active_pos: Arc<Mutex<Vec2>>,
  scale_mode_left: Arc<Mutex<crate::core::scale::ScaleMode>>,
  scale_mode_top: Arc<Mutex<crate::core::scale::ScaleMode>>,
  accumulation_counter: Arc<Mutex<usize>>,
  accumulation_mode: Arc<Mutex<bool>>,
  tempo: Arc<Mutex<usize>>,
  ratio: Arc<Mutex<(i64, usize)>>,
  position_stack: Arc<Mutex<Vec<(usize, usize)>>>,
  pushed_positions: Arc<Mutex<HashMap<(usize, usize), bool>>>,
  reverse_mode: Arc<Mutex<bool>>,
  arpeggiator_mode: Arc<Mutex<bool>>,
  random_mode: Arc<Mutex<bool>>,
}

impl MarkerArea {
  pub fn new(midi_tx: Sender<midi::Message>) -> Self {
    MarkerArea {
      pos: Arc::new(Mutex::new(Vec2::zero())),
      area: Arc::new(Mutex::new(Rect::from_point(Vec2::zero()))),
      drag_start_y: AtomicUsize::new(0),
      drag_start_x: AtomicUsize::new(0),
      actived_pos: Arc::new(Mutex::new(Vec2::zero())),
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      text_matcher: Arc::new(Mutex::new(None)),
      midi_tx,
      grid_width: Arc::new(Mutex::new(0)),
      grid_height: Arc::new(Mutex::new(0)),
      prev_active_pos: Arc::new(Mutex::new(Vec2::zero())),
      scale_mode_left: Arc::new(Mutex::new(crate::core::scale::ScaleMode::default())),
      scale_mode_top: Arc::new(Mutex::new(crate::core::scale::ScaleMode::default())),
      accumulation_counter: Arc::new(Mutex::new(0)),
      accumulation_mode: Arc::new(Mutex::new(false)),
      tempo: Arc::new(Mutex::new(120)),
      ratio: Arc::new(Mutex::new((1, 16))),
      position_stack: Arc::new(Mutex::new(Vec::new())),
      pushed_positions: Arc::new(Mutex::new(HashMap::new())),
      reverse_mode: Arc::new(Mutex::new(false)),
      arpeggiator_mode: Arc::new(Mutex::new(false)),
      random_mode: Arc::new(Mutex::new(false)),
    }
  }

  fn build_mode_status_string(&self) -> String {
    let reverse = *self.reverse_mode.lock().unwrap();
    let arpeggiator = *self.arpeggiator_mode.lock().unwrap();
    let accumulation = *self.accumulation_mode.lock().unwrap();
    let random = *self.random_mode.lock().unwrap();

    format!(
      "{}{}{}{}",
      if reverse { "R" } else { "r" },
      if arpeggiator { "A" } else { "a" },
      if accumulation { "U" } else { "u" },
      if random { "D" } else { "d" }
    )
  }

  pub fn toggle_reverse_mode(&self, cb_sink: cursive::CbSink) {
    let mut reverse = self.reverse_mode.lock().unwrap();
    *reverse = !*reverse;
    let is_reversed = *reverse;
    drop(reverse);

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

    self.drag_start_x.store(top_left.x, Ordering::SeqCst);
    self.drag_start_y.store(top_left.y, Ordering::SeqCst);
  }

  pub fn set_actived_pos(&self, pos: usize) {
    let area = self.area.lock().unwrap();
    let mut actived_pos = self.actived_pos.lock().unwrap();
    let reverse = *self.reverse_mode.lock().unwrap();
    let arpeggiator = *self.arpeggiator_mode.lock().unwrap();
    let random = *self.random_mode.lock().unwrap();
    let marker_w = area.width();
    let marker_h = area.height();
    let marker_x = area.left();
    let marker_y = area.top();
    let canvas_w = *self.grid_width.lock().unwrap();

    // Adjust pos based on ratio: higher denominators = faster, lower = slower
    // Clock ticks at 4 per beat (sixteenth note resolution)
    // 1/16 = every tick, 1/8 = every 2 ticks, 1/4 = every 4 ticks, etc.
    let ratio = *self.ratio.lock().unwrap();
    let divisor = std::cmp::max(1, 16 / ratio.1);
    let adjusted_pos = pos / divisor;

    if arpeggiator {
      let regex_indexes = self.regex_indexes.lock().unwrap();
      let mut matches: Vec<(usize, usize)> = regex_indexes
        .iter()
        .filter_map(|&idx| {
          let x = idx % canvas_w;
          let y = idx / canvas_w;
          // Only include matches inside the marker area
          if x >= marker_x && x < marker_x + marker_w && y >= marker_y && y < marker_y + marker_h {
            Some((x - marker_x, y - marker_y))
          } else {
            None
          }
        })
        .collect();
      matches.sort_by_key(|&(x, y)| (y, x));
      if reverse {
        matches.reverse();
      }
      if !matches.is_empty() {
        let step = if random {
          let mut hasher = DefaultHasher::new();
          adjusted_pos.hash(&mut hasher);
          (hasher.finish() as usize) % matches.len()
        } else {
          adjusted_pos % matches.len()
        };
        let (x, y) = matches[step];
        actived_pos.x = x;
        actived_pos.y = y;
      } else {
        // No matches, fallback to normal running
        if random {
          let mut hasher = DefaultHasher::new();
          adjusted_pos.hash(&mut hasher);
          let hash = hasher.finish() as usize;
          actived_pos.x = hash % marker_w;
          actived_pos.y = (hash / marker_w) % marker_h;
        } else if !reverse {
          actived_pos.x = adjusted_pos % marker_w;
          if actived_pos.x == 0 {
            actived_pos.y += 1;
            actived_pos.y %= marker_h;
          }
        } else {
          actived_pos.x = marker_w - 1 - (adjusted_pos % marker_w);
          if actived_pos.x == marker_w - 1 {
            if actived_pos.y == 0 {
              actived_pos.y = marker_h - 1;
            } else {
              actived_pos.y -= 1;
            }
          }
        }
      }
    } else {
      // Normal running
      if random {
        let mut hasher = DefaultHasher::new();
        adjusted_pos.hash(&mut hasher);
        let hash = hasher.finish() as usize;
        actived_pos.x = hash % marker_w;
        actived_pos.y = (hash / marker_w) % marker_h;
      } else if !reverse {
        actived_pos.x = adjusted_pos % marker_w;
        if actived_pos.x == 0 {
          actived_pos.y += 1;
          actived_pos.y %= marker_h;
        }
      } else {
        actived_pos.x = marker_w - 1 - (adjusted_pos % marker_w);
        if actived_pos.x == marker_w - 1 {
          if actived_pos.y == 0 {
            actived_pos.y = marker_h - 1;
          } else {
            actived_pos.y -= 1;
          }
        }
      }
    }
  }

  pub fn toggle_arpeggiator_mode(&self, cb_sink: cursive::CbSink) {
    let mut arp = self.arpeggiator_mode.lock().unwrap();
    *arp = !*arp;
    let is_arp = *arp;
    drop(arp);

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
    let mut rand = self.random_mode.lock().unwrap();
    *rand = !*rand;
    let is_rand = *rand;
    drop(rand);

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

  fn check_stack_operator(&self, abs_x: usize, cb_sink: &cursive::CbSink) {
    // Stack operators: P (Push), S (Swap), O (pOp) with spacing of 11 (10 spaces + 1 char)
    // Only active when accumulation mode is enabled
    let is_accumulation_mode = *self.accumulation_mode.lock().unwrap();
    if !is_accumulation_mode {
      return;
    }

    let spacing = 11;
    let operators = ['P', 'S', 'O'];

    // Check if abs_x aligns with an operator position
    if abs_x.is_multiple_of(spacing) {
      let operator_index = (abs_x / spacing) % operators.len();
      let operator = operators[operator_index];

      match operator {
        'P' => {
          // Push: Add marker area starting position to stack
          let marker_pos = self.pos.lock().unwrap();
          let push_pos = (marker_pos.x, marker_pos.y);
          drop(marker_pos);

          let mut pushed = self.pushed_positions.lock().unwrap();
          if let Entry::Vacant(e) = pushed.entry(push_pos) {
            e.insert(true);
            drop(pushed);

            let mut stack = self.position_stack.lock().unwrap();
            stack.push(push_pos);
            let stack_display = format!("{:?}", *stack);
            drop(stack);

            // Update UI
            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(consts::stack_status_unit_view, |view: &mut TextView| {
                  view.set_content(stack_display);
                });
              }))
              .unwrap();
          }
        }
        'S' => {
          // Swap: Swap top two positions in stack
          let mut stack = self.position_stack.lock().unwrap();
          let len = stack.len();
          if len >= 2 {
            stack.swap(len - 1, len - 2);
          }
          let stack_display = format!("{:?}", *stack);
          drop(stack);

          // Update UI
          cb_sink
            .send(Box::new(move |siv| {
              siv.call_on_name(consts::stack_status_unit_view, |view: &mut TextView| {
                view.set_content(stack_display);
              });
            }))
            .unwrap();
        }
        'O' => {
          // pOp: Remove last position from stack
          let mut stack = self.position_stack.lock().unwrap();
          if let Some(pos) = stack.pop() {
            let stack_display = format!("{:?}", *stack);
            drop(stack);

            let mut pushed = self.pushed_positions.lock().unwrap();
            pushed.remove(&pos);
            drop(pushed);

            // Update UI
            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(consts::stack_status_unit_view, |view: &mut TextView| {
                  view.set_content(stack_display);
                });
              }))
              .unwrap();
          }
        }
        _ => {}
      }
    }
  }

  fn is_head(&self, curr_pos: Vec2) -> bool {
    let pos = self.pos.lock().unwrap();
    pos.eq(&curr_pos)
  }

  fn is_actived_position(&self, curr_pos: Vec2) -> bool {
    let pos = self.pos.lock().unwrap();
    let actived_pos = self.actived_pos.lock().unwrap();
    pos.saturating_add(*actived_pos).eq(&curr_pos)
  }

  pub fn run(self: Arc<Self>) -> Sender<Message> {
    let (tx, rx) = channel();

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
            self.set_actived_pos(tick);

            let active_pos_mutex = self.actived_pos.lock().unwrap();
            let mut active_pos = *active_pos_mutex;
            drop(active_pos_mutex);

            // Calculate current running marker position
            let pos = self.pos.lock().unwrap();
            let grid_width = *self.grid_width.lock().unwrap();
            let grid_height = *self.grid_height.lock().unwrap();
            let abs_y = pos.y + active_pos.y;
            let abs_x = pos.x + active_pos.x;
            let curr_running_marker = (abs_y * grid_width) + abs_x;
            drop(pos);

            // Get previous active_pos and determine movement direction
            let mut prev_active = self.prev_active_pos.lock().unwrap();
            let prev_active_pos = *prev_active;

            // Calculate which axis of active_pos changed more (horizontal vs vertical movement)
            let x_diff = active_pos.x.abs_diff(prev_active_pos.x);
            let y_diff = active_pos.y.abs_diff(prev_active_pos.y);

            // Determine note position and scale mode based on movement direction
            let (note_position, scale_mode) = if x_diff > y_diff {
              // Horizontal movement (active_pos.x changed): use top keyboard mapping (x % grid_height)
              let pos = if grid_height > 0 {
                abs_x % grid_height
              } else {
                abs_y
              };
              let scale = *self.scale_mode_top.lock().unwrap();
              (pos, scale)
            } else {
              // Vertical movement (active_pos.y changed): use left keyboard mapping (y position directly)
              let scale = *self.scale_mode_left.lock().unwrap();
              (abs_y, scale)
            };

            // Store current active_pos for next comparison
            *prev_active = active_pos;
            drop(prev_active); // Release lock

            // Check if current position has a regex match and trigger with position-based note
            if let Some(matcher) = self.text_matcher.lock().unwrap().as_ref() {
              if matcher.get(&curr_running_marker).is_some() {
                // Get current tempo
                let current_tempo = *self.tempo.lock().unwrap();

                // Trigger MIDI with position-based note mapping
                let _ = self.midi_tx.send(midi::Message::TriggerWithPosition((
                  curr_running_marker,
                  note_position, // Note position based on movement direction
                  grid_width,
                  grid_height,
                  scale_mode,
                  current_tempo,
                )));

                // Handle accumulation mode
                if *self.accumulation_mode.lock().unwrap() {
                  // Check stack operator at current abs_x position (only when accumulation mode is on)
                  self.check_stack_operator(abs_x, &cb_sink);

                  let area = self.area.lock().unwrap();
                  let marker_area_size = area.width() * area.height();
                  drop(area);

                  let mut counter = self.accumulation_counter.lock().unwrap();
                  *counter += 1;
                  let current_count = *counter;

                  // When counter reaches marker area size, randomly jump to new position
                  if *counter >= marker_area_size {
                    *counter = 0; // Reset counter
                    drop(counter);

                    // Update UI to show reset counter
                    let cb_sink_update = cb_sink.clone();
                    cb_sink_update
                      .send(Box::new(move |siv| {
                        siv.call_on_name(
                          consts::input_status_unit_view,
                          move |view: &mut TextView| {
                            view.set_content(format!("acc: 0/{}", marker_area_size));
                          },
                        );
                      }))
                      .unwrap();

                    // Generate random position within canvas_editor bounds
                    use rand::Rng;
                    let mut rng = rand::thread_rng();

                    let area = self.area.lock().unwrap();
                    let marker_width = area.width();
                    let marker_height = area.height();
                    drop(area);

                    // Get current marker position to check against stack
                    let current_marker_pos = {
                      let pos = self.pos.lock().unwrap();
                      (pos.x, pos.y)
                    };

                    // Determine jump position: check stack first, otherwise random
                    let mut stack = self.position_stack.lock().unwrap();
                    let (new_x, new_y) = if let Some(&top_pos) = stack.last() {
                      // Check if stack position is the same as current position
                      if top_pos == current_marker_pos {
                        // Same position, jump randomly but keep position in stack for next time
                        drop(stack);

                        // Generate random position
                        let max_x = grid_width.saturating_sub(marker_width);
                        let max_y = grid_height.saturating_sub(marker_height);

                        if max_x > 0 && max_y > 0 {
                          (rng.gen_range(0..=max_x), rng.gen_range(0..=max_y))
                        } else {
                          (0, 0)
                        }
                      } else {
                        // Use position from stack (pop it since we're jumping to it)
                        let pos = stack.pop().unwrap();
                        let stack_display = format!("{:?}", *stack);
                        drop(stack);

                        // Remove from pushed_positions
                        let mut pushed = self.pushed_positions.lock().unwrap();
                        pushed.remove(&pos);
                        drop(pushed);

                        // Update stack UI
                        let cb_sink_stack = cb_sink.clone();
                        cb_sink_stack
                          .send(Box::new(move |siv| {
                            siv.call_on_name(
                              consts::stack_status_unit_view,
                              |view: &mut TextView| {
                                view.set_content(stack_display);
                              },
                            );
                          }))
                          .unwrap();

                        pos
                      }
                    } else {
                      drop(stack);

                      // No position in stack, jump randomly
                      let max_x = grid_width.saturating_sub(marker_width);
                      let max_y = grid_height.saturating_sub(marker_height);

                      if max_x > 0 && max_y > 0 {
                        (rng.gen_range(0..=max_x), rng.gen_range(0..=max_y))
                      } else {
                        (0, 0)
                      }
                    };

                    // Update marker position
                    let mut pos = self.pos.lock().unwrap();
                    pos.x = new_x;
                    pos.y = new_y;
                    let new_pos = *pos;
                    drop(pos);

                    // Update marker area
                    let mut area = self.area.lock().unwrap();
                    *area = Rect::from_size((new_x, new_y), (marker_width, marker_height));
                    let new_area = *area;
                    drop(area);

                    // Reset active position to start
                    let mut actived = self.actived_pos.lock().unwrap();
                    *actived = Vec2::zero();
                    active_pos = Vec2::zero(); // Update local variable for UI update
                    drop(actived);

                    // Update UI with new marker position and area immediately
                    let cb_sink_clone = cb_sink.clone();
                    cb_sink_clone
                      .send(Box::new(move |siv| {
                        siv.call_on_name(
                          consts::canvas_editor_section_view,
                          move |canvas: &mut Canvas<CanvasEditor>| {
                            let editor = canvas.state_mut();
                            editor.marker_ui.marker_pos = new_pos;
                            editor.marker_ui.marker_area = new_area;
                          },
                        );
                      }))
                      .unwrap();
                  } else {
                    // Update UI to show current counter progress
                    drop(counter);
                    let cb_sink_update = cb_sink.clone();
                    cb_sink_update
                      .send(Box::new(move |siv| {
                        siv.call_on_name(
                          consts::input_status_unit_view,
                          move |view: &mut TextView| {
                            view
                              .set_content(format!("acc: {}/{}", current_count, marker_area_size));
                          },
                        );
                      }))
                      .unwrap();
                  }
                }
              }
            }

            // Always update active_pos in UI (will be Vec2::zero() if we just jumped)
            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.marker_ui.actived_pos = active_pos;
                  },
                );
              }))
              .unwrap();
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
            let mut grid_width = self.grid_width.lock().unwrap();
            *grid_width = width;
            let mut grid_height = self.grid_height.lock().unwrap();
            *grid_height = height;
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
            let mut mode = self.accumulation_mode.lock().unwrap();
            *mode = !*mode;
            let is_enabled = *mode;

            // Reset counter when toggling mode
            let mut counter = self.accumulation_counter.lock().unwrap();
            *counter = 0;
            drop(counter);
            drop(mode);

            // Clear stack when disabling accumulation mode
            if !is_enabled {
              let mut stack = self.position_stack.lock().unwrap();
              stack.clear();
              drop(stack);

              let mut pushed = self.pushed_positions.lock().unwrap();
              pushed.clear();
              drop(pushed);
            }

            let mode_status = self.build_mode_status_string();

            // Update UI to clear accumulation display and stack display
            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
                  view.set_content("-");
                });

                if !is_enabled {
                  siv.call_on_name(consts::stack_status_unit_view, |view: &mut TextView| {
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
            let mut tempo = self.tempo.lock().unwrap();
            *tempo = bpm;
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
