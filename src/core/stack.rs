use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::midi::{self, MidiMsg};

#[derive(Clone, Debug)]
pub enum Message {
  Push(MidiMsg),
}

pub struct Stack {
  pub stack: Arc<Mutex<Vec<midi::MidiMsg>>>,
}

impl Stack {
  pub fn new() -> Stack {
    Stack {
      stack: Arc::new(Mutex::new(vec![])),
    }
  }

  pub fn run(self: Arc<Self>, midi_tx: Sender<midi::Message>) -> Sender<Message> {
    let (tx, rx) = channel();

    thread::spawn(move || {
      for control_message in &rx {
        match control_message {
          Message::Push(midi_msg) => {
            self.push(midi_msg);
          }
        }
      }
    });

    tx
  }

  pub fn refresh(self: Arc<Self>, midi_tx: Sender<midi::Message>) {
    thread::spawn(move || {
      let frame_duration = Duration::from_millis(34); // 33.33 ms per frame
      loop {
        let frame_start = Instant::now();

        {
          let mut st = self.stack.lock().unwrap();
          st.retain_mut(|item| {
            if !item.is_played {
              let _ = midi_tx.send(midi::Message::Trigger(item.clone(), true));
              item.is_played = true;
            }
            if item.length < 2 {
              let _ = midi_tx.send(midi::Message::Trigger(item.clone(), false));
            }
            item.length -= 1;
            // Keep the message if its length is still >= 1
            item.length >= 1
          });
        }

        let elapsed = frame_start.elapsed();

        if elapsed < frame_duration {
          thread::sleep(frame_duration - elapsed);
        }
      }
    });
  }

  pub fn push(&self, midi_msg: MidiMsg) {
    // Retrigger duplicates
    // for (const id in this.stack) {
    //   const dup = this.stack[id]
    //   if (dup.channel === channel && dup.octave === octave && dup.note === note) { this.release(item, id) }
    // }
    let mut stack = self.stack.lock().unwrap();
    stack.push(midi_msg);
  }

  // pub fn press(&self, item: &MidiMsg) {
  //   // if !item.is_some() {
  //   //   return;
  //   // }
  //   // self.trigger(item, true);
  //   item.is_played = true;
  // }

  // pub fn release(&self, item: &MidiMsg) {
  //   // if !item.is_some() {
  //   //   return;
  //   // }
  //   // println!("release: {:?}", item.length);
  //   // self.trigger(item, false);
  //   // let mut stack = self.stack.lock().unwrap();
  //   // stack.remove(id);
  //   // stack.remove(id);
  //   // println!("release stack: {:?}", stack);
  // }
}
