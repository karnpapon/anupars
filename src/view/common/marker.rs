use std::{
  sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
  },
  thread, usize,
};

use cursive::XY;

use super::marker_area::{self, MarkerArea};

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
  SetActivePos(usize),
  Scale((i32, i32)),
}

pub struct Marker {
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

    Marker { tx, rx, cb_sink }
  }

  pub fn run(self) {
    let marker_area = Arc::new(MarkerArea::new());
    let marker_area_tx = marker_area.run();

    thread::spawn(move || {
      for control_message in &self.rx {
        match control_message {
          Message::Move(direction, canvas_size) => {
            marker_area_tx
              .send(marker_area::Message::Move(
                direction,
                canvas_size,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetCurrentPos(position, offset) => {
            marker_area_tx
              .send(marker_area::Message::SetCurrentPos(
                position,
                offset,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::UpdateInfoStatusView() => {
            marker_area_tx
              .send(marker_area::Message::UpdateInfoStatusView(
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetGridArea(current_pos) => {
            marker_area_tx
              .send(marker_area::Message::SetGridArea(
                current_pos,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetActivePos(tick) => {
            marker_area_tx
              .send(marker_area::Message::SetActivePos(tick))
              .unwrap();
          }
          Message::Scale(dir) => {
            marker_area_tx
              .send(marker_area::Message::Scale(dir, self.cb_sink.clone()))
              .unwrap();
          }
        }
      }
    });
  }
}
