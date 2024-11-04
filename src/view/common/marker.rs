use std::{
  collections::BTreeSet,
  sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
  },
  usize,
};

use cursive::{
  event::{Callback, EventResult},
  theme::Style,
  utils::span::SpannedString,
  views::TextView,
  Printer, Rect, Vec2, XY,
};

use crate::core::{config, midi, utils};

use super::canvas_editor::CanvasEditor;

#[derive(Clone, Debug)]
pub enum Direction {
  Up,
  Down,
  Left,
  Right,
  Idle,
}

#[derive(Clone, Debug)]
pub enum Message {
  Move(Direction, XY<usize>),
  SetCurrentPos(XY<usize>, XY<usize>),
  UpdateInfoStatusView(),
  SetGridArea(XY<usize>),
}

#[derive(Debug)]
pub struct Marker {
  pos: Arc<Mutex<Vec2>>,
  area: Arc<Mutex<Rect>>,
  drag_start_x: AtomicUsize,
  drag_start_y: AtomicUsize,
  actived_pos: Vec2,
  midi_msg_config_list: Arc<Mutex<Vec<midi::MidiMsg>>>,
  regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  cb_sink: cursive::CbSink,
}

impl Direction {
  pub fn get_direction(&self) -> (i32, i32) {
    match self {
      Direction::Right => (1, 0),
      Direction::Up => (0, -1),
      Direction::Left => (-1, 0),
      Direction::Down => (0, 1),
      Direction::Idle => (0, 0),
    }
  }
}

impl Marker {
  pub fn new(cb_sink: cursive::CbSink) -> Self {
    let (tx, rx) = channel();

    Marker {
      pos: Arc::new(Mutex::new(Vec2::zero())),
      area: Arc::new(Mutex::new(Rect::from_point(Vec2::zero()))),
      drag_start_y: AtomicUsize::new(0),
      drag_start_x: AtomicUsize::new(0),
      actived_pos: Vec2::zero(),
      midi_msg_config_list: Arc::new(Mutex::new(Vec::new())),
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      tx,
      rx,
      cb_sink,
    }
  }

  pub fn run(self) {
    for control_message in &self.rx {
      match control_message {
        Message::Move(direction, canvas_size) => {
          self.set_move(direction, canvas_size);
        }
        Message::SetCurrentPos(position, offset) => {
          self.set_current_pos(position, offset);
        }
        Message::UpdateInfoStatusView() => {
          let pos = self.pos.lock().unwrap();
          let area = self.area.lock().unwrap();
          let pos_x = pos.x;
          let pos_y = pos.y;
          let w = area.width();
          let h = area.height();

          self
            .cb_sink
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
        Message::SetGridArea(current_pos) => {
          self.set_grid_area(current_pos);

          let area = self.area.lock().unwrap();
          let w = area.width();
          let h = area.height();

          self
            .cb_sink
            .send(Box::new(move |siv| {
              siv.call_on_name(config::len_status_unit_view, move |view: &mut TextView| {
                view.set_content(utils::build_len_status_str((w, h)));
              });
            }))
            .unwrap();
        }
      }
    }
  }

  pub fn print(&self, printer: &Printer, editor: &CanvasEditor) {
    let mut pos = self.pos.lock().unwrap();
    let area = self.area.lock().unwrap();
    for x in 0..area.width() {
      for y in 0..area.height() {
        let offset_x = pos.x + x;
        let offset_y = pos.y + y;

        if self.is_head((offset_x, offset_y).into()) {
          printer.print_styled(
            (offset_x, offset_y),
            &SpannedString::styled('>', Style::highlight()),
          );
          continue;
        }

        // let curr_running_marker = offset_x + offset_y * editor.grid.width;
        // let (displayed_style, displayed_char) =
        //   if self.is_actived_position((offset_x, offset_y).into()) {
        //     if editor.text_matcher.is_some() {
        //       let hl: &HashMap<usize, Match> = editor.text_matcher.as_ref().unwrap();
        //       if hl.get(&curr_running_marker).is_some() {
        //         let _ = editor.midi_tx.send(midi::Message::TriggerWithRegexPos((
        //           curr_running_marker,
        //           self.regex_indexes.clone(),
        //         )));
        //         (Style::none(), '@')
        //       } else {
        //         (Style::none(), '>')
        //       }
        //     } else {
        //       (Style::none(), '>')
        //     }
        //   } else {
        //     let ch = if editor.text_matcher.is_some() {
        //       let hl = editor.text_matcher.as_ref().unwrap();
        //       let hl_item = hl.get(&curr_running_marker);
        //       if hl_item.is_some() {
        //         let mut regex_indexes = self.regex_indexes.lock().unwrap();
        //         regex_indexes.insert(curr_running_marker);
        //         regex_indexes.retain(|re_idx: &usize| {
        //           let dd = editor.index_to_xy(re_idx);
        //           dd.fits(self.pos) && dd.fits_in(self.pos + self.area.size())
        //         });
        //         '*'
        //       } else {
        //         editor.get(offset_x, offset_y)
        //       }
        //     } else {
        //       editor.get(offset_x, offset_y)
        //     };

        //     (
        //       Style::highlight(),
        //       ch.display_char((offset_x, offset_y).into()),
        //     )
        //   };

        // printer.print_styled(
        //   (offset_x, offset_y),
        //   &SpannedString::styled(displayed_char, displayed_style),
        // );
      }
    }
  }

  fn is_head(&self, curr_pos: Vec2) -> bool {
    let pos = self.pos.lock().unwrap();
    pos.eq(&curr_pos)
  }
  fn is_actived_position(&self, curr_pos: Vec2) -> bool {
    let pos = self.pos.lock().unwrap();
    pos.saturating_add(self.actived_pos).eq(&curr_pos)
  }

  fn set_move(&self, direction: Direction, canvas_size: Vec2) {
    let mut pos = self.pos.lock().unwrap();
    let area = self.area.lock().unwrap();
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

    let pos_x = pos.x;
    let pos_y = pos.y;

    self
      .cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(config::pos_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_pos_status_str((pos_x, pos_y).into()));
        });
      }))
      .unwrap();
  }

  pub fn set_current_pos(&self, pos: XY<usize>, offset: XY<usize>) {
    let mut mutex_pos = self.pos.lock().unwrap();
    let pos_x = pos.x.abs_diff(1);
    let pos_y = pos.y.abs_diff(offset.y);
    *mutex_pos = (pos_x, pos_y).into();
  }

  pub fn set_grid_area(&self, current_pos: XY<usize>) {
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

    self.drag_start_x.store(new_x, Ordering::SeqCst);
    self.drag_start_y.store(new_y, Ordering::SeqCst);
  }

  pub fn set_actived_pos(&mut self, pos: usize) {
    let area = self.area.lock().unwrap();

    self.actived_pos.x = pos % area.width();

    if self.actived_pos.x == 0 {
      self.actived_pos.y += 1;
      self.actived_pos.y %= area.height();
    }
  }

  pub fn scale(&mut self, (w, h): (i32, i32)) {
    let pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();

    *area = Rect::from_size(
      *pos,
      ((area.width() as i32) + w, (area.height() as i32) - h),
    );
  }

  pub fn get_area_size(&self) -> (usize, usize) {
    let area = self.area.lock().unwrap();
    (area.width(), area.height())
  }
}
