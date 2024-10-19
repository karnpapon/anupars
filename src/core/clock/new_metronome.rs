use std::sync::{Arc, RwLock};

use hound;
use lazy_static::lazy_static;
use log::info;

use super::dsp::AudioGenerator;

static PRIMARY_CLICK_BYTES: &[u8] = include_bytes!("assets/click.wav");
static ACCENT_CLICK_BYTES: &[u8] = include_bytes!("assets/accent_click.wav");
static SUBDIVISION_CLICK_BYTES: &[u8] = include_bytes!("assets/subdivision_click.wav");

lazy_static! {
  static ref PRIMARY_CLICK_SAMPLES: Vec<f32> = sample_bytes_to_vec(PRIMARY_CLICK_BYTES);
  static ref ACCENT_CLICK_SAMPLES: Vec<f32> = sample_bytes_to_vec(ACCENT_CLICK_BYTES);
  static ref SUBDIVISION_CLICK_SAMPLES: Vec<f32> = sample_bytes_to_vec(SUBDIVISION_CLICK_BYTES);
}

/// Parse the given bytes into a vector of audio samples.
///
/// Return vector of parsed sample values.
/// Return an empty vector if parsing fails.
fn sample_bytes_to_vec(bytes: &[u8]) -> Vec<f32> {
  let click_reader = if let Ok(reader) = hound::WavReader::new(bytes) {
    Some(reader)
  } else {
    None
  };

  return if let Some(mut reader) = click_reader {
    reader.samples::<f32>().filter_map(Result::ok).collect()
  } else {
    Vec::<f32>::new()
  };
}

#[derive(Debug)]
pub enum MetronomeCommand {
  SetVolume(f32),
  SetTempo(usize),
  Toggle8ths,
  Toggle16ths,
  ToggleTriplets,
}

struct MetronomeState {
  samples_since_last_primary_click: usize,
  primary_click_sample_index: usize,
  samples_since_last_even_subdivision_click: usize,
  even_subdivision_click_sample_index: usize,
  samples_since_last_triplet_click: usize,
  triplet_click_sample_index: usize,
  tempo: usize,
  eighths: bool,
  sixteenths: bool,
  triplets: bool,
  volume: f32,
}

impl MetronomeState {
  pub fn new() -> Self {
    let tempo: usize = 120;
    MetronomeState {
      samples_since_last_primary_click: 0,
      primary_click_sample_index: 0,
      samples_since_last_even_subdivision_click: 0,
      even_subdivision_click_sample_index: 0,
      samples_since_last_triplet_click: 0,
      triplet_click_sample_index: 0,
      tempo: tempo,
      eighths: false,
      sixteenths: false,
      triplets: false,
      volume: 1.0,
    }
  }

  fn increment_sample_counters(
    state: &mut MetronomeState,
    s_per_primary: usize,
    s_per_even: usize,
    s_per_triple: usize,
  ) {
    state.triplet_click_sample_index += 1;
    state.samples_since_last_triplet_click += 1;
    state.primary_click_sample_index += 1;
    state.samples_since_last_primary_click += 1;
    state.even_subdivision_click_sample_index += 1;
    state.samples_since_last_even_subdivision_click += 1;

    if state.samples_since_last_even_subdivision_click >= s_per_even {
      state.even_subdivision_click_sample_index = 0;
      state.samples_since_last_even_subdivision_click = 0;
    }

    if state.samples_since_last_triplet_click >= s_per_triple {
      state.triplet_click_sample_index = 0;
      state.samples_since_last_triplet_click = 0;
    }

    if state.samples_since_last_primary_click >= s_per_primary {
      state.primary_click_sample_index = 0;
      state.samples_since_last_primary_click = 0;
      state.samples_since_last_even_subdivision_click = 0;
      state.even_subdivision_click_sample_index = 0;
      state.samples_since_last_triplet_click = 0;
      state.triplet_click_sample_index = 0;
    }
  }
}

pub struct SimpleMetronome {
  state: Arc<RwLock<MetronomeState>>,
}

impl SimpleMetronome {
  pub fn new() -> Self {
    let state = Arc::new(RwLock::new(MetronomeState::new()));
    SimpleMetronome { state }
  }

