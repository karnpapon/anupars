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
  let components = initialize_components();

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

  let midi_output_devices: Vec<String> = components
    .midi
    .get_available_devices()
    .into_iter()
    .map(|(name, _)| name)
    .collect();
  let midi_input_devices: Vec<String> = components
    .midi
    .get_available_input_devices()
    .into_iter()
    .map(|(name, _)| name)
    .collect();
  let initial_midi_device = components.midi.out_device_name();

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
    midi_output_devices,
    midi_input_devices,
    initial_midi_device,
    components.synth_pb_shared,
    components.synth_cc_shared,
  )
  .expect("event loop error");
}
