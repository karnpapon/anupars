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
    }
  }

  fn set_move(&self, direction: Direction, canvas_size: Vec2) {
    let mut pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();
    let next_pos = pos.saturating_add(direction.get_direction());
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

    actived_pos.x = pos % area.width();

    if actived_pos.x == 0 {
      actived_pos.y += 1;
      actived_pos.y %= area.height();
    }
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
                // Trigger MIDI with position-based note mapping
                let _ = self.midi_tx.send(midi::Message::TriggerWithPosition((
                  curr_running_marker,
                  note_position, // Note position based on movement direction
                  grid_width,
                  grid_height,
                  scale_mode,
                )));

                // Handle accumulation mode
                if *self.accumulation_mode.lock().unwrap() {
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

                    // Ensure random position keeps marker within grid bounds
                    let max_x = grid_width.saturating_sub(marker_width);
                    let max_y = grid_height.saturating_sub(marker_height);

                    if max_x > 0 && max_y > 0 {
                      let new_x = rng.gen_range(0..=max_x);
                      let new_y = rng.gen_range(0..=max_y);

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
                    }
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

            // Reset counter when toggling mode
            let mut counter = self.accumulation_counter.lock().unwrap();
            *counter = 0;
            drop(counter);
            drop(mode);

            // Update UI to clear accumulation display
            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
                  view.set_content("-");
                });
              }))
              .unwrap();
          }
        }
      }
    });

    tx
  }
}
