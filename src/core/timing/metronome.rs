use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;

// use cursive::theme::{ColorStyle, ColorType, Style};
// use cursive::utils::markup::StyledString;
// use cursive::views::TextView;
use num::ToPrimitive;

use crate::core::consts;
use crate::core::midi;
use crate::view::common::playhead_controller;

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
  pub playhead_tx: Sender<playhead_controller::Message>,
  pub midi_tx: Option<Sender<midi::Message>>,
  cb_sink: cursive::CbSink,
  is_playing: Arc<AtomicBool>,
  current_position: Arc<AtomicUsize>,
  current_bpm: Arc<AtomicUsize>,
}

impl Metronome {
  pub fn new(cb_sink: cursive::CbSink, playhead_tx: Sender<playhead_controller::Message>) -> Self {
    let (tx, rx) = channel();

    Self {
      tx,
      rx,
      cb_sink,
      playhead_tx,
      midi_tx: None,
      is_playing: Arc::new(AtomicBool::new(false)),
      current_position: Arc::new(AtomicUsize::new(0)),
      current_bpm: Arc::new(AtomicUsize::new(consts::DEFAULT_TEMPO)),
    }
  }

  pub fn set_midi_tx(&mut self, midi_tx: Sender<midi::Message>) {
    self.midi_tx = Some(midi_tx);
  }

  pub fn run(self) {
    let clock = Arc::new(clock::Clock::new());
    let metronome_tx_cloned = self.tx.clone();
    let clock_tx = clock.run(metronome_tx_cloned);

    for control_message in self.rx {
      match control_message {
        Message::Reset => {
          clock_tx.send(clock::Message::Reset).unwrap();

          self.current_position.store(0, Ordering::Relaxed);
          if let Some(ref midi_tx) = self.midi_tx {
            let _ = midi_tx.send(midi::Message::ClockSongPosition(0));
          }
        }
        Message::StartStop => {
          clock_tx.send(clock::Message::StartStop).unwrap();

          let was_playing = self.is_playing.fetch_xor(true, Ordering::SeqCst);
          if let Some(ref midi_tx) = self.midi_tx {
            if was_playing {
              let _ = midi_tx.send(midi::Message::ClockStop());
            } else {
              let position = self.current_position.load(Ordering::Relaxed);
              let _ = midi_tx.send(midi::Message::ClockSongPosition(position));
              let _ = midi_tx.send(midi::Message::ClockStart());
            }
          }
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

          // Forward tempo to playhead as BPM (convert from Ratio to usize)
          let bpm = tempo.to_integer() as usize;
          self.current_bpm.store(bpm, Ordering::Relaxed);
          self
            .playhead_tx
            .send(playhead_controller::Message::SetTempo(bpm))
            .unwrap();
        }
        Message::Time(time) => {
          let tick = time.ticks().to_usize().unwrap();
          self.current_position.store(tick, Ordering::Relaxed);

          self
            .playhead_tx
            .send(playhead_controller::Message::SetActivePos(tick))
            .unwrap();

          // Send MIDI clock ticks
          // Internal: 4 ticks per quarter note (beat)
          // MIDI Standard: 24 PPQN (pulses per quarter note)
          // Solution: Send 6 MIDI clocks per tick (24/4 = 6)
          if let Some(ref midi_tx) = self.midi_tx {
            for _ in 0..6 {
              let _ = midi_tx.send(midi::Message::ClockTick());
            }
          }

          // let bpm = self.current_bpm.load(Ordering::Relaxed);
          // let tick_in_beat = time.ticks_since_beat().to_integer() as usize;
          // let (symbol, color) = match tick_in_beat {
          //   0 => ("\\", ColorType::rgb(255, 255, 255)),
          //   1 => ("|", ColorType::rgb(100, 100, 100)),
          //   2 => ("/", ColorType::rgb(100, 100, 100)),
          //   _ => ("|", ColorType::rgb(100, 100, 100)),
          // };
          // let styled = StyledString::styled(
          //   format!("{bpm} {symbol}"),
          //   Style::from(ColorStyle::front(color)),
          // );
          // let _ = self
          //   .cb_sink
          //   .send(Box::new(move |siv: &mut cursive::Cursive| {
          //     siv.call_on_name(consts::bpm_status_unit_view, |view: &mut TextView| {
          //       view.set_content(styled);
          //     });
          //   }));
        }
      }
    }
  }
}
