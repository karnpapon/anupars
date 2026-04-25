#![cfg(not(target_arch = "wasm32"))]

mod app;
mod app_event;
mod app_state;
mod core;
mod terminal;
mod view;
use app::{initialize_components, setup_ui, spawn_background_threads};
use cursive::CursiveExt;
use std::sync::Arc;

fn main() {
  let mut components = initialize_components();
  setup_ui(&mut components);

  let cb_sink = components.cursive.cb_sink().clone();
  spawn_background_threads(
    Arc::clone(&components.last_key_time),
    Arc::clone(&components.current_tempo),
    components.metronome.tx.clone(),
    components.regex_handler,
    components.metronome,
    components.ui_rx,
    cb_sink,
    Arc::clone(&components.sym_state),
  );

  components.midi.run();
  components.cursive.run();
}
