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
}

impl Metronome {
  pub fn new() -> Self {
    let (tx, rx) = channel();

    Self { tx, rx }
  }

  pub fn run(self, cb_sink: cursive::CbSink) {
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
          // terminal_tx
          //   .send(thread::Message::Signature(signature))
          //   .unwrap();
        }
        // sent by clock
        Message::Tempo(tempo) => {
          clock_tx.send(clock::Message::Tempo(tempo)).unwrap();
          // terminal_tx.send(thread::Message::Tempo(tempo)).unwrap();
        }
        Message::Time(time) => {
          cb_sink
            .send(Box::new(move |s| {
              // let num = 1;
              // let denom = 8;
              // let beats_since_bar = time.beats_since_bar();
              let tick = time.ticks().to_usize().unwrap();
              // let mut tick_str = String::from("-").repeat(denom);
              // utils::replace_nth_char_ascii(
              //   &mut tick_str,
              //   beats_since_bar.to_usize().unwrap(),
              //   '|',
              // );
              // s.call_on_name(config::ratio_status_unit_view, |c: &mut TextView| {
              //   c.set_content(utils::build_ratio_status_str((num, denom), &tick_str))
              // })
              // .unwrap();
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
