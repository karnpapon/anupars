mod app;
mod core;
mod view;

use app::{initialize_components, setup_ui, spawn_background_threads};
use cursive::CursiveExt;
use std::sync::Arc;

fn main() {
  let mut components = initialize_components();
  setup_ui(&mut components);

  spawn_background_threads(
    Arc::clone(&components.last_key_time),
    Arc::clone(&components.current_tempo),
    components.metronome.tx.clone(),
    components.regex_handler,
    components.metronome,
  );

  components.marker.run();
  components.midi.run();
  components.cursive.run();
}
