use super::metronome;
use num::integer::Integer;
use num::rational::Ratio;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

pub type Tick = Ratio<i64>;
pub type Tempo = Ratio<i64>;
pub type NudgeTempo = Ratio<i64>;

static SECONDS_PER_MINUTE: i64 = 60;
static NANOS_PER_SECOND: i64 = 1_000_000_000;

static DEFAULT_TICKS_PER_BEAT: i64 = 4;
static DEFAULT_BEATS_PER_BAR: i64 = 8;
static DEFAULT_BARS_PER_LOOP: i64 = 8;
static DEFAULT_BEATS_PER_MINUTE: i64 = 120;

#[derive(Clone, Copy, Debug)]
pub struct Signature {
  pub ticks_per_beat: Tick,
  pub beats_per_bar: Tick,
  pub bars_per_loop: Tick,
}

impl Signature {
  pub fn default() -> Self {
    Self {
      ticks_per_beat: Ratio::from_integer(DEFAULT_TICKS_PER_BEAT),
      beats_per_bar: Ratio::from_integer(DEFAULT_BEATS_PER_BAR),
      bars_per_loop: Ratio::from_integer(DEFAULT_BARS_PER_LOOP),
    }
  }

  #[allow(dead_code)]
  pub fn ticks_per_beat(&self) -> Tick {
    self.ticks_per_beat
  }

  #[allow(dead_code)]
  pub fn ticks_per_bar(&self) -> Tick {
    self.ticks_per_beat() * self.beats_per_bar
  }

  #[allow(dead_code)]
  pub fn ticks_per_loop(&self) -> Tick {
    self.ticks_per_bar() * self.bars_per_loop
  }

  pub fn ticks_to_beats(&self, ticks: Tick) -> Tick {
    ticks / self.ticks_per_beat
  }

  pub fn ticks_to_bars(&self, ticks: Tick) -> Tick {
    self.ticks_to_beats(ticks) / self.beats_per_bar
  }

  pub fn nanos_per_tick(&self, beats_per_minute: Tick) -> Tick {
    let minutes_per_beat = Ratio::from_integer(1) / beats_per_minute;
    let seconds_per_beat = minutes_per_beat * Ratio::from_integer(SECONDS_PER_MINUTE);
    let nanos_per_beat = seconds_per_beat * Ratio::from_integer(NANOS_PER_SECOND);

    nanos_per_beat / self.ticks_per_beat
  }

  pub fn nanos_per_beat(&self, beats_per_minute: Tick) -> Tick {
    self.nanos_per_tick(beats_per_minute) * self.ticks_per_beat
  }

  #[allow(dead_code)]
  pub fn nanos_per_bar(&self, beats_per_minute: Tick) -> Tick {
    self.nanos_per_beat(beats_per_minute) * self.beats_per_bar
  }

  #[allow(dead_code)]
  pub fn nanos_per_loop(&self, beats_per_minute: Tick) -> Tick {
    self.nanos_per_bar(beats_per_minute) * self.bars_per_loop
  }

