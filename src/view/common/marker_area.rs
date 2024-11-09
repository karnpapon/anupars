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

use crate::core::{config, midi, rect::Rect, regex::Match, utils};

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
}

pub struct MarkerArea {
  pos: Arc<Mutex<Vec2>>,
  area: Arc<Mutex<Rect>>,
  drag_start_x: AtomicUsize,
  drag_start_y: AtomicUsize,
  actived_pos: Arc<Mutex<Vec2>>,
  // midi_msg_config_list: Arc<Mutex<Vec<midi::MidiMsg>>>,
  regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,
}

impl MarkerArea {
  pub fn new() -> Self {
    MarkerArea {
      pos: Arc::new(Mutex::new(Vec2::zero())),
      area: Arc::new(Mutex::new(Rect::from_point(Vec2::zero()))),
      drag_start_y: AtomicUsize::new(0),
      drag_start_x: AtomicUsize::new(0),
      actived_pos: Arc::new(Mutex::new(Vec2::zero())),
      // midi_msg_config_list: Arc::new(Mutex::new(Vec::new())),
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      text_matcher: Arc::new(Mutex::new(None)),
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
            let pos_mutex = self.pos.lock().unwrap();
            let pos = *pos_mutex;

            let area_mutex = self.area.lock().unwrap();
            let area = *area_mutex;

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(config::pos_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_pos_status_str(pos));
                });

                siv.call_on_name(
                  config::canvas_editor_section_view,
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
            let mutex_pos = self.pos.lock().unwrap();
            let pos = *mutex_pos;
            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  config::canvas_editor_section_view,
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
                siv.call_on_name(config::pos_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_pos_status_str((pos_x, pos_y).into()))
                });

                siv.call_on_name(config::len_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_len_status_str((w, h)));
                });
              }))
              .unwrap();
          }
          Message::SetGridArea(current_pos, cb_sink) => {
            self.set_grid_area(current_pos);

            let area = self.area.lock().unwrap();
            let w = area.width();
            let h = area.height();
            let marker_area = *area;

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(config::len_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_len_status_str((w, h)));
                });

                siv.call_on_name(
                  config::canvas_editor_section_view,
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
            let active_pos = *active_pos_mutex;

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  config::canvas_editor_section_view,
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
            let area = self.area.lock().unwrap();
            let marker_area = *area;
            let area_size = area.size();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(config::len_status_unit_view, move |view: &mut TextView| {
                  view.set_content(utils::build_len_status_str((area_size.x, area_size.y)));
                });

                siv.call_on_name(
                  config::canvas_editor_section_view,
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
                  config::canvas_editor_section_view,
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.marker_ui.text_matcher = mm;
                    editor.marker_ui.regex_indexes = regex_indexes_cloned;
                  },
                );
              }))
              .unwrap();
          }
        }
      }
    });

    tx
  }
}
