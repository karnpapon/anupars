#![cfg(not(target_arch = "wasm32"))]

mod app;
mod app_event;
mod app_state;
mod core;
mod terminal;
mod view;
use app::{initialize_components, spawn_background_threads};
use std::sync::Arc;

fn main() {
  let mut components = initialize_components();

  let sym_state = Arc::clone(&components.sym_state);
  let regex_tx = components.regex_tx.clone();
  let midi_tx = components.midi.tx.clone();

  spawn_background_threads(
    Arc::clone(&components.last_key_time),
    Arc::clone(&components.current_tempo),
    components.metronome.tx.clone(),
    components.regex_handler,
    components.metronome,
  );

  components.midi.run();

  let (init_w, init_h) = crossterm::terminal::size().unwrap_or((80, 24));
  let mut renderer = terminal::Renderer::new(init_w, init_h);

  app::run_event_loop(
    components.ui_rx,
    components.playhead_tx,
    &components.cmd_mgr,
    sym_state,
    regex_tx,
    midi_tx,
    &mut renderer,
  )
  .expect("event loop error");
}
