/// Musical scale modes and their interval patterns
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScaleMode {
  Chromatic,     // All 12 notes
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
}

impl ScaleMode {
  /// Get the intervals (in semitones from root) for this scale mode
  /// Returns which notes (0-11) are included in the scale
  pub fn intervals(&self) -> &'static [u8] {
    match self {
      ScaleMode::Chromatic => &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
      ScaleMode::Major => &[0, 2, 4, 5, 7, 9, 11],
      ScaleMode::Minor => &[0, 2, 3, 5, 7, 8, 10],
      ScaleMode::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
      ScaleMode::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
      ScaleMode::Dorian => &[0, 2, 3, 5, 7, 9, 10],
      ScaleMode::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
      ScaleMode::Lydian => &[0, 2, 4, 6, 7, 9, 11],
      ScaleMode::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
      ScaleMode::Locrian => &[0, 1, 3, 5, 6, 8, 10],
      ScaleMode::MajorPentatonic => &[0, 2, 4, 7, 9],
      ScaleMode::MinorPentatonic => &[0, 3, 5, 7, 10],
      ScaleMode::Blues => &[0, 3, 5, 6, 7, 10],
      ScaleMode::WholeTone => &[0, 2, 4, 6, 8, 10],
      ScaleMode::Diminished => &[0, 2, 3, 5, 6, 8, 9, 11],
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
    }
  }

  /// Check if a note index (0-11) is in this scale
  pub fn contains_note(&self, note_index: u8) -> bool {
    self.intervals().contains(&(note_index % 12))
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
    ]
  }

  /// Map a Y position to the nearest note in the scale
  /// Returns (note_index, octave) where note_index is 0-11
  pub fn y_to_scale_note(&self, y: usize, total_rows: usize, base_octave: u8) -> (u8, u8) {
    if total_rows == 0 {
      return (0, base_octave);
    }

    // Invert Y so top = higher notes
    let inverted_y = total_rows.saturating_sub(1).saturating_sub(y);

    if *self == ScaleMode::Chromatic {
      // Chromatic: use all notes
      let note_index = (inverted_y % 12) as u8;
      let octave = base_octave + ((inverted_y / 12) as u8);
      return (note_index, octave);
    }

    // For other scales: map Y position to scale degrees
    let intervals = self.intervals();
    let scale_length = intervals.len();

    // Calculate which scale degree and octave
    let scale_degree = inverted_y % scale_length;
    let octave_offset = inverted_y / scale_length;

    let note_index = intervals[scale_degree];
    let octave = base_octave + (octave_offset as u8);

    (note_index, octave)
  }
}

impl Default for ScaleMode {
  fn default() -> Self {
    ScaleMode::Chromatic
  }
}
