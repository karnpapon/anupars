use crate::core::command::types;
use serde::{Deserialize, Serialize};

/// Musical scale modes and their interval patterns
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ScaleMode {
  #[default]
  Chromatic, // All 12 notes
  Major,         // Ionian mode
  Minor,         // Natural minor (Aeolian)
  HarmonicMinor, // Harmonic minor
  MelodicMinor,  // Melodic minor (ascending)
  Dorian,
  Phrygian,
  Lydian,
  Mixolydian,
  Locrian,
  MajorPentatonic,
  MinorPentatonic,
  Blues,
  WholeTone,
  Diminished,
  Thai7Tet, // Thai 7-TET microtonal scale
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ScaleRoot {
  #[default]
  C = 0,
  CSharp = 1,
  D = 2,
  DSharp = 3,
  E = 4,
  F = 5,
  FSharp = 6,
  G = 7,
  GSharp = 8,
  A = 9,
  ASharp = 10,
  B = 11,
}

impl ScaleRoot {
  /// Get human-readable name
  pub fn name(&self) -> &'static str {
    match self {
      ScaleRoot::C => "C",
      ScaleRoot::CSharp => "C#",
      ScaleRoot::D => "D",
      ScaleRoot::DSharp => "D#",
      ScaleRoot::E => "E",
      ScaleRoot::F => "F",
      ScaleRoot::FSharp => "F#",
      ScaleRoot::G => "G",
      ScaleRoot::GSharp => "G#",
      ScaleRoot::A => "A",
      ScaleRoot::ASharp => "A#",
      ScaleRoot::B => "B",
    }
  }

  /// Get all available scale roots
  pub fn all() -> &'static [ScaleRoot] {
    &[
      ScaleRoot::C,
      ScaleRoot::CSharp,
      ScaleRoot::D,
      ScaleRoot::DSharp,
      ScaleRoot::E,
      ScaleRoot::F,
      ScaleRoot::FSharp,
      ScaleRoot::G,
      ScaleRoot::GSharp,
      ScaleRoot::A,
      ScaleRoot::ASharp,
      ScaleRoot::B,
    ]
  }

  pub fn is_natural(&self) -> bool {
    matches!(
      self,
      ScaleRoot::C
        | ScaleRoot::D
        | ScaleRoot::E
        | ScaleRoot::F
        | ScaleRoot::G
        | ScaleRoot::A
        | ScaleRoot::B
    )
  }

  /// Get the root note offset (0-11) for this scale root
  /// This is added to the intervals of the scale mode to get the actual note indices
  #[allow(clippy::wrong_self_convention)]
  pub fn to_root_offset(&self) -> u8 {
    match self {
      ScaleRoot::C => 0,
      ScaleRoot::CSharp => 1,
      ScaleRoot::D => 2,
      ScaleRoot::DSharp => 3,
      ScaleRoot::E => 4,
      ScaleRoot::F => 5,
      ScaleRoot::FSharp => 6,
      ScaleRoot::G => 7,
      ScaleRoot::GSharp => 8,
      ScaleRoot::A => 9,
      ScaleRoot::ASharp => 10,
      ScaleRoot::B => 11,
    }
  }

  /// Cycle to the next or previous root note based on adjustment
  pub fn cycle(&self, adjustment: types::Adjustment) -> ScaleRoot {
    let all = Self::all();
    let idx = all.iter().position(|&r| r == *self).unwrap_or(0);
    match adjustment {
      types::Adjustment::Increase => all[(idx + 1) % all.len()],
      types::Adjustment::Decrease => all[(idx + all.len() - 1) % all.len()],
    }
  }
}

