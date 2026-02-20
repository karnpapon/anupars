use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum MoveDirection {
  Up,
  Down,
  Left,
  Right,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum Adjustment {
  Increase,
  Decrease,
}

#[derive(Clone, Serialize, Deserialize, Debug, Copy)]
pub enum Command {
  Quit,
  TogglePlay,
  ShowMenubar,
  ToggleInputRegexAndCanvas,
  AdjustMarker(MoveDirection),
  AdjustBPM(Adjustment),
  AdjustRatio(Adjustment),
  ToggleReverse,
  ToggleArpeggiator,
  ToggleAccumulation,
}

impl fmt::Display for Command {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut repr_tokens = vec![self.basename().to_owned()];
    let mut extras_args = match self {
      Self::Quit
      | Self::ToggleInputRegexAndCanvas
      | Self::ShowMenubar
      | Self::TogglePlay
      | Self::AdjustBPM(_)
      | Self::AdjustRatio(_)
      | Self::AdjustMarker(_)
      | Self::ToggleReverse
      | Self::ToggleArpeggiator
      | Self::ToggleAccumulation => vec![],
    };
    repr_tokens.append(&mut extras_args);
    write!(f, "{}", repr_tokens.join(" "))
  }
}

impl Command {
  pub fn basename(&self) -> &str {
    match self {
      Self::Quit => "quit",
      Self::TogglePlay => "playpause",
      Self::ShowMenubar => "showmenubar",
      Self::ToggleInputRegexAndCanvas => "toggleinputregexandcanvas",
      Self::AdjustMarker(_) => "adjustmarker",
      Self::AdjustBPM(_) => "adjustbpm",
      Self::AdjustRatio(_) => "adjustratio",
      Self::ToggleReverse => "togglereverse",
      Self::ToggleArpeggiator => "togglearpeggiator",
      Self::ToggleAccumulation => "toggleaccumulation",
    }
  }
}
