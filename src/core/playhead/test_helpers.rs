use crate::core::io::midi;

use super::{Playhead, UIUpdate};

pub fn make_playhead() -> Playhead {
  let (midi_tx, _midi_rx) = std::sync::mpsc::channel::<midi::Message>();
  let (ui_tx, _ui_rx) = std::sync::mpsc::channel::<UIUpdate>();
  Playhead::new(midi_tx, ui_tx)
}