impl ScaleMode {
  /// Get the intervals (in semitones from root) for this scale mode
  /// Returns which notes (0-11) are included in the scale
  pub fn intervals(&self) -> &'static [f32] {
    match self {
      ScaleMode::Chromatic => &[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0],
      ScaleMode::Major => &[0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0],
      ScaleMode::Minor => &[0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 10.0],
      ScaleMode::HarmonicMinor => &[0.0, 2.0, 3.0, 5.0, 7.0, 8.0, 11.0],
      ScaleMode::MelodicMinor => &[0.0, 2.0, 3.0, 5.0, 7.0, 9.0, 11.0],
      ScaleMode::Dorian => &[0.0, 2.0, 3.0, 5.0, 7.0, 9.0, 10.0],
      ScaleMode::Phrygian => &[0.0, 1.0, 3.0, 5.0, 7.0, 8.0, 10.0],
      ScaleMode::Lydian => &[0.0, 2.0, 4.0, 6.0, 7.0, 9.0, 11.0],
      ScaleMode::Mixolydian => &[0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 10.0],
      ScaleMode::Locrian => &[0.0, 1.0, 3.0, 5.0, 6.0, 8.0, 10.0],
      ScaleMode::MajorPentatonic => &[0.0, 2.0, 4.0, 7.0, 9.0],
      ScaleMode::MinorPentatonic => &[0.0, 3.0, 5.0, 7.0, 10.0],
      ScaleMode::Blues => &[0.0, 3.0, 5.0, 6.0, 7.0, 10.0],
      ScaleMode::WholeTone => &[0.0, 2.0, 4.0, 6.0, 8.0, 10.0],
      ScaleMode::Diminished => &[0.0, 2.0, 3.0, 5.0, 6.0, 8.0, 9.0, 11.0],
      ScaleMode::Thai7Tet => &[0.0, 1.714, 3.429, 5.143, 6.857, 8.571, 10.286, 12.0],
    }
  }

  /// Get human-readable name
  pub fn name(&self) -> &'static str {
    match self {
      ScaleMode::Chromatic => "Chromatic",
      ScaleMode::Major => "Major",
      ScaleMode::Minor => "Natural Minor",
      ScaleMode::HarmonicMinor => "Harmonic Minor",
      ScaleMode::MelodicMinor => "Melodic Minor",
      ScaleMode::Dorian => "Dorian",
      ScaleMode::Phrygian => "Phrygian",
      ScaleMode::Lydian => "Lydian",
      ScaleMode::Mixolydian => "Mixolydian",
      ScaleMode::Locrian => "Locrian",
      ScaleMode::MajorPentatonic => "Major Pentatonic",
      ScaleMode::MinorPentatonic => "Minor Pentatonic",
      ScaleMode::Blues => "Blues",
      ScaleMode::WholeTone => "Whole Tone",
      ScaleMode::Diminished => "Diminished",
      ScaleMode::Thai7Tet => "Thai 7-TET",
    }
  }

  pub fn short_name(&self) -> &'static str {
    match self {
      ScaleMode::Chromatic => "CHRM",
      ScaleMode::Major => "MAJR",
      ScaleMode::Minor => "MINR",
      ScaleMode::HarmonicMinor => "HRMN",
      ScaleMode::MelodicMinor => "MLOD",
      ScaleMode::Dorian => "DRIA",
      ScaleMode::Phrygian => "PHRY",
      ScaleMode::Lydian => "LYDI",
      ScaleMode::Mixolydian => "MIXO",
      ScaleMode::Locrian => "LOCR",
      ScaleMode::MajorPentatonic => "MJPT",
      ScaleMode::MinorPentatonic => "MNPT",
      ScaleMode::Blues => "BLUS",
      ScaleMode::WholeTone => "WHOL",
      ScaleMode::Diminished => "DIMI",
      ScaleMode::Thai7Tet => "THA7",
    }
  }

  /// Check if a note index (0-11) is in this scale
  pub fn contains_note(&self, note_index: f32) -> bool {
    let intervals = self.intervals();
    let note_mod = note_index % 12.0;
    intervals.iter().any(|&i| (i - note_mod).abs() < 0.01)
  }

  /// Get all available scale modes
  pub fn all() -> &'static [ScaleMode] {
    &[
      ScaleMode::Chromatic,
      ScaleMode::Major,
      ScaleMode::Minor,
      ScaleMode::HarmonicMinor,
      ScaleMode::MelodicMinor,
      ScaleMode::Dorian,
      ScaleMode::Phrygian,
      ScaleMode::Lydian,
      ScaleMode::Mixolydian,
      ScaleMode::Locrian,
      ScaleMode::MajorPentatonic,
      ScaleMode::MinorPentatonic,
      ScaleMode::Blues,
      ScaleMode::WholeTone,
      ScaleMode::Diminished,
      ScaleMode::Thai7Tet,
    ]
  }

  pub fn get_root_note(&self, note_index: f32) -> f32 {
    let intervals = self.intervals();
    let note_mod = note_index % 12.0;
    for &interval in intervals {
      if (interval - note_mod).abs() < 0.01 {
        return interval;
      }
    }
    note_index
  }

  pub fn cycle(&self, adjustment: types::Adjustment) -> ScaleMode {
    let all = Self::all();
    let idx = all.iter().position(|&m| m == *self).unwrap_or(0);
    match adjustment {
      types::Adjustment::Increase => all[(idx + 1) % all.len()],
      types::Adjustment::Decrease => all[(idx + all.len() - 1) % all.len()],
    }
  }

  /// Map a Y position to the nearest note in the scale
  /// Returns (note_index, octave) where note_index can be fractional for microtonal scales
  pub fn pos_to_scale_note(
    &self,
    y: usize,
    total_rows: usize,
    base_octave: u8,
    root_note: u8,
  ) -> (f32, u8) {
    if total_rows == 0 {
      return (0.0, base_octave);
    }

    // Invert Y so top = higher notes
    let inverted_y = total_rows.saturating_sub(1).saturating_sub(y);

    if *self == ScaleMode::Chromatic {
      // Chromatic: use all notes
      let note_index = (inverted_y % 12) as f32;
      let octave = base_octave + ((inverted_y / 12) as u8);
      return (note_index, octave);
    }

    // For other scales: map Y position to scale degrees
    let intervals = self.intervals();
    let scale_length = intervals.len();

    // Calculate which scale degree and octave
    let scale_degree = inverted_y % scale_length;
    let octave_offset = (inverted_y / scale_length) as u8;
    let interval = intervals[scale_degree];

    // Add the root note (0-11) to find the real pitch class
    let raw_pitch = interval + (root_note % 12) as f32;

    // If the root + interval > 12, it pushes into the next octave
    let final_note_index = raw_pitch % 12.0;
    let extra_octave = (raw_pitch / 12.0).floor() as u8;

    (final_note_index, base_octave + octave_offset + extra_octave)
  }
}
