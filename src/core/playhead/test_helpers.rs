use crate::core::io::midi;

use super::{Playhead, UIUpdate};

pub fn make_playhead() -> Playhead {
  let (midi_tx, _midi_rx) = std::sync::mpsc::channel::<midi::Message>();
  let (ui_tx, _ui_rx) = std::sync::mpsc::channel::<UIUpdate>();
  let synth_pb = crate::app_state::make_synth_bufs_shared();
  let synth_cc = crate::app_state::make_synth_cc_shared();
  Playhead::new(midi_tx, ui_tx, synth_pb, synth_cc)
}
