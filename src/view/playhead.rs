use std::collections::HashMap;
#[cfg(feature = "symspell")]
use std::path::Path;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
#[cfg(feature = "symspell")]
use std::sync::Mutex;
use std::thread;

#[cfg(feature = "symspell")]
use symspell_rs::SymSpell;

use cursive::{views::Canvas, XY};

#[cfg(feature = "symspell")]
use crate::core::engine::regex;
use crate::core::tonal::scale;
use crate::core::{consts, engine::regex::Match, io::midi};

use super::grid_editor::GridEditor;
use super::playhead_handler;
use super::playhead_handler::PlayheadArea;
use crate::core::command::types;

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
  Leap(Direction, XY<usize>),
  SetCurrentPos(XY<usize>, XY<usize>),
  UpdateInfoStatusView(),
  SetGridArea(XY<usize>),
  SetActivePos(usize),
  Scale((i32, i32)),
  SetMatcher(Option<HashMap<usize, Match>>),
  SetGridSize(usize, usize),
  SetScaleModeLeft(scale::ScaleMode),
  SetScaleModeTop(scale::ScaleMode),
  SetScaleRootTop(scale::ScaleRoot),
  CycleScaleRootTop(types::Adjustment),
  CycleScaleMode(types::Adjustment),
  ToggleAccumulationMode(),
  ToggleForwardMode(),
  ToggleReverseMode(),
  ToggleArpeggiatorMode(),
  ToggleRandomMode(),
  TogglePendulumMode(),
  ToggleEventOperatorMode(),
  ToggleDrainQueueMode(),
  ToggleSweepMode(),
  ToggleDynLengthMode(),
  ToggleFreezeMode(),
  SetTempo(usize),
  SetRatio((usize, usize)),
  ClearQueue(),
  SetGridSplits(usize, usize),
  CycleTiltMode(),
}

pub struct Playhead {
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  cb_sink: cursive::CbSink,
  midi_tx: Sender<midi::Message>,
  #[cfg(feature = "symspell")]
  regex_tx: Sender<regex::Message>,
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
  pub fn new(
    cb_sink: cursive::CbSink,
    midi_tx: Sender<midi::Message>,
    #[cfg(feature = "symspell")] regex_tx: Sender<regex::Message>,
  ) -> Self {
    let (tx, rx) = channel();

    Playhead {
      tx,
      rx,
      cb_sink,
      midi_tx,
      #[cfg(feature = "symspell")]
      regex_tx,
    }
  }

