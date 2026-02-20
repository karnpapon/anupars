use std::{
  collections::{BTreeSet, HashMap},
  sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
  },
  thread, usize,
};

use cursive::{views::Canvas, XY};

use crate::core::{consts, midi, regex::Match};

use super::{
  canvas_editor::CanvasEditor,
  marker_area::{self, MarkerArea},
};

#[derive(Clone, Debug, PartialEq)]
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
  SetMatcher(Option<HashMap<usize, Match>>),
  TriggerWithRegexPos((usize, Arc<Mutex<BTreeSet<usize>>>)),
  SetGridSize(usize, usize),
  SetScaleModeLeft(crate::core::scale::ScaleMode),
  SetScaleModeTop(crate::core::scale::ScaleMode),
  ToggleAccumulationMode(),
  ToggleReverseMode(),
  SetTempo(usize),
}

pub struct Marker {
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  cb_sink: cursive::CbSink,
  midi_tx: Sender<midi::Message>,
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
  pub fn new(cb_sink: cursive::CbSink, midi_tx: Sender<midi::Message>) -> Self {
    let (tx, rx) = channel();

    Marker {
      tx,
      rx,
      cb_sink,
      midi_tx,
    }
  }

  pub fn run(self) {
    let marker_area = Arc::new(MarkerArea::new(self.midi_tx.clone()));
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
              .send(marker_area::Message::SetActivePos(
                tick,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::Scale(dir) => {
            marker_area_tx
              .send(marker_area::Message::Scale(dir, self.cb_sink.clone()))
              .unwrap();
          }
          Message::SetMatcher(matcher) => {
            marker_area_tx
              .send(marker_area::Message::SetMatcher(
                matcher,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::TriggerWithRegexPos(msg) => {
            self
              .midi_tx
              .send(midi::Message::TriggerWithRegexPos(msg))
              .unwrap();
          }
          Message::SetGridSize(width, height) => {
            marker_area_tx
              .send(marker_area::Message::SetGridSize(width, height))
              .unwrap();
          }
          Message::SetScaleModeLeft(scale_mode) => {
            let cb_sink = self.cb_sink.clone();

            marker_area_tx
              .send(marker_area::Message::SetScaleModeLeft(scale_mode))
              .unwrap();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.scale_mode_left = scale_mode;
                  },
                );
              }))
              .unwrap();
          }
          Message::SetScaleModeTop(scale_mode) => {
            let cb_sink = self.cb_sink.clone();

            marker_area_tx
              .send(marker_area::Message::SetScaleModeTop(scale_mode))
              .unwrap();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<CanvasEditor>| {
                    let editor = canvas.state_mut();
                    editor.scale_mode_top = scale_mode;
                  },
                );
              }))
              .unwrap();
          }
          Message::ToggleAccumulationMode() => {
            let cb_sink = self.cb_sink.clone();
            marker_area_tx
              .send(marker_area::Message::ToggleAccumulationMode(cb_sink))
              .unwrap();
          }
          Message::ToggleReverseMode() => {
            let cb_sink = self.cb_sink.clone();
            marker_area_tx
              .send(marker_area::Message::ToggleReverseMode(cb_sink))
              .unwrap();
          }
          Message::SetTempo(bpm) => {
            marker_area_tx
              .send(marker_area::Message::SetTempo(bpm))
              .unwrap();

            self.midi_tx.send(midi::Message::SetTempo(bpm)).unwrap();
          }
        }
      }
    });
  }
}