  fn set_tempo(&self, tempo: usize) {
    let mut state = self.state.write().unwrap();
    state.tempo = tempo;
  }

  fn set_volume(&self, vol: f32) {
    let mut state = self.state.write().unwrap();
    state.volume = vol;
  }

  fn toggle_8ths(&self) {
    let mut state = self.state.write().unwrap();
    state.eighths = !state.eighths;
  }

  fn toggle_16ths(&self) {
    let mut state = self.state.write().unwrap();
    state.sixteenths = !state.sixteenths;
  }

  fn toggle_triplets(&self) {
    let mut state = self.state.write().unwrap();
    state.triplets = !state.triplets;
  }

  /// Generate a stream of metronome audio samples
  fn generate_metronome_sound(&self) -> f32 {
    let mut state = self.state.write().unwrap();

    let volume = state.volume;
    let tempo = state.tempo;
    let (eighths, sixteenths, triplets) = { (state.eighths, state.sixteenths, state.triplets) };

    let samples_per_primary_click = self.samples_per_beat(tempo);
    let samples_per_even_subdivision_click =
      self.samples_per_even_subdivision(eighths, sixteenths, samples_per_primary_click);
    let samples_per_triplet_click = self.samples_per_triplet(triplets, samples_per_primary_click);

    let primary_sample =
      self.get_click_sample(&PRIMARY_CLICK_SAMPLES, state.primary_click_sample_index);

    let even_subdivision_sample = if eighths || sixteenths {
      self.get_click_sample(
        &SUBDIVISION_CLICK_SAMPLES,
        state.even_subdivision_click_sample_index,
      )
    } else {
      0.0
    };

    let triplet_sample = if triplets {
      self.get_click_sample(&SUBDIVISION_CLICK_SAMPLES, state.triplet_click_sample_index)
    } else {
      0.0
    };

    MetronomeState::increment_sample_counters(
      &mut state,
      samples_per_primary_click,
      samples_per_even_subdivision_click,
      samples_per_triplet_click,
    );

    let sample = (primary_sample + even_subdivision_sample + triplet_sample) * volume;
    sample
  }

  /// Helper function to fetch the appropriate click sample
  fn get_click_sample(&self, samples: &[f32], index: usize) -> f32 {
    if index < samples.len() {
      samples[index]
    } else {
      0.0
    }
  }

  /// Calculate the number of samples between even subdivisions
  fn samples_per_even_subdivision(
    &self,
    eighths: bool,
    sixteenths: bool,
    samples_per_primary_click: usize,
  ) -> usize {
    if sixteenths {
      samples_per_primary_click / 4
    } else if eighths {
      samples_per_primary_click / 2
    } else {
      0
    }
  }

  /// Calculate the number of samples between triplet subdivisions
  fn samples_per_triplet(&self, triplets: bool, samples_per_primary_click: usize) -> usize {
    if triplets {
      samples_per_primary_click / 3
    } else {
      0
    }
  }

  fn samples_per_beat(&self, tempo: usize) -> usize {
    (48000.0 * 60.0 / tempo as f32 * 2.0) as usize
  }
}

impl AudioGenerator for SimpleMetronome {
  type Command = MetronomeCommand;

  fn next_sample(&self) -> f32 {
    self.generate_metronome_sound()
  }

  fn process_command(&self, command: Self::Command) {
    match command {
      MetronomeCommand::SetVolume(vol) => {
        self.set_volume(vol);
        info!("Volume set: {}", vol);
      }
      MetronomeCommand::SetTempo(tempo) => {
        self.set_tempo(tempo);
        info!("Tempo set: {}", tempo);
      }
      MetronomeCommand::Toggle8ths => {
        self.toggle_8ths();
        info!("Metronome received toggle 8ths message.");
      }
      MetronomeCommand::Toggle16ths => {
        self.toggle_16ths();
        info!("Metronome received toggle 16ths message.");
      }
      MetronomeCommand::ToggleTriplets => {
        self.toggle_triplets();
        info!("Metronome received toggle triplets message.");
      }
    };
  }
}