  pub fn run(self) {
    let playhead_area = Arc::new(PlayheadArea::new(self.midi_tx.clone()));

    #[cfg(feature = "symspell")]
    // Initialise SymSpell once (max edit distance 2, prefix length 7)
    let mut symspell_inner = SymSpell::new(2, None, 7, 1);
    #[cfg(feature = "symspell")]
    symspell_inner.load_dictionary(
      Path::new("data/frequency_dictionary_en_82_765.txt"),
      0,
      1,
      " ",
    );
    #[cfg(feature = "symspell")]
    let symspell = Arc::new(Mutex::new(symspell_inner));

    // Spawn UI batch processor thread (60 FPS)
    playhead_handler::PlayheadArea::spawn_ui_processor(
      Arc::clone(&playhead_area.ui_update_queue),
      self.cb_sink.clone(),
      #[cfg(feature = "symspell")]
      Arc::clone(&playhead_area.tmp_buf),
      #[cfg(feature = "symspell")]
      Arc::clone(&symspell),
      #[cfg(feature = "symspell")]
      Arc::clone(&playhead_area.rpl_state),
      #[cfg(feature = "symspell")]
      self.regex_tx.clone(),
    );

    let playhead_area_tx = playhead_area.run();

    thread::spawn(move || {
      for control_message in &self.rx {
        match control_message {
          Message::Move(direction, canvas_size) => {
            playhead_area_tx
              .send(playhead_handler::Message::Move(
                direction,
                canvas_size,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::Leap(direction, canvas_size) => {
            let leap_steps = match direction {
              Direction::Up | Direction::Down => 4,
              Direction::Left | Direction::Right => 8,
              Direction::Idle => 0,
            };
            playhead_area_tx
              .send(playhead_handler::Message::Leap(
                direction,
                leap_steps,
                canvas_size,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetCurrentPos(position, offset) => {
            playhead_area_tx
              .send(playhead_handler::Message::SetCurrentPos(
                position,
                offset,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::UpdateInfoStatusView() => {
            playhead_area_tx
              .send(playhead_handler::Message::UpdateInfoStatusView(
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetGridArea(current_pos) => {
            playhead_area_tx
              .send(playhead_handler::Message::SetGridArea(
                current_pos,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetActivePos(tick) => {
            playhead_area_tx
              .send(playhead_handler::Message::SetActivePos(
                tick,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::Scale(dir) => {
            playhead_area_tx
              .send(playhead_handler::Message::Scale(dir, self.cb_sink.clone()))
              .unwrap();
          }
          Message::SetMatcher(matcher) => {
            playhead_area_tx
              .send(playhead_handler::Message::SetMatcher(
                matcher,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetGridSize(width, height) => {
            playhead_area_tx
              .send(playhead_handler::Message::SetGridSize(
                width,
                height,
                self.cb_sink.clone(),
              ))
              .unwrap();
          }
          Message::SetScaleModeLeft(scale_mode) => {
            let cb_sink = self.cb_sink.clone();

            playhead_area_tx
              .send(playhead_handler::Message::SetScaleModeLeft(scale_mode))
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
              .send(playhead_handler::Message::SetScaleModeTop(scale_mode))
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
          Message::SetScaleRootTop(scale_root) => {
            let cb_sink = self.cb_sink.clone();

            playhead_area_tx
              .send(playhead_handler::Message::SetScaleRootTop(scale_root))
              .unwrap();

            cb_sink
              .send(Box::new(move |siv| {
                siv.call_on_name(
                  consts::canvas_editor_section_view,
                  move |canvas: &mut Canvas<GridEditor>| {
                    let editor = canvas.state_mut();
                    editor.scale_root_top = scale_root;
                  },
                );
              }))
              .unwrap();
          }
          Message::ToggleAccumulationMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleAccumulationMode(cb_sink))
              .unwrap();
          }
          Message::ToggleForwardMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleForwardMode(cb_sink))
              .unwrap();
          }
          Message::ToggleReverseMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleReverseMode(cb_sink))
              .unwrap();
          }
          Message::TogglePendulumMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::TogglePendulumMode(cb_sink))
              .unwrap();
          }
          Message::SetTempo(bpm) => {
            playhead_area_tx
              .send(playhead_handler::Message::SetTempo(bpm))
              .unwrap();

            self.midi_tx.send(midi::Message::SetTempo(bpm)).unwrap();
          }
          Message::SetRatio(ratio) => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::SetRatio(ratio, cb_sink))
              .unwrap();
          }
          Message::ToggleArpeggiatorMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleArpeggiatorMode(cb_sink))
              .unwrap();
          }
          Message::ToggleRandomMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleRandomMode(cb_sink))
              .unwrap();
          }
          Message::ToggleEventOperatorMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleEventOperatorMode(cb_sink))
              .unwrap();
          }
          Message::ToggleDrainQueueMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleDrainQueueMode(cb_sink))
              .unwrap();
          }
          Message::ToggleSweepMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleSweepMode(cb_sink))
              .unwrap();
          }
          Message::ToggleDynLengthMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleDynLengthMode(cb_sink))
              .unwrap();
          }
          Message::ToggleFreezeMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ToggleFreezeMode(cb_sink))
              .unwrap();
          }
          Message::CycleScaleRootTop(dir) => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::CycleScaleRootTop(cb_sink, dir))
              .unwrap();
          }
          Message::CycleScaleMode(dir) => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::CycleScaleMode(cb_sink, dir))
              .unwrap();
          }
          Message::ClearQueue() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::ClearQueue(cb_sink))
              .unwrap();
          }
          Message::SetGridSplits(v, h) => {
            playhead_area_tx
              .send(playhead_handler::Message::SetGridSplits(v, h))
              .unwrap();
          }
          Message::CycleTiltMode() => {
            let cb_sink = self.cb_sink.clone();
            playhead_area_tx
              .send(playhead_handler::Message::CycleTiltMode(cb_sink))
              .unwrap();
          }
        }
      }
    });
  }
}
