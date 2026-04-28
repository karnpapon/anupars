use std::collections::HashMap;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use crate::core::io::midi::{self, MidiMsg};

type NoteKey = (u8, u8, u8); // (note, octave, channel)
type HeldNotes = Arc<Mutex<HashMap<NoteKey, midi::MidiMsg>>>;

#[derive(Clone, Debug)]
pub enum Message {
  Push(MidiMsg),
  Hold(MidiMsg),
  Release(MidiMsg),
  ReleaseAll(),
}

pub struct Stack {
  pub stack: Arc<Mutex<Vec<midi::MidiMsg>>>,
  pub held_notes: HeldNotes,
  // pub stack_msg_config: Arc<Mutex<Vec<midi::MidiMsg>>>,
}

impl Default for Stack {
  fn default() -> Self {
    Self::new()
  }
}

impl Stack {
  pub fn new() -> Stack {
    Stack {
      stack: Arc::new(Mutex::new(vec![])),
      held_notes: Arc::new(Mutex::new(HashMap::new())),
      // stack_msg_config: Arc::new(Mutex::new(vec![])),
    }
  }

  pub fn run(self: Arc<Self>, _midi_tx: Sender<midi::Message>) -> Sender<Message> {
    let (tx, rx) = channel();

    thread::Builder::new()
      .name(crate::core::consts::THREAD_NAME_STACK_EVENT_LOOP.to_string())
      .spawn(move || {
        for control_message in &rx {
          match control_message {
            Message::Push(midi_msg) => {
              self.push(midi_msg);
            }
            Message::Hold(midi_msg) => {
              self.hold(midi_msg);
            }
            Message::Release(midi_msg) => {
              self.release(midi_msg, &_midi_tx);
            }
            Message::ReleaseAll() => {
              let mut held = self.held_notes.lock().unwrap();
              let mut stack = self.stack.lock().unwrap();
              let notes_to_release: Vec<midi::MidiMsg> = held.values().cloned().collect();

              held.clear();
              stack.clear();

              drop(held);
              drop(stack);

              for msg in notes_to_release {
                let _ = _midi_tx.send(midi::Message::Trigger(msg, false));
              }
            }
          }
        }
      })
      .expect("Failed to spawn stack-event-loop thread");

    tx
  }

  pub fn refresh(self: Arc<Self>, midi_tx: Sender<midi::Message>) {
    thread::Builder::new()
      .name(crate::core::consts::THREAD_NAME_STACK_REFRESH.to_string())
      .spawn(move || {
        // Reduced from 34ms to 8ms for 8x better timing resolution
        // This provides ~125 FPS update rate for precise MIDI note releases
        let frame_duration = Duration::from_millis(8); // 8 ms per frame (125 FPS)
        loop {
          let frame_start = Instant::now();

          {
            let mut st = self.stack.lock().unwrap();
            let held = self.held_notes.lock().unwrap();

            // Batch collect all note-offs to send at once (reduces lock time)
            let mut notes_to_release = Vec::new();

            st.retain_mut(|item| {
              // Skip held notes - they should not be auto-released
              let note_key = (item.note.round() as u8, item.octave, item.channel);
              if held.contains_key(&note_key) {
                return true; // Keep held notes in the stack
              }

              if item.length < 2 {
                notes_to_release.push(item.clone());
              }
              item.length -= 1;
              // Keep the message if its length is still >= 1
              item.length >= 1
            });

            // Release lock before sending MIDI (reduces contention)
            drop(st);
            drop(held);

            // Send all note-offs in batch
            for note in notes_to_release {
              let _ = midi_tx.send(midi::Message::Trigger(note, false));
            }
          }

          let elapsed = frame_start.elapsed();

          if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
          }
        }
      })
      .expect("Failed to spawn stack-refresh thread");
  }

  pub fn push(&self, midi_msg: MidiMsg) {
    let mut stack = self.stack.lock().unwrap();
    stack.push(midi_msg);
  }

  pub fn hold(&self, midi_msg: MidiMsg) {
    use std::collections::hash_map::Entry;

    let note_key = (
      midi_msg.note.round() as u8,
      midi_msg.octave,
      midi_msg.channel,
    );
    let mut held = self.held_notes.lock().unwrap();

    // Use Entry API to avoid duplicate lookups
    if let Entry::Vacant(e) = held.entry(note_key) {
      e.insert(midi_msg.clone());
      drop(held);

      // Push to stack with a very long length so it doesn't auto-release
      let mut stack = self.stack.lock().unwrap();
      let mut held_msg = midi_msg;
      held_msg.length = 255; // Max length to prevent auto-release
      stack.push(held_msg);
    }
  }

  pub fn release(&self, midi_msg: MidiMsg, midi_tx: &Sender<midi::Message>) {
    let note_key = (
      midi_msg.note.round() as u8,
      midi_msg.octave,
      midi_msg.channel,
    );

    // Remove from held notes if present
    let mut held = self.held_notes.lock().unwrap();
    let was_held = held.remove(&note_key).is_some();
    drop(held);

    // Always remove from stack (whether held or not)
    let mut stack = self.stack.lock().unwrap();
    let had_in_stack = stack.iter().any(|item| {
      let item_key = (item.note.round() as u8, item.octave, item.channel);
      item_key == note_key
    });

    stack.retain(|item| {
      let item_key = (item.note.round() as u8, item.octave, item.channel);
      item_key != note_key
    });
    drop(stack);

    // Send note-off if the note was active (either held or in stack)
    if was_held || had_in_stack {
      let _ = midi_tx.send(midi::Message::Trigger(midi_msg, false));
    }
  }
}
