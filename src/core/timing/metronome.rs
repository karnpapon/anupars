use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use num_traits::ToPrimitive;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use crate::core::consts;
use crate::core::io::midi;
use crate::core::playhead;
use crate::core::playhead::UIUpdate;
use crate::core::utils;

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
  /// Raw MIDI clock byte received from an external device:
  /// 0xF8 = timing clock (24 PPQN), 0xFA = start, 0xFB = continue, 0xFC = stop
  ExternalClock(u8),
}

#[derive(Debug)]
pub struct Metronome {
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  pub playhead_tx: Sender<playhead::Message>,
  pub midi_tx: Option<Sender<midi::Message>>,
  ui_tx: Sender<UIUpdate>,
  is_playing: Arc<AtomicBool>,
  current_position: Arc<AtomicUsize>,
  current_bpm: Arc<AtomicUsize>,
  /// True while the sequencer is being driven by incoming MIDI clock (external sync)
  ext_active: Arc<AtomicBool>,
  /// Total count of received 0xF8 pulses; used to convert 24 PPQN → 16 TPB
  ext_pulse_count: Arc<AtomicUsize>,
}

impl Metronome {
  pub fn new(ui_tx: Sender<UIUpdate>, playhead_tx: Sender<playhead::Message>) -> Self {
    let (tx, rx) = channel();

    Self {
      tx,
      rx,
      ui_tx,
      playhead_tx,
      midi_tx: None,
      is_playing: Arc::new(AtomicBool::new(false)),
      current_position: Arc::new(AtomicUsize::new(0)),
      current_bpm: Arc::new(AtomicUsize::new(consts::DEFAULT_TEMPO)),
      ext_active: Arc::new(AtomicBool::new(false)),
      ext_pulse_count: Arc::new(AtomicUsize::new(0)),
    }
  }

  pub fn set_midi_tx(&mut self, midi_tx: Sender<midi::Message>) {
    self.midi_tx = Some(midi_tx);
  }

  /// WASM: read the currently stored `is_playing` flag.
  #[cfg(target_arch = "wasm32")]
  pub fn wasm_is_playing(&self) -> bool {
    self.is_playing.load(Ordering::Relaxed)
  }

  /// WASM: read the currently stored BPM.
  #[cfg(target_arch = "wasm32")]
  pub fn wasm_current_bpm(&self) -> f64 {
    self.current_bpm.load(Ordering::Relaxed) as f64
  }

  /// WASM: drain pending control messages (StartStop, Tempo, …) without
  /// touching the Clock or spawning anything.
  #[cfg(target_arch = "wasm32")]
  pub fn wasm_tick(&self) {
    while let Ok(msg) = self.rx.try_recv() {
      match msg {
        Message::StartStop => {
          self.is_playing.fetch_xor(true, Ordering::SeqCst);
        }
        Message::Reset => {
          self.current_position.store(0, Ordering::Relaxed);
        }
        Message::Tempo(tempo) => {
          let bpm = tempo.to_integer() as usize;
          self.current_bpm.store(bpm, Ordering::Relaxed);
          let _ = self.playhead_tx.send(playhead::Message::SetTempo(bpm));
          let _ = self.ui_tx.send(UIUpdate::BpmDisplay(utils::build_bpm_status_str(bpm)));
        }
        Message::NudgeTempo(nudge) => {
          let old_bpm = self.current_bpm.load(Ordering::Relaxed);
          let new_bpm = (old_bpm as i64 + nudge.to_integer()).max(20).min(999) as usize;
          self.current_bpm.store(new_bpm, Ordering::Relaxed);
          let _ = self.playhead_tx.send(playhead::Message::SetTempo(new_bpm));
          let _ = self.ui_tx.send(UIUpdate::BpmDisplay(utils::build_bpm_status_str(new_bpm)));
        }
        Message::Tap => { /* simplified: ignore in WASM */ }
        Message::ExternalClock(_) => { /* no external MIDI in WASM */ }
        // These are normally sent by the internal Clock; ignore in WASM.
        Message::Time(_) | Message::Signature(_) => {}
      }
    }
  }