  #[allow(dead_code)]
  pub fn beats_per_minute(&self, nanos_per_tick: Tick) -> Tempo {
    let nanos_per_beat = nanos_per_tick * self.ticks_per_beat;
    let beats_per_nano = Ratio::from_integer(1) / nanos_per_beat;
    let beats_per_second = beats_per_nano * Ratio::from_integer(NANOS_PER_SECOND);

    beats_per_second * Ratio::from_integer(SECONDS_PER_MINUTE)
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Time {
  ticks: Tick,
  signature: Signature,
}

impl Time {
  pub fn new(signature: Signature) -> Self {
    Self {
      ticks: Ratio::from_integer(0),
      signature,
    }
  }

  pub fn ticks(&self) -> Tick {
    self.ticks
  }

  pub fn beats(&self) -> Tick {
    self.signature.ticks_to_beats(self.ticks)
  }

  pub fn bars(&self) -> Tick {
    self.signature.ticks_to_bars(self.ticks)
  }

  pub fn ticks_since_beat(&self) -> Tick {
    self.ticks() % self.signature.ticks_per_beat
  }

  pub fn beats_since_bar(&self) -> Tick {
    self.beats() % self.signature.beats_per_bar
  }

  pub fn bars_since_loop(&self) -> Tick {
    self.bars() % self.signature.bars_per_loop
  }

  pub fn ticks_before_beat(&self) -> Tick {
    self.ticks() - self.ticks_since_beat()
  }

  pub fn is_first_tick(&self) -> bool {
    self.ticks_since_beat().floor() == Ratio::from_integer(0)
  }

  pub fn is_first_beat(&self) -> bool {
    self.beats_since_bar().floor() == Ratio::from_integer(0)
  }

  pub fn is_first_bar(&self) -> bool {
    self.bars_since_loop().floor() == Ratio::from_integer(0)
  }

  pub fn next(&self) -> Self {
    Self {
      ticks: self.ticks + 1,
      signature: self.signature,
    }
  }

  pub fn quantize_beat(&self) -> Self {
    // find how far off the beat we are
    let ticks_per_beat = self.signature.ticks_per_beat();
    let ticks_per_half_beat = ticks_per_beat / 2;

    Self {
      // if the beat happened recently
      ticks: if self.ticks_since_beat() < ticks_per_half_beat {
        // nudge back to the beat
        self.ticks_before_beat()
      } else {
        // nudge to the next beat
        self.ticks_before_beat() + ticks_per_beat
      },
      signature: self.signature,
    }
  }
}

#[derive(Clone, Copy, Debug)]
pub struct Timer {
  instant: Instant,
  signature: Signature,
}

impl Timer {
  pub fn new(signature: Signature) -> Self {
    Self {
      instant: Instant::now(),
      signature,
    }
  }

  pub fn nanos(&self) -> Tick {
    Ratio::from_integer(duration_to_nanos(self.instant.elapsed()))
  }

  pub fn nanos_since_tick(&self, beats_per_minute: Tick) -> Tick {
    self.nanos() % self.signature.nanos_per_tick(beats_per_minute)
  }

  #[allow(dead_code)]
  pub fn nanos_since_beat(&self, beats_per_minute: Tick) -> Tick {
    self.nanos() % self.signature.nanos_per_beat(beats_per_minute)
  }

  #[allow(dead_code)]
  pub fn nanos_since_bar(&self, beats_per_minute: Tick) -> Tick {
    self.nanos() % self.signature.nanos_per_bar(beats_per_minute)
  }

  #[allow(dead_code)]
  pub fn nanos_since_loop(&self, beats_per_minute: Tick) -> Tick {
    self.nanos() % self.signature.nanos_per_loop(beats_per_minute)
  }

  pub fn nanos_until_tick(&self, beats_per_minute: Tick) -> Tick {
    let nanos_since_tick = self.nanos_since_tick(beats_per_minute);
    let nanos_per_tick = self.signature.nanos_per_tick(beats_per_minute);
    nanos_per_tick - nanos_since_tick
  }

  pub fn next(&self, beats_per_minute: Tick) -> Tick {
    let nanos_until_tick = self.nanos_until_tick(beats_per_minute);

    let nanos = nanos_until_tick.numer() / nanos_until_tick.denom();

    sleep(Duration::new(0, nanos as u32));

    nanos_until_tick
  }
}

#[derive(Debug)]
pub struct Clock {
  time: Arc<Mutex<Time>>,
  timer: Arc<Mutex<Timer>>,
  signature: Arc<Mutex<Signature>>,
  tempo: Arc<Mutex<Tempo>>,
  tap: Arc<Mutex<Option<Instant>>>,
  playing: AtomicBool,
}

#[derive(Clone, Debug)]
pub enum Message {
  Tempo(Tempo),
  NudgeTempo(NudgeTempo),
  Reset,
  StartStop,
  Signature(Signature),
  Tap,
}

impl Clock {
  pub fn new() -> Self {
    let signature = Arc::new(Mutex::new(Signature::default()));
    let time = Arc::new(Mutex::new(Time::new(Signature::default())));
    let timer = Arc::new(Mutex::new(Timer::new(Signature::default())));
    let tempo = Arc::new(Mutex::new(Ratio::from_integer(DEFAULT_BEATS_PER_MINUTE)));

    Self {
      time,
      timer,
      signature,
      tempo,
      tap: Arc::new(Mutex::new(None)),
      playing: AtomicBool::new(false),
    }
  }

  fn is_playing(&self) -> bool {
    self.playing.load(Ordering::SeqCst)
  }

  pub fn run_tick(self: Arc<Self>, metronome_tx: Sender<metronome::Message>) {
    metronome_tx
      .send(metronome::Message::Signature(Signature::default()))
      .unwrap();
    metronome_tx
      .send(metronome::Message::Tempo(*self.get_tempo().deref()))
      .unwrap();

    thread::spawn(move || loop {
      if self.is_playing() {
        let tick = self.tick();
        metronome_tx
          .send(metronome::Message::Time(self.time()))
          .unwrap();
      } else {
        thread::sleep(Duration::from_millis(100));
      }
    });
  }

  fn get_tempo(&self) -> MutexGuard<Ratio<i64>> {
    let tempo = self.tempo.lock().unwrap();
    tempo
  }

  pub fn run(self: Arc<Self>, metronome_tx: Sender<metronome::Message>) -> Sender<Message> {
    let (tx, rx) = channel();

    metronome_tx
      .send(metronome::Message::Signature(Signature::default()))
      .unwrap();
    metronome_tx
      .send(metronome::Message::Tempo(*self.get_tempo()))
      .unwrap();

    thread::spawn(move || {
      for control_message in &rx {
        match control_message {
          Message::Reset => self.reset(),
          Message::StartStop => {
            self.playing.fetch_xor(true, Ordering::SeqCst);
          }
          Message::Signature(signature) => {
            self.set_signature(signature);
          }
          Message::Tap => {
            if let Some(new_tempo) = self.tap() {
              metronome_tx
                .send(metronome::Message::Tempo(new_tempo))
                .unwrap();
            }
          }
          Message::NudgeTempo(nudge) => {
            let mut _tempo = self.get_tempo();
            let old_tempo = _tempo;
            let new_tempo = *old_tempo + nudge;
            metronome_tx
              .send(metronome::Message::Tempo(new_tempo))
              .unwrap();
          }
          Message::Tempo(tempo) => {
            let mut _tempo = self.get_tempo();
            *_tempo = tempo;
          }
        }
      }
    });

    tx
  }

  // fn restart_task(&mut self, metronome_tx: Sender<metronome::Message>) {
  //   let (tx, rx) = channel();

  //   metronome_tx
  //     .send(metronome::Message::Signature(Signature::default()))
  //     .unwrap();
  //   metronome_tx
  //     .send(metronome::Message::Tempo(self.tempo))
  //     .unwrap();

  //   // Cancel the current task and instantiate a new task with a fresh cancellation token
  //   self.current_token.cancel();
  //   self.current_token = CancellationToken::new();

  //   // Clone the current state post previous user input from which to construct the new task.
  //   let cloned_token = self.current_token.clone();

  //   std::thread::sleep(Duration::from_millis(150));
  //   let _ = tokio::spawn(async move {
  //     tokio::select! {
  //         _ = cloned_token.cancelled() => {}
  //         _ =  Clock::start(tempo, db, ts)  => {}
  //     }
  //   });
  // }

  pub fn reset(&self) {
    let mut time = self.time.lock().unwrap();
    let mut timer = self.timer.lock().unwrap();
    let signature = self.signature.lock().unwrap();
    *time = Time::new(*signature);
    *timer = Timer::new(*signature);
  }

  pub fn set_signature(&self, signature: Signature) {
    let mut sig = self.signature.lock().unwrap();
    *sig = signature;
    let mut time = self.time.lock().unwrap();
    let mut timer = self.timer.lock().unwrap();
    *time = Time::new(*sig);
    *timer = Timer::new(*sig);
  }

  pub fn time(&self) -> Time {
    let t = self.time.lock().unwrap();
    *t
  }

  pub fn tick(&self) -> Tick {
    let nanos_until_tick = self.timer.lock().unwrap().next(*self.get_tempo().deref());
    let mut time = self.time.lock().unwrap();
    *time = time.next();
    nanos_until_tick
  }

  pub fn tap(&self) -> Option<Tempo> {
    let mut time = self.time.lock().unwrap();
    // on every tap, quantize beat
    *time = time.quantize_beat();

    let mut next_tempo = None;

    let mut tap = self.tap.lock().unwrap();

    // if second tap on beat, adjust tempo
    if let Some(t) = *tap {
      let sig = self.signature.lock().unwrap();
      let tap_nanos = Ratio::from_integer(duration_to_nanos(t.elapsed()));
      if tap_nanos < sig.nanos_per_beat(*self.get_tempo().deref()) * 2 {
        let tap_beats_per_nanos = Ratio::from_integer(1) / tap_nanos;
        let tap_beats_per_seconds = tap_beats_per_nanos * Ratio::from_integer(NANOS_PER_SECOND);
        let beats_per_minute = tap_beats_per_seconds * Ratio::from_integer(SECONDS_PER_MINUTE);
        next_tempo = Some(round_to_nearest(beats_per_minute, 100));
      }
    }

    *tap = Some(Instant::now());

    next_tempo
  }
}

fn duration_to_nanos(duration: Duration) -> i64 {
  duration.as_secs() as i64 * 1_000_000_000 + duration.subsec_nanos() as i64
}

fn round_to_nearest<T: Clone + Copy + Integer>(value: Ratio<T>, quantum: T) -> Ratio<T> {
  let quantum_rat = Ratio::from_integer(quantum);
  (value * quantum_rat).round() / quantum_rat
}
