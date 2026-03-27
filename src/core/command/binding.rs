use std::collections::HashMap;

use cursive::event::Event;
use cursive::event::Key;

use super::types::Adjustment;
use super::types::Command;

pub fn get_bindings() -> HashMap<String, Vec<Command>> {
  default_keybindings()
}

#[rustfmt::skip]
fn default_keybindings() -> HashMap<String, Vec<Command>> {
  let mut kb = HashMap::new();

  kb.insert("q".into(), vec![Command::Quit]);
  kb.insert("Space".into(), vec![Command::TogglePlay]);
  kb.insert("Ctrl+b".into(), vec![Command::ShowMenubar]);
  kb.insert("Esc".into(), vec![Command::ToggleInputRegexAndCanvas]);
  kb.insert(">".into(), vec![Command::AdjustBPM(Adjustment::Increase)]);
  kb.insert("<".into(), vec![Command::AdjustBPM(Adjustment::Decrease)]);
  kb.insert("}".into(), vec![Command::AdjustRatio(Adjustment::Increase)]);
  kb.insert("{".into(), vec![Command::AdjustRatio(Adjustment::Decrease)]);
  kb.insert("~".into(), vec![Command::ToggleSpatialKeyboard]);
  kb.insert("Ctrl+f".into(), vec![Command::ToggleForward]);
  kb.insert("Ctrl+r".into(), vec![Command::ToggleReverse]);
  kb.insert("Ctrl+d".into(), vec![Command::ToggleRandom]);
  kb.insert("Ctrl+p".into(), vec![Command::TogglePendulum]);
  kb.insert("Ctrl+a".into(), vec![Command::ToggleArpeggiator]);
  kb.insert("Ctrl+u".into(), vec![Command::ToggleAccumulation]);
  kb.insert("Ctrl+e".into(), vec![Command::ToggleEventOperator]);
  kb.insert("Ctrl+n".into(), vec![Command::ToggleDrainQueue]);
  kb.insert("Ctrl+s".into(), vec![Command::ToggleSweep]);
  kb.insert("Ctrl+o".into(), vec![Command::ToggleDrone]);
  kb.insert("i".into(), vec![Command::MoveDroneLeft]);
  kb.insert("p".into(), vec![Command::MoveDroneRight]);
  kb.insert("I".into(), vec![Command::CycleDroneChannel(Adjustment::Decrease)]);
  kb.insert("P".into(), vec![Command::CycleDroneChannel(Adjustment::Increase)]);
  kb.insert("Ctrl+y".into(), vec![Command::ToggleDynLength]);
  kb.insert("Ctrl+z".into(), vec![Command::ToggleFreeze]);
  kb.insert("Ctrl+t".into(), vec![Command::CycleTilt]);
  kb.insert("Pipe".into(), vec![Command::CycleSweepMode]);
  kb.insert("Shift+Plus".into(), vec![Command::ChangeScaleMode(Adjustment::Increase)]);
  kb.insert("Shift+Underscore".into(), vec![Command::ChangeScaleMode(Adjustment::Decrease)]);
  kb.insert("Equal".into(), vec![Command::ChangeRootNote(Adjustment::Increase)]);
  kb.insert("Minus".into(), vec![Command::ChangeRootNote(Adjustment::Decrease)]);
  kb.insert(")".into(), vec![Command::ChangeScaleModeLeft(Adjustment::Increase)]);
  kb.insert("(".into(), vec![Command::ChangeScaleModeLeft(Adjustment::Decrease)]);
  kb.insert("0".into(), vec![Command::ChangeRootNoteLeft(Adjustment::Increase)]);
  kb.insert("9".into(), vec![Command::ChangeRootNoteLeft(Adjustment::Decrease)]);
  kb
}

fn parse_key(key: &str) -> Event {
  match key {
    "Enter" => Event::Key(Key::Enter),
    "Space" => Event::Char(" ".chars().next().unwrap()),
    "Tab" => Event::Key(Key::Tab),
    "Backspace" => Event::Key(Key::Backspace),
    "Esc" => Event::Key(Key::Esc),
    "Left" => Event::Key(Key::Left),
    "Right" => Event::Key(Key::Right),
    "Up" => Event::Key(Key::Up),
    "Down" => Event::Key(Key::Down),
    "Ins" => Event::Key(Key::Ins),
    "Del" => Event::Key(Key::Del),
    "Home" => Event::Key(Key::Home),
    "End" => Event::Key(Key::End),
    "PageUp" => Event::Key(Key::PageUp),
    "PageDown" => Event::Key(Key::PageDown),
    "PauseBreak" => Event::Key(Key::PauseBreak),
    "NumpadCenter" => Event::Key(Key::NumpadCenter),
    "Plus" => Event::Char('+'),
    "Underscore" => Event::Char('_'),
    "Pipe" => Event::Char('|'),
    "Backslash" => Event::Char('\\'),
    "Equal" => Event::Char('='),
    "Minus" => Event::Char('-'),
    "F0" => Event::Key(Key::F0),
    "F1" => Event::Key(Key::F1),
    "F2" => Event::Key(Key::F2),
    "F3" => Event::Key(Key::F3),
    "F4" => Event::Key(Key::F4),
    "F5" => Event::Key(Key::F5),
    "F6" => Event::Key(Key::F6),
    "F7" => Event::Key(Key::F7),
    "F8" => Event::Key(Key::F8),
    "F9" => Event::Key(Key::F9),
    "F10" => Event::Key(Key::F10),
    "F11" => Event::Key(Key::F11),
    "F12" => Event::Key(Key::F12),
    s => Event::Char(s.chars().next().unwrap()),
  }
}

pub fn parse_keybinding(kb: &str) -> Option<cursive::event::Event> {
  let mut split = kb.split('+');
  if kb != "+" && split.clone().count() == 2 {
    let modifier = split.next().unwrap();
    let key = split.next().unwrap();
    let parsed = parse_key(key);
    if let Event::Key(parsed) = parsed {
      match modifier {
        "Shift" => Some(Event::Shift(parsed)),
        "Alt" => Some(Event::Alt(parsed)),
        "Ctrl" => Some(Event::Ctrl(parsed)),
        _ => None,
      }
    } else if let Event::Char(parsed) = parsed {
      match modifier {
        "Shift" => Some(Event::Char(parsed.to_uppercase().next().unwrap())),
        "Alt" => Some(Event::AltChar(parsed)),
        "Ctrl" => Some(Event::CtrlChar(parsed)),
        _ => None,
      }
    } else {
      None
    }
  } else {
    Some(parse_key(kb))
  }
}
