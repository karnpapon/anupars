use std::fmt;

#[derive(Clone, Debug, Copy)]
pub enum Adjustment {
  Increase,
  Decrease,
}

#[derive(Clone, Debug, Copy)]
pub enum Command {
  Quit,
  TogglePlay,
  ShowMenubar,
  ToggleInputRegexAndCanvas,
  AdjustBPM(Adjustment),
  AdjustRatio(Adjustment),
  ToggleForward,
  ToggleReverse,
  ToggleArpeggiator,
  ToggleAccumulation,
  ToggleRandom,
  TogglePendulum,
  ToggleEventOperator,
  ToggleDrainQueue,
  ToggleSweep,
  ToggleDrone,
  MoveDroneLeft,
  MoveDroneRight,
  CycleDroneChannel(Adjustment),
  ToggleDynLength,
  ToggleFreeze,
  ChangeRootNote(Adjustment),
  ChangeScaleMode(Adjustment),
  CycleTilt,
  CycleSweepMode,
  ToggleSpatialKeyboard,
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
      | Self::ToggleForward
      | Self::ToggleReverse
      | Self::TogglePendulum
      | Self::ToggleArpeggiator
      | Self::ToggleAccumulation
      | Self::ToggleEventOperator
      | Self::ToggleDrainQueue
      | Self::ToggleSweep
      | Self::ToggleDrone
      | Self::MoveDroneLeft
      | Self::MoveDroneRight
      | Self::CycleDroneChannel(_)
      | Self::ToggleDynLength
      | Self::ChangeRootNote(_)
      | Self::ChangeScaleMode(_)
      | Self::CycleTilt
      | Self::CycleSweepMode
      | Self::ToggleFreeze
      | Self::ToggleRandom
      | Self::ToggleSpatialKeyboard => vec![],
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
      Self::AdjustBPM(_) => "adjustbpm",
      Self::AdjustRatio(_) => "adjustratio",
      Self::ToggleForward => "toggleforward",
      Self::ToggleReverse => "togglereverse",
      Self::TogglePendulum => "togglependulum",
      Self::ToggleArpeggiator => "togglearpeggiator",
      Self::ToggleAccumulation => "toggleaccumulation",
      Self::ToggleRandom => "togglerandom",
      Self::ToggleEventOperator => "toggleeventoperator",
      Self::ToggleDrainQueue => "toggledrainqueue",
      Self::ToggleSweep => "togglesweep",
      Self::ToggleDrone => "toggledrone",
      Self::MoveDroneLeft => "droneLeft",
      Self::MoveDroneRight => "droneRight",
      Self::CycleDroneChannel(_) => "cycledronechannel",
      Self::ToggleDynLength => "toggledynlength",
      Self::ChangeRootNote(_) => "changerootnote",
      Self::ChangeScaleMode(_) => "changescale",
      Self::CycleTilt => "cycletilt",
      Self::CycleSweepMode => "cyclesweepmode",
      Self::ToggleFreeze => "togglefreeze",
      Self::ToggleSpatialKeyboard => "togglekeyboard",
    }
  }
}
