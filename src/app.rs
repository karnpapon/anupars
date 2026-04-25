use std::fmt;

use crate::core::command::register::CommandManager;
use crate::core::consts;
use crate::core::engine::regex::RegExpHandler;
use crate::core::io::midi;
use crate::core::playhead::{Message as PlayheadMessage, Playhead, UIUpdate};
use crate::core::timing::metronome::{Message, Metronome};
use crate::view::menubar::{set_contents, Menubar};
#[cfg(target_arch = "wasm32")]
use cursive::theme::{BaseColor, Color, PaletteColor};
use cursive::theme::{BorderStyle, Palette};
#[cfg(target_arch = "wasm32")]
use cursive::views::Dialog;
use cursive::views::TextView;
use cursive::Cursive;
use num_rational::Ratio;
use num_traits::FromPrimitive;
use std::rc::Rc;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use std::time::Instant;

use consts::{DEFAULT_TEMPO, MANIFESTO_TEXT, TEMPO_CHECK_INTERVAL_MS, TEMPO_RESET_DELAY_MS};

use crate::view::layout::Program;

pub type UserData = Rc<UserDataInner>;
pub struct UserDataInner {
  pub cmd: CommandManager,
  pub midi_tx: Sender<midi::Message>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum AppMode {
  Arpeggiator,   // a
  DrainQueue,    // n
  Accumulation,  // u
  EventOperator, // e
  Sweep,         // s
  DynLength,     // y
  Freeze,        // z
  Drone,         // o
  None,
}

impl fmt::Display for AppMode {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      AppMode::Arpeggiator => write!(f, "a"),
      AppMode::Accumulation => write!(f, "u"),
      AppMode::EventOperator => write!(f, "e"),
      AppMode::DrainQueue => write!(f, "n"),
      AppMode::Sweep => write!(f, "s"),
      AppMode::DynLength => write!(f, "y"),
      AppMode::Freeze => write!(f, "z"),
      AppMode::Drone => write!(f, "o"),
      AppMode::None => write!(f, "-"),
    }
  }
}

const APP_MODE_ORDER: [AppMode; 8] = [
  AppMode::Arpeggiator,
  AppMode::DrainQueue,
  AppMode::Accumulation,
  AppMode::EventOperator,
  AppMode::Sweep,
  AppMode::DynLength,
  AppMode::Freeze,
  AppMode::Drone,
];

impl AppMode {
  pub fn print_modes(&self) -> String {
    APP_MODE_ORDER.iter().map(|mode| mode.to_string()).collect()
  }

  /// Print a string representing the activated modes, with active modes in uppercase.
  pub fn print_activated_modes_from_vec(active_modes: &[AppMode]) -> String {
    APP_MODE_ORDER
      .iter()
      .map(|mode| {
        if active_modes.contains(mode) {
          mode.to_string().to_ascii_uppercase()
        } else {
          mode.to_string()
        }
      })
      .collect::<Vec<_>>()
      .join("")
  }
}

/// Application components bundle
pub struct Application {
  pub cursive: Cursive,
  pub midi: midi::Midi,
  pub regex_handler: RegExpHandler,
  pub program: Program,
  pub playhead_tx: Sender<PlayheadMessage>,
  pub metronome: Metronome,
  /// for tracking holding keypress
  pub last_key_time: Arc<Mutex<Option<Instant>>>,
  pub current_tempo: Arc<Mutex<usize>>,
  pub ui_tx: Sender<UIUpdate>,
  #[cfg(not(target_arch = "wasm32"))]
  pub sym_state: std::sync::Arc<crate::core::engine::symspell::SymSpellState>,

  // WASM-only: the Arc<Playhead> so wasm.rs can call wasm_tick()
  #[cfg(target_arch = "wasm32")]
  pub playhead: std::sync::Arc<Playhead>,

  pub ui_rx: std::sync::mpsc::Receiver<UIUpdate>,
}

