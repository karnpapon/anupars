mod app;
mod core;
mod view;
use app::{initialize_components, setup_ui, spawn_background_threads};
use cursive::CursiveExt;
use std::sync::Arc;

use crate::core::midi::Message;

fn main() {
  let mut components = initialize_components();
  setup_ui(&mut components);

  let midi_clone = components.midi.tx.clone();

  spawn_background_threads(
    Arc::clone(&components.last_key_time),
    Arc::clone(&components.current_tempo),
    components.metronome.tx.clone(),
    components.regex_handler,
    components.metronome,
  );

  // TODO: still not working
  ctrlc::set_handler(move || {
    let _ = midi_clone.send(Message::ClearMsgConfig());
    let _ = midi_clone.send(Message::Panic());
    std::process::exit(0);
  })
  .expect("Error setting Ctrl-C handler");

  components.playhead.run();
  components.midi.run();
  components.cursive.run();
}
