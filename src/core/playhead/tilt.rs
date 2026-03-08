use std::fmt;

/// Sweep tilt mode: controls which column the crosshair sweeps per row
/// when sweep mode is active and channels > 4.
///
/// - `Vertical`  ("|"): same column for every row (default, current behaviour)
/// - `DiagDown`  ("\"): column shifts right (+1) for each row below playhead,
///   shifts left (−1) for each row above playhead
/// - `DiagUp`    ("/"): column shifts left (−1) for each row below playhead,
///   shifts right (+1) for each row above playhead
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TiltMode {
  #[default]
  Vertical,
  DiagDown,
  DiagUp,
}

impl fmt::Display for TiltMode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      TiltMode::Vertical => write!(f, "|"),
      TiltMode::DiagDown => write!(f, "\\"),
      TiltMode::DiagUp => write!(f, "/"),
    }
  }
}

impl TiltMode {
  /// Cycle to the next tilt mode in order: | → \ → / → |
  pub fn cycle_next(self) -> Self {
    match self {
      TiltMode::Vertical => TiltMode::DiagUp,
      TiltMode::DiagDown => TiltMode::Vertical,
      TiltMode::DiagUp => TiltMode::DiagDown,
    }
  }

  pub fn print_tilts(&self) -> String {
    let modes = [TiltMode::DiagDown, TiltMode::Vertical, TiltMode::DiagUp];
    modes
      .iter()
      .map(|m| {
        if m == self {
          format!("{}", m)
        } else {
          // format!("[{}]", m)
          "".to_string()
        }
      })
      .collect::<Vec<_>>()
      .join("")
  }

  /// Given current playhead column, the current row, and the playhead row,
  /// return the effective sweep column, or `None` if the diagonal falls outside
  /// [0, v-1] (meaning that row should not be highlighted).
  /// For `Vertical` tilt the playhead column is always valid.
  /// Only applies when total channels (v * h) > 4.
  pub fn sweep_col_for_row(
    &self,
    playhead_col: usize,
    playhead_row: usize,
    current_row: usize,
    v: usize,
    h: usize,
  ) -> Option<usize> {
    if v * h <= 4 {
      return Some(playhead_col);
    }
    let delta = current_row as isize - playhead_row as isize;
    let offset = match self {
      TiltMode::Vertical => 0,
      TiltMode::DiagDown => delta,
      TiltMode::DiagUp => -delta,
    };
    let col = playhead_col as isize + offset;
    if col < 0 || col >= v as isize {
      None
    } else {
      Some(col as usize)
    }
  }
}