/// Initialize the default cursive theme with simple borders and terminal colors
fn init_cursive_theme(cursive: &mut Cursive) {
  #[allow(unused_mut)] // mut is required by the wasm32 cfg block below
  let mut palette = Palette::terminal_default();

  // In WASM the backend stores raw Color values - TerminalDefault is emitted
  // as \x1b[39m/\x1b[49m which xterm.js renders identically for all cells,
  // making highlighted cells indistinguishable from normal ones.
  // Set explicit colors so the highlight contrast is visible.
  #[cfg(target_arch = "wasm32")]
  {
    palette[PaletteColor::Highlight] = Color::Dark(BaseColor::White);
    palette[PaletteColor::HighlightText] = Color::Dark(BaseColor::Black);
    palette[PaletteColor::HighlightInactive] = Color::Dark(BaseColor::White);
  }

  cursive.set_theme(cursive::theme::Theme {
    shadow: false,
    borders: BorderStyle::Simple,
    palette,
  });
}

/// Initialize all application components
pub fn initialize_components() -> Application {
  let mut cursive = Cursive::new();
  init_cursive_theme(&mut cursive);

  let mut midi = midi::Midi::new();
  midi.init().unwrap();

  let last_key_time = Arc::new(Mutex::new(None));
  let current_tempo = Arc::new(Mutex::new(DEFAULT_TEMPO));
  let prog = Program::new();

  let (ui_tx, ui_rx) = std::sync::mpsc::channel::<UIUpdate>();

  let playhead_area = std::sync::Arc::new(Playhead::new(midi.tx.clone(), ui_tx.clone()));

  #[cfg(not(target_arch = "wasm32"))]
  let sym_state = std::sync::Arc::clone(&playhead_area.sym_state);

  #[cfg(not(target_arch = "wasm32"))]
  let playhead_tx = std::sync::Arc::clone(&playhead_area).run();

  #[cfg(target_arch = "wasm32")]
  let playhead_tx = std::sync::Arc::clone(&playhead_area).wasm_setup();

  let regex_handler = RegExpHandler::new(ui_tx.clone(), playhead_tx.clone());
  let mut metronome = Metronome::new(ui_tx.clone(), playhead_tx.clone());

  metronome.set_midi_tx(midi.tx.clone());
  midi.enable_clock(false);

  // fwd incoming MIDI clock bytes to the metronome for external sync.
  // The closure captures only a Sender<metronome::Message> so there is no
  // circular module dependency between `midi` and `metronome`.
  let metro_tx_for_midi = metronome.tx.clone();
  midi.set_ext_clock_handler(move |byte| {
    let _ = metro_tx_for_midi.send(Message::ExternalClock(byte));
  });

  let midi_tx = midi.tx.clone();

  cursive.set_on_pre_event_inner(cursive::event::Event::CtrlChar('c'), move |_| {
    let _ = midi_tx.send(midi::Message::ClearMsgConfig());
    let _ = midi_tx.send(midi::Message::Panic());
    None
  });

  Application {
    cursive,
    midi,
    regex_handler,
    program: prog,
    playhead_tx,
    metronome,
    last_key_time,
    current_tempo,
    ui_tx,
    #[cfg(not(target_arch = "wasm32"))]
    sym_state,
    #[cfg(target_arch = "wasm32")]
    playhead: playhead_area,
    ui_rx,
  }
}

/// Setup the user interface, menus, and views
pub fn setup_ui(components: &mut Application) {
  let midi_tx = components.midi.tx.clone();
  let playhead_tx = components.playhead_tx.clone();
  let metronome_tx = components.metronome.tx.clone();

  let mut command_manager = CommandManager::new(
    components.program.clone(),
    metronome_tx.clone(),
    components.ui_tx.clone(),
    Arc::clone(&components.current_tempo),
    Arc::clone(&components.last_key_time),
    playhead_tx.clone(),
  );

  command_manager.register_all();
  command_manager.register_keybindings(&mut components.cursive);

  components.cursive.set_autohide_menu(true);
  components.cursive.set_autorefresh(false); // Prevent unintended events

  components.cursive.set_user_data(Rc::new(UserDataInner {
    cmd: command_manager,
    midi_tx: midi_tx.clone(),
  }));

  let main_view = components
    .program
    .build(components.regex_handler.tx.clone(), playhead_tx);

  let devices = components.midi.get_available_devices();
  let menu_app = Menubar::build_menu_app(&devices, midi_tx.clone());
  let menu_view = Menubar::build_menu_view();
  let menu_help = Menubar::build_menu_help();

  components
    .cursive
    .menubar()
    .add_subtree("anupars", menu_app)
    .add_subtree("view", menu_view)
    .add_subtree("help", menu_help)
    .add_delimiter()
    .add_leaf("quit", |s| {
      #[cfg(not(target_arch = "wasm32"))]
      s.quit();
      #[cfg(target_arch = "wasm32")]
      s.add_layer(
        Dialog::info(
          "\"quit\" is only available in the terminal-based app.\n\nclose the browser tab to exit.",
        )
        .title("quit"),
      );
    });

  components.cursive.add_layer(main_view);

  // Load the manifesto text automatically on startup. Queued via cb_sink so
  // it runs after the first layout pass, when grid dimensions are known.
  components
    .cursive
    .cb_sink()
    .send(Box::new(|siv| {
      set_contents(siv, MANIFESTO_TEXT.to_string());
    }))
    .unwrap();

  components
    .cursive
    .call_on_name(consts::midi_status_unit_view, |view: &mut TextView| {
      view.set_content(components.midi.out_device_name());
    })
    .unwrap();
}

