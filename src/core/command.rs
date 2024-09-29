// use crate::queue::RepeatSetting;
// use crate::spotify_url::SpotifyUrl;
use std::collections::HashMap;
use std::fmt;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
// use strum_macros::Display;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SeekInterval {
  Forward,
  Backwards,
  Custom(usize),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum TargetMode {
  Current,
  Selected,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum MoveMode {
  Up,
  Down,
  Left,
  Right,
  Playing,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum MoveAmount {
  Integer(i32),
  Float(f32),
  Extreme,
}

impl Default for MoveAmount {
  fn default() -> Self {
    Self::Integer(1)
  }
}

/// Keys that can be used to sort songs on.
#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum SortKey {
  Title,
  Duration,
  Artist,
  Album,
  Added,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum SortDirection {
  Ascending,
  Descending,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum JumpMode {
  Previous,
  Next,
  Query(String),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum ShiftMode {
  Up,
  Down,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
// #[strum(serialize_all = "lowercase")]
pub enum GotoMode {
  Album,
  Artist,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum SeekDirection {
  Relative(i32),
  Absolute(u32),
}

impl fmt::Display for SeekDirection {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let repr = match self {
      Self::Absolute(pos) => format!("{pos}"),
      Self::Relative(delta) => {
        format!("{}{}", if delta > &0 { "+" } else { "" }, delta)
      }
    };
    write!(f, "{repr}")
  }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum InsertSource {
  #[cfg(feature = "share_clipboard")]
  Clipboard,
  // Input(SpotifyUrl),
}

// impl fmt::Display for InsertSource {
//   fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//     let repr = match self {
//       #[cfg(feature = "share_clipboard")]
//       Self::Clipboard => "".into(),
//       // Self::Input(url) => url.to_string(),
//     };
//     write!(f, "{repr}")
//   }
// }

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum Command {
  Quit,
  TogglePlay,
  ShowMenubar,
  ToggleInputRegexAndCanvas,
}

impl fmt::Display for Command {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let mut repr_tokens = vec![self.basename().to_owned()];
    let mut extras_args = match self {
      Self::Quit | Self::ToggleInputRegexAndCanvas | Self::ShowMenubar | Self::TogglePlay => vec![],
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
    }
  }
}

fn register_aliases(map: &mut HashMap<&str, &str>, cmd: &'static str, names: Vec<&'static str>) {
  for a in names {
    map.insert(a, cmd);
  }
}

fn handle_aliases(input: &str) -> &str {
  // NOTE: There is probably a better way to write this than a static HashMap. The HashMap doesn't
  // improve performance as there's far too few keys, and the use of static doesn't seem good.
  static ALIASES: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

  let aliases = ALIASES.get_or_init(|| {
    let mut m = HashMap::new();

    register_aliases(&mut m, "quit", vec!["q", "x"]);
    register_aliases(
      &mut m,
      "playpause",
      vec!["pause", "toggleplay", "toggleplayback"],
    );
    register_aliases(&mut m, "repeat", vec!["loop"]);
    m
  });

  if let Some(cmd) = aliases.get(input) {
    handle_aliases(cmd)
  } else {
    input
  }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum CommandParseError {
  NoSuchCommand {
    cmd: String,
  },
  InsufficientArgs {
    cmd: String,
    hint: Option<String>,
  },
  BadEnumArg {
    arg: String,
    accept: Vec<String>,
    optional: bool,
  },
  ArgParseError {
    arg: String,
    err: String,
  },
}

impl fmt::Display for CommandParseError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let formatted = match self {
      Self::NoSuchCommand { cmd } => format!("No such command \"{cmd}\""),
      Self::InsufficientArgs { cmd, hint } => {
        if let Some(hint_str) = hint {
          format!("\"{cmd}\" requires additional arguments: {hint_str}")
        } else {
          format!("\"{cmd}\" requires additional arguments")
        }
      }
      Self::BadEnumArg {
        arg,
        accept,
        optional,
      } => {
        let accept = accept.join("|");
        if *optional {
          format!("Argument \"{arg}\" should be one of {accept} or be omitted")
        } else {
          format!("Argument \"{arg}\" should be one of {accept}")
        }
      }
      Self::ArgParseError { arg, err } => format!("Error with argument \"{arg}\": {err}"),
    };
    write!(f, "{formatted}")
  }
}

pub fn parse(input: &str) -> Result<Vec<Command>, CommandParseError> {
  let mut command_inputs = vec!["".to_string()];
  let mut command_idx = 0;
  enum ParseState {
    Normal,
    SeparatorEncountered,
  }
  let mut parse_state = ParseState::Normal;
  for c in input.chars() {
    let is_separator = c == ';';
    match parse_state {
      ParseState::Normal if is_separator => parse_state = ParseState::SeparatorEncountered,
      ParseState::Normal => command_inputs[command_idx].push(c),
      // ";" is escaped using ";;", so if the previous char already was a ';' push a ';'.
      ParseState::SeparatorEncountered if is_separator => {
        command_inputs[command_idx].push(c);
        parse_state = ParseState::Normal;
      }
      ParseState::SeparatorEncountered => {
        command_idx += 1;
        command_inputs.push(c.to_string());
        parse_state = ParseState::Normal;
      }
    }
  }

  let mut commands = vec![];
  for command_input in command_inputs {
    let components: Vec<_> = command_input.split_whitespace().collect();

    if let Some((command, _args)) = components.split_first() {
      let command = handle_aliases(command);
      use CommandParseError as E;
      let command = match command {
        "quit" => Command::Quit,
        _ => {
          return Err(E::NoSuchCommand {
            cmd: command.into(),
          })
        }
      };
      commands.push(command);
    };
  }
  Ok(commands)
}
