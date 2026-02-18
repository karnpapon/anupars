use crate::core::application::UserDataInner;
use crate::core::clock::metronome::{Message, Metronome};
use crate::core::commands::CommandManager;
use crate::core::consts;
use crate::core::midi::Midi;
use crate::core::regex::RegExpHandler;
use crate::view::common::marker::Marker;
use crate::view::common::menubar::Menubar;
use cursive::theme::{BorderStyle, Palette};
use cursive::views::TextView;
use cursive::Cursive;
use num::rational::Ratio;
use num::FromPrimitive;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use consts::{DEFAULT_TEMPO, TEMPO_CHECK_INTERVAL_MS, TEMPO_RESET_DELAY_MS};

#[cfg(feature = "desktop")]
use crate::view::desktop::anu::Anu;

#[cfg(feature = "microcontroller")]
use crate::view::microcontroller::anu::Anu;

/// Application components bundle
pub struct AppComponents {
  pub cursive: Cursive,
  pub midi: Midi,
  pub regex_handler: RegExpHandler,
  pub anu: Anu,
  pub marker: Marker,
  pub metronome: Metronome,
  pub last_key_time: Arc<Mutex<Option<Instant>>>,
  pub current_tempo: Arc<Mutex<i64>>,
}

/// Initialize the default cursive theme with simple borders and terminal colors
fn init_cursive_theme(cursive: &mut Cursive) {
  let palette = Palette::terminal_default();
  cursive.set_theme(cursive::theme::Theme {
    shadow: false,
    borders: BorderStyle::Simple,
    palette,
  });
}

/// Initialize all application components
pub fn initialize_components() -> AppComponents {
  let mut cursive = Cursive::new();
  init_cursive_theme(&mut cursive);

  let mut midi = Midi::new();
  midi.init().unwrap();

  let regex_handler = RegExpHandler::new(cursive.cb_sink().clone());
  let last_key_time = Arc::new(Mutex::new(None));
  let current_tempo = Arc::new(Mutex::new(DEFAULT_TEMPO));
  let anu = Anu::new();

  let marker = Marker::new(cursive.cb_sink().clone(), midi.tx.clone());
  let metronome = Metronome::new(cursive.cb_sink().clone(), marker.tx.clone());

  AppComponents {
    cursive,
    midi,
    regex_handler,
    anu,
    marker,
    metronome,
    last_key_time,
    current_tempo,
  }
}

/// Setup the user interface, menus, and views
pub fn setup_ui(components: &mut AppComponents) {
  let midi_tx = components.midi.tx.clone();
  let marker_tx = components.marker.tx.clone();
  let metronome_tx = components.metronome.tx.clone();

  // Initialize command manager
  let mut command_manager = CommandManager::new(
    components.anu.clone(),
    metronome_tx.clone(),
    components.cursive.cb_sink().clone(),
    Arc::clone(&components.current_tempo),
    Arc::clone(&components.last_key_time),
    marker_tx.clone(),
  );

  command_manager.register_all();
  command_manager.register_keybindings(&mut components.cursive);

  // Configure cursive settings
  components.cursive.set_autohide_menu(true);
  components.cursive.set_autorefresh(false); // Prevent unintended events

  components.cursive.set_user_data(Rc::new(UserDataInner {
    cmd: command_manager,
    midi_tx: midi_tx.clone(),
  }));

  // Build main view
  let main_view = components
    .anu
    .build(components.regex_handler.tx.clone(), marker_tx);

  // Build menu system
  let devices = components.midi.get_available_devices();
  let menu_app = Menubar::build_menu_app(&devices, midi_tx.clone());
  let menu_help = Menubar::build_menu_help();

  components
    .cursive
    .menubar()
    .add_subtree("Anu", menu_app)
    .add_subtree("Help", menu_help)
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  components.cursive.add_layer(main_view);

  // Update MIDI status display
  components
    .cursive
    .call_on_name(consts::midi_status_unit_view, |view: &mut TextView| {
      view.set_content(components.midi.out_device_name());
    })
    .unwrap();
}

/// Spawn a background thread to monitor key press timing and reset tempo
fn spawn_tempo_monitor_thread(
  last_key_time: Arc<Mutex<Option<Instant>>>,
  current_tempo: Arc<Mutex<i64>>,
  metronome_tx: Sender<Message>,
) {
  thread::Builder::new()
    .name("tempo-monitor".to_string())
    .spawn(move || loop {
      thread::sleep(Duration::from_millis(TEMPO_CHECK_INTERVAL_MS));

      let mut last_press = last_key_time.lock().unwrap();
      if let Some(last_time) = *last_press {
        if last_time.elapsed() > Duration::from_millis(TEMPO_RESET_DELAY_MS) {
          *last_press = None;
          let tempo = *current_tempo.lock().unwrap();
          let _ = metronome_tx.send(Message::Tempo(Ratio::from_i64(tempo).unwrap()));
        }
      }
    })
    .expect("Failed to spawn tempo monitor thread");
}

/// Spawn all background worker threads
pub fn spawn_background_threads(
  last_key_time: Arc<Mutex<Option<Instant>>>,
  current_tempo: Arc<Mutex<i64>>,
  metronome_tx: Sender<Message>,
  regex_handler: RegExpHandler,
  metronome: Metronome,
) {
  spawn_tempo_monitor_thread(last_key_time, current_tempo, metronome_tx);

  thread::Builder::new()
    .name("regex-handler".to_string())
    .spawn(move || regex_handler.run())
    .expect("Failed to spawn regex handler thread");

  thread::Builder::new()
    .name("metronome".to_string())
    .spawn(move || metronome.run())
    .expect("Failed to spawn metronome thread");
}