/// Spawn a background thread to monitor key press timing and reset tempo
#[cfg(not(target_arch = "wasm32"))]
fn spawn_tempo_monitor_thread(
  last_key_time: Arc<Mutex<Option<Instant>>>,
  current_tempo: Arc<Mutex<usize>>,
  metronome_tx: Sender<Message>,
) {
  thread::Builder::new()
    .name(consts::THREAD_NAME_TEMPO_MONITOR.to_string())
    .spawn(move || loop {
      thread::sleep(Duration::from_millis(TEMPO_CHECK_INTERVAL_MS));

      let mut last_press = last_key_time.lock().unwrap();
      if let Some(last_time) = *last_press {
        if last_time.elapsed() > Duration::from_millis(TEMPO_RESET_DELAY_MS) {
          *last_press = None;
          let tempo = *current_tempo.lock().unwrap();
          let _ = metronome_tx.send(Message::Tempo(
            Ratio::from_i64(tempo.try_into().unwrap()).unwrap(),
          ));
        }
      }
    })
    .expect("Failed to spawn tempo monitor thread");
}

/// Spawn all background worker threads
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_background_threads(
  last_key_time: Arc<Mutex<Option<Instant>>>,
  current_tempo: Arc<Mutex<usize>>,
  metronome_tx: Sender<Message>,
  regex_handler: RegExpHandler,
  metronome: Metronome,
  ui_rx: std::sync::mpsc::Receiver<UIUpdate>,
  cb_sink: cursive::CbSink,
  sym_state: std::sync::Arc<crate::core::engine::symspell::SymSpellState>,
) {
  spawn_tempo_monitor_thread(last_key_time, current_tempo, metronome_tx);

  let regex_tx = regex_handler.tx.clone();

  thread::Builder::new()
    .name(consts::THREAD_NAME_REGEX_HANDLER.to_string())
    .spawn(move || regex_handler.run())
    .expect("Failed to spawn regex handler thread");

  thread::Builder::new()
    .name(consts::THREAD_NAME_METRONOME.to_string())
    .spawn(move || metronome.run())
    .expect("Failed to spawn metronome thread");

  Playhead::spawn_ui_processor(ui_rx, cb_sink, sym_state, regex_tx);
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn run_event_loop(
  ui_rx: std::sync::mpsc::Receiver<crate::core::playhead::UIUpdate>,
  renderer: &mut crate::terminal::Renderer,
) -> std::io::Result<()> {
  use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
  use std::time::Duration;

  let mut state = crate::app_state::AppState::default();

  loop {
    if poll(Duration::from_millis(16))? {
      match read()? {
        Event::Key(key) => {
          if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            break;
          }
        }
        Event::Resize(w, h) => {
          renderer.resize(w, h);
          state.resize(w, h);
        }
        Event::Mouse(_) => {}
        _ => {}
      }
    }

    while let Ok(update) = ui_rx.try_recv() {
      crate::app_state::apply_ui_update(update, &mut state);
    }

    // Phase 8: draw_frame(&state, renderer.current_mut())
    renderer.flush(&mut std::io::stdout())?;
  }
  Ok(())
}
