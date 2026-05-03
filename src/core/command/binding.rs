use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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
  kb.insert("!".into(), vec![Command::ToggleSweepMovement]);
  kb.insert("Ctrl+o".into(), vec![Command::ToggleDrone]);
  kb.insert("i".into(), vec![Command::MoveDroneLeft]);
  kb.insert("p".into(), vec![Command::MoveDroneRight]);
  kb.insert("I".into(), vec![Command::CycleDroneChannel(Adjustment::Decrease)]);
  kb.insert("P".into(), vec![Command::CycleDroneChannel(Adjustment::Increase)]);
  kb.insert("Ctrl+y".into(), vec![Command::ToggleDynLength]);
  kb.insert("Ctrl+z".into(), vec![Command::ToggleFreeze]);
  kb.insert("Ctrl+t".into(), vec![Command::CycleTilt]);
  kb.insert("$".into(), vec![Command::CycleEasing]);
  kb.insert("Pipe".into(), vec![Command::CycleSweepMode]);
  kb.insert("@".into(), vec![Command::CycleSweepOutputMode]);
  kb.insert("[".into(), vec![Command::AdjustSweepCC(Adjustment::Decrease)]);
  kb.insert("]".into(), vec![Command::AdjustSweepCC(Adjustment::Increase)]);
  kb.insert("Shift+Plus".into(), vec![Command::ChangeScaleMode(Adjustment::Increase)]);
  kb.insert("Shift+Underscore".into(), vec![Command::ChangeScaleMode(Adjustment::Decrease)]);
  kb.insert("Equal".into(), vec![Command::ChangeRootNote(Adjustment::Increase)]);
  kb.insert("Minus".into(), vec![Command::ChangeRootNote(Adjustment::Decrease)]);
  kb
}

#[cfg(not(target_arch = "wasm32"))]
fn parse_key_code(key: &str) -> Option<KeyCode> {
  match key {
    "Enter" => Some(KeyCode::Enter),
    "Space" => Some(KeyCode::Char(' ')),
    "Tab" => Some(KeyCode::Tab),
    "Backspace" => Some(KeyCode::Backspace),
    "Esc" => Some(KeyCode::Esc),
    "Left" => Some(KeyCode::Left),
    "Right" => Some(KeyCode::Right),
    "Up" => Some(KeyCode::Up),
    "Down" => Some(KeyCode::Down),
    "Del" | "Delete" => Some(KeyCode::Delete),
    "Home" => Some(KeyCode::Home),
    "End" => Some(KeyCode::End),
    "PageUp" => Some(KeyCode::PageUp),
    "PageDown" => Some(KeyCode::PageDown),
    "Plus" => Some(KeyCode::Char('+')),
    "Underscore" => Some(KeyCode::Char('_')),
    "Pipe" => Some(KeyCode::Char('|')),
    "Backslash" => Some(KeyCode::Char('\\')),
    "Equal" => Some(KeyCode::Char('=')),
    "Minus" => Some(KeyCode::Char('-')),
    "F1" => Some(KeyCode::F(1)),
    "F2" => Some(KeyCode::F(2)),
    "F3" => Some(KeyCode::F(3)),
    "F4" => Some(KeyCode::F(4)),
    "F5" => Some(KeyCode::F(5)),
    "F6" => Some(KeyCode::F(6)),
    "F7" => Some(KeyCode::F(7)),
    "F8" => Some(KeyCode::F(8)),
    "F9" => Some(KeyCode::F(9)),
    "F10" => Some(KeyCode::F(10)),
    "F11" => Some(KeyCode::F(11)),
    "F12" => Some(KeyCode::F(12)),
    s if s.chars().count() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
    _ => None,
  }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn parse_keybinding(kb: &str) -> Option<KeyEvent> {
  let mut split = kb.split('+');
  if kb != "+" && split.clone().count() == 2 {
    let modifier = split.next().unwrap();
    let key = split.next().unwrap();
    let code = parse_key_code(key)?;
    let (mods, code) = match (modifier, code) {
      ("Ctrl", KeyCode::Char(c)) => (KeyModifiers::CONTROL, KeyCode::Char(c)),
      ("Alt", KeyCode::Char(c)) => (KeyModifiers::ALT, KeyCode::Char(c)),
      ("Shift", KeyCode::Char(c)) => (KeyModifiers::SHIFT, KeyCode::Char(c.to_ascii_uppercase())),
      ("Ctrl", k) => (KeyModifiers::CONTROL, k),
      ("Alt", k) => (KeyModifiers::ALT, k),
      ("Shift", k) => (KeyModifiers::SHIFT, k),
      _ => return None,
    };
    Some(KeyEvent::new(code, mods))
  } else {
    let code = parse_key_code(kb)?;
    Some(KeyEvent::new(code, KeyModifiers::NONE))
  }
}
