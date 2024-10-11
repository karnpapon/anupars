use std::sync::mpsc::{channel, Receiver, Sender};

// use crossbeam_utils::sync::{Parker, Unparker};
use cursive::views::{Canvas, TextView};
use num::ToPrimitive;

use crate::{
  core::{config, utils},
  view::canvas_editor::CanvasEditor,
};

use super::clock;
// use super::thread;

#[derive(Clone, Debug)]
pub enum Message {
  Time(clock::Time),
  Signature(clock::Signature),
  Tempo(clock::Tempo),
  Reset,
  // Start,
  StartStop,
  NudgeTempo(clock::NudgeTempo),
  Tap,
}

#[derive(Debug)]
pub struct Metronome {
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  cb_sink: cursive::CbSink,
}

impl Metronome {
  pub fn new(cb_sink: cursive::CbSink) -> Self {
    let (tx, rx) = channel();

    Self { tx, rx, cb_sink }
  }

  pub fn run(self) {
    let clock = clock::Clock::new();
    let clock_tx = clock.start(self.tx.clone());

    for control_message in self.rx {
      match control_message {
        Message::Reset => {
          clock_tx.send(clock::Message::Reset).unwrap();
        }
        // Message::Start => clock_tx.send(clock::Message::Start).unwrap(),
        Message::StartStop => clock_tx.send(clock::Message::StartStop).unwrap(),
        Message::NudgeTempo(nudge) => {
          clock_tx.send(clock::Message::NudgeTempo(nudge)).unwrap();
        }
        Message::Tap => {
          clock_tx.send(clock::Message::Tap).unwrap();
        }
        // sent by clock
        Message::Signature(signature) => {
          clock_tx.send(clock::Message::Signature(signature)).unwrap();
        }
        // sent by clock
        Message::Tempo(tempo) => {
          // self
          //   .cb_sink
          //   .send(Box::new(move |s| {
          //     s.call_on_name(config::bpm_status_unit_view, |view: &mut TextView| {
          //       view.set_content(utils::build_bpm_status_str(tempo.to_usize().unwrap()));
          //     })
          //     .unwrap();
          //   }))
          //   .unwrap();
          clock_tx.send(clock::Message::Tempo(tempo)).unwrap();
        }
        Message::Time(time) => {
          let tick = time.ticks().to_usize().unwrap();
          self
            .cb_sink
            .send(Box::new(move |s| {
              s.call_on_name(
                config::canvas_editor_section_view,
                |c: &mut Canvas<CanvasEditor>| {
                  c.state_mut().marker_mut().set_actived_pos(tick);
                },
              )
              .unwrap();
            }))
            .unwrap();
        }
      }
    }
  }
}
