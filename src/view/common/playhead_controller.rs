use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use cursive::{views::Canvas, XY};

use crate::core::{consts, midi, regex::Match};

use super::grid_editor::GridEditor;
use super::playhead;
use super::playhead::PlayheadArea;

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
  SetGridSize(usize, usize),
  SetScaleModeLeft(crate::core::scale::ScaleMode),
  SetScaleModeTop(crate::core::scale::ScaleMode),
  ToggleAccumulationMode(),
  ToggleReverseMode(),
  ToggleArpeggiatorMode(),
  ToggleRandomMode(),
  ToggleEventOperatorMode(),
  ToggleDrainQueueMode(),
  ToggleSweepMode(),
  SetTempo(usize),
  SetRatio((usize, usize)),
}

pub struct Playhead {
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

impl Playhead {
  pub fn new(cb_sink: cursive::CbSink, midi_tx: Sender<midi::Message>) -> Self {
    let (tx, rx) = channel();

    Playhead {
      tx,
      rx,
      cb_sink,
      midi_tx,
    }
  }

  pub fn run(self) {
    let playhead_area = Arc::new(PlayheadArea::new(self.midi_tx.clone()));

    // Spawn UI batch processor thread (60 FPS)
    playhead::PlayheadArea::spawn_ui_processor(
      Arc::clone(&playhead_area.ui_update_queue),
      self.cb_sink.clone(),
    );

    let playhead_area_tx = playhead_area.run();

    thread::spawn(move || {
      for control_message in &self.rx {
        match control_message {
          Message::Move(direction, canvas_size) => {
            playhead_area_tx
              .send(playhead::Message::Move(
                direction,
                canvas_size,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetCurrentPos(position, offset) => {
            playhead_area_tx
              .send(playhead::Message::SetCurrentPos(
                position,
                offset,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::UpdateInfoStatusView() => {
            playhead_area_tx
              .send(playhead::Message::UpdateInfoStatusView(
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetGridArea(current_pos) => {
            playhead_area_tx
              .send(playhead::Message::SetGridArea(
                current_pos,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetActivePos(tick) => {
            playhead_area_tx
              .send(playhead::Message::SetActivePos(tick, self.cb_sink.clone()))
              .unwrap();
          }
          Message::Scale(dir) => {
            playhead_area_tx
              .send(playhead::Message::Scale(dir, self.cb_sink.clone()))
              .unwrap();
          }
          Message::SetMatcher(matcher) => {
            playhead_area_tx
              .send(playhead::Message::SetMatcher(matcher, self.cb_sink.clone()))
              .unwrap();
          }
          Message::SetGridSize(width, height) => {
            playhead_area_tx
              .send(playhead::Message::SetGridSize(width, height))
              .unwrap();
          }
          Message::SetScaleModeLeft(scale_mode) => {
            let cb_sink = self.cb_sink.clone();

            playhead_area_tx
              .send(playhead::Message::SetScaleModeLeft(scale_mode))
              .unwrap();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<GridEditor>| {
                    let editor = canvas.state_mut();
                    editor.scale_mode_left = scale_mode;
                  },
                );
              }))
              .unwrap();
          }
          Message::SetScaleModeTop(scale_mode) => {
            let cb_sink = self.cb_sink.clone();

            playhead_area_tx
              .send(playhead::Message::SetScaleModeTop(scale_mode))
              .unwrap();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<GridEditor>| {
                    let editor = canvas.state_mut();
                    editor.scale_mode_top = scale_mode;
                  },
                );
              }))
              .unwrap();
          }
          Message::ToggleAccumulationMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::ToggleAccumulationMode(cb_sink))
              .unwrap();
          }
          Message::ToggleReverseMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::ToggleReverseMode(cb_sink))
              .unwrap();
          }
          Message::SetTempo(bpm) => {
            playhead_area_tx
              .send(playhead::Message::SetTempo(bpm))
              .unwrap();

            self.midi_tx.send(midi::Message::SetTempo(bpm)).unwrap();
          }
          Message::SetRatio(ratio) => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::SetRatio(ratio, cb_sink))
              .unwrap();
          }
          Message::ToggleArpeggiatorMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::ToggleArpeggiatorMode(cb_sink))
              .unwrap();
          }
          Message::ToggleRandomMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::ToggleRandomMode(cb_sink))
              .unwrap();
          }
          Message::ToggleEventOperatorMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::ToggleEventOperatorMode(cb_sink))
              .unwrap();
          }
          Message::ToggleDrainQueueMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::ToggleDrainQueueMode(cb_sink))
              .unwrap();
          }
          Message::ToggleSweepMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead::Message::ToggleSweepMode(cb_sink))
              .unwrap();
          }
        }
      }
    });
  }
}
