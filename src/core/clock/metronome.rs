use std::{
  collections::BTreeSet,
  sync::{
    mpsc::{channel, Receiver, Sender},
    Arc, Mutex,
  },
};

use num::ToPrimitive;

use crate::view::common::marker;

use super::clock;

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
  pub marker_tx: Sender<marker::Message>,
  cb_sink: cursive::CbSink,
}

impl Metronome {
  pub fn new(cb_sink: cursive::CbSink, marker_tx: Sender<marker::Message>) -> Self {
    let (tx, rx) = channel();

    Self {
      tx,
      rx,
      cb_sink,
      marker_tx,
    }
  }

  pub fn run(self) {
    let clock = Arc::new(clock::Clock::new());
    let metronome_tx_cloned = self.tx.clone();
    let metronome_tx_cloned_2 = self.tx.clone();
    let clock_cloned = Arc::clone(&clock);
    let clock_tx = clock.run(metronome_tx_cloned);
    clock_cloned.run_tick(metronome_tx_cloned_2);

    for control_message in self.rx {
      match control_message {
        Message::Reset => {
          clock_tx.send(clock::Message::Reset).unwrap();
        }
        Message::StartStop => {
          clock_tx.send(clock::Message::StartStop).unwrap();
        }
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
          clock_tx.send(clock::Message::Tempo(tempo)).unwrap();
        }
        Message::Time(time) => {
          let tick = time.ticks().to_usize().unwrap();
          self
            .marker_tx
            .send(marker::Message::SetActivePos(tick))
            .unwrap();
        }
      }
    }
  }
}