  #[cfg(not(target_arch = "wasm32"))]
  pub fn run(self) {
    let clock = Arc::new(clock::Clock::new());
    let metronome_tx_cloned = self.tx.clone();
    let clock_tx = clock.run(metronome_tx_cloned);
    let mut ext_beat_instant: Option<Instant> = None;

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
        Message::ExternalClock(byte) => {
          match byte {
            0xFA => {
              // Start: stop internal clock, reset to tick 0, activate external sync
              let was_playing = self.is_playing.swap(false, Ordering::SeqCst);
              if was_playing {
                clock_tx.send(clock::Message::StartStop).unwrap();
              }
              self.ext_pulse_count.store(0, Ordering::SeqCst);
              self.current_position.store(0, Ordering::Relaxed);
              self.ext_active.store(true, Ordering::SeqCst);
              consts::EXT_CLOCK_ACTIVE.store(true, Ordering::SeqCst);
              ext_beat_instant = None;
            }
            0xFB => {
              // Continue: stop internal clock, resume from current position
              let was_playing = self.is_playing.swap(false, Ordering::SeqCst);
              if was_playing {
                clock_tx.send(clock::Message::StartStop).unwrap();
              }
              self.ext_active.store(true, Ordering::SeqCst);
              consts::EXT_CLOCK_ACTIVE.store(true, Ordering::SeqCst);
            }
            0xFC => {
              // Stop: deactivate external sync
              self.ext_active.store(false, Ordering::SeqCst);
              consts::EXT_CLOCK_ACTIVE.store(false, Ordering::SeqCst);
              ext_beat_instant = None;
              let bpm = self.current_bpm.load(Ordering::Relaxed);
              let _ = self.ui_tx.send(UIUpdate::BpmDisplay(utils::build_bpm_status_str(bpm)));
            }
            0xF8 => {
              // Timing clock: convert 24 PPQN to 16 internal ticks-per-beat
              // Fire an internal tick whenever (pulse * 16) / 24 increments.
              if self.ext_active.load(Ordering::Relaxed) {
                let pulse = self.ext_pulse_count.fetch_add(1, Ordering::SeqCst);
                // Measure BPM at each beat boundary (every 24 pulses = 1 beat)
                if pulse.is_multiple_of(24) {
                  let now = Instant::now();
                  if let Some(prev) = ext_beat_instant.replace(now) {
                    let elapsed_ms = now.duration_since(prev).as_millis() as usize;
                    if elapsed_ms > 0 {
                      let bpm = (60_000 / elapsed_ms).clamp(20, 999);
                      self.current_bpm.store(bpm, Ordering::Relaxed);
                      let _ = self.ui_tx.send(UIUpdate::BpmDisplay(format!("~{bpm}")));
                    }
                  }
                }
                let prev_tick = (pulse * 16) / 24;
                let curr_tick = ((pulse + 1) * 16) / 24;
                if curr_tick > prev_tick {
                  self.current_position.store(curr_tick, Ordering::Relaxed);
                  let _ = self
                    .playhead_tx
                    .send(playhead::Message::SetActivePos(curr_tick));
                }
              }
            }
            _ => {}
          }
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
            .send(playhead::Message::SetTempo(bpm))
            .unwrap();
        }
        Message::Time(time) => {
          let tick = time.ticks().to_usize().unwrap();
          self.current_position.store(tick, Ordering::Relaxed);

          self
            .playhead_tx
            .send(playhead::Message::SetActivePos(tick))
            .unwrap();

          // Send MIDI clock ticks
          // Internal: 16 ticks per quarter note (beat)
          // MIDI Standard: 24 PPQN (pulses per quarter note)
          // 24 / 16 = 1.5 per tick → alternate 2 clocks on even ticks, 1 on odd ticks
          // Over 16 ticks: 8×2 + 8×1 = 24 ✓
          if let Some(ref midi_tx) = self.midi_tx {
            let midi_count = if tick % 2 == 0 { 2 } else { 1 };
            for _ in 0..midi_count {
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
