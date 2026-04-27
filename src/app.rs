use std::fmt;

use crate::core::command::register::CommandManager;
use crate::core::consts;
use crate::core::engine::regex::RegExpHandler;
use crate::core::io::midi;
use crate::core::playhead::{Message as PlayheadMessage, Playhead, UIUpdate};
use crate::core::timing::metronome::{Message, Metronome};
use num_rational::Ratio;
use num_traits::FromPrimitive;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;
use std::time::Instant;

use consts::{DEFAULT_TEMPO, TEMPO_CHECK_INTERVAL_MS, TEMPO_RESET_DELAY_MS};

use crate::view::layout::Program;

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
  pub midi: midi::Midi,
  pub regex_handler: RegExpHandler,
  pub regex_tx: Sender<crate::core::engine::regex::Message>,
  pub program: Program,
  pub playhead_tx: Sender<PlayheadMessage>,
  pub metronome: Metronome,
  pub last_key_time: Arc<Mutex<Option<Instant>>>,
  pub current_tempo: Arc<Mutex<usize>>,
  pub ui_tx: Sender<UIUpdate>,
  pub ui_rx: std::sync::mpsc::Receiver<UIUpdate>,
  pub cmd_mgr: CommandManager,
  #[cfg(not(target_arch = "wasm32"))]
  pub sym_state: std::sync::Arc<crate::core::engine::symspell::SymSpellState>,
  #[cfg(target_arch = "wasm32")]
  pub playhead: std::sync::Arc<Playhead>,
}

/// Initialize all application components
pub fn initialize_components() -> Application {
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
  let regex_tx = regex_handler.tx.clone();
  let mut metronome = Metronome::new(ui_tx.clone(), playhead_tx.clone());

  metronome.set_midi_tx(midi.tx.clone());
  midi.enable_clock(false);

  let metro_tx_for_midi = metronome.tx.clone();
  midi.set_ext_clock_handler(move |byte| {
    let _ = metro_tx_for_midi.send(Message::ExternalClock(byte));
  });

  let mut cmd_mgr = CommandManager::new(
    prog.clone(),
    metronome.tx.clone(),
    ui_tx.clone(),
    Arc::clone(&current_tempo),
    Arc::clone(&last_key_time),
    playhead_tx.clone(),
  );
  cmd_mgr.register_all();

  Application {
    midi,
    regex_handler,
    regex_tx,
    program: prog,
    playhead_tx,
    metronome,
    last_key_time,
    current_tempo,
    ui_tx,
    ui_rx,
    cmd_mgr,
    #[cfg(not(target_arch = "wasm32"))]
    sym_state,
    #[cfg(target_arch = "wasm32")]
    playhead: playhead_area,
  }
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
#[cfg(not(target_arch = "wasm32"))]
pub fn spawn_background_threads(
  last_key_time: Arc<Mutex<Option<Instant>>>,
  current_tempo: Arc<Mutex<usize>>,
  metronome_tx: Sender<Message>,
  regex_handler: RegExpHandler,
  metronome: Metronome,
) {
  spawn_tempo_monitor_thread(last_key_time, current_tempo, metronome_tx);

  thread::Builder::new()
    .name(consts::THREAD_NAME_REGEX_HANDLER.to_string())
    .spawn(move || regex_handler.run())
    .expect("Failed to spawn regex handler thread");

  thread::Builder::new()
    .name(consts::THREAD_NAME_METRONOME.to_string())
    .spawn(move || metronome.run())
    .expect("Failed to spawn metronome thread");
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_event_loop(
  ui_rx: std::sync::mpsc::Receiver<crate::core::playhead::UIUpdate>,
  playhead_tx: std::sync::mpsc::Sender<crate::core::playhead::Message>,
  cmd_mgr: &CommandManager,
  sym_state: std::sync::Arc<crate::core::engine::symspell::SymSpellState>,
  regex_tx: std::sync::mpsc::Sender<crate::core::engine::regex::Message>,
  midi_tx: std::sync::mpsc::Sender<midi::Message>,
  renderer: &mut crate::terminal::Renderer,
) -> std::io::Result<()> {
  use crate::app_state::{apply_ui_update, AppState, Focus};
  use crate::view::consts::{CONSOLE_HEIGHT, PADDING_X, PADDING_Y};
  use crate::view::grid::GridEditor;
  use crate::view::menubar::{handle_menu_key, set_grid_contents, MenuAction};
  use crate::view::ui_processor::apply_sym_anim_tick;
  use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
  use std::time::Duration;

  let mut state = AppState::default();
  let mut should_quit = false;

  let (init_w, init_h) = crossterm::terminal::size().unwrap_or((80, 24));
  state.resize(init_w, init_h);

  let mut grid = GridEditor::new(playhead_tx.clone());
  grid.regex_tx = Some(regex_tx.clone());
  let init_grid_h = init_h.saturating_sub(CONSOLE_HEIGHT + 2 + PADDING_Y * 2);
  grid.resize(crate::core::geom::Vec2::new(
    init_w.saturating_sub(PADDING_X * 2) as usize,
    init_grid_h as usize,
  ));

  // Load the manifesto text after initial resize so grid dimensions are known.
  set_grid_contents(&mut grid, consts::MANIFESTO_TEXT.to_string());

  loop {
    if poll(Duration::from_millis(16))? {
      match read()? {
        Event::Key(key) => {
          if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            let _ = midi_tx.send(midi::Message::ClearMsgConfig());
            let _ = midi_tx.send(midi::Message::Panic());
            break;
          }

          match state.focus {
            Focus::Menu => {
              let action = handle_menu_key(&mut state.menu, key);
              match action {
                MenuAction::Close => {
                  state.focus = Focus::Grid;
                }
                MenuAction::Quit => {
                  should_quit = true;
                }
                MenuAction::ReleaseAll => {
                  let _ = midi_tx.send(midi::Message::ClearMsgConfig());
                  let _ = midi_tx.send(midi::Message::Panic());
                }
                MenuAction::InsertFile => {
                  // TODO: file picker integration
                }
                _ => {}
              }
            }
            Focus::RegexInput => {
              // Regex input handled by line_editor events - dispatch non-Esc keys to it.
              use crossterm::event::KeyCode;
              if key.code == KeyCode::Esc {
                state.focus = Focus::Grid;
              } else {
                state.line_editor.handle_key(key);
                // Trigger regex solve on each edit.
                let pattern = state.line_editor.content().to_string();
                if pattern.is_empty() {
                  let _ = regex_tx.send(crate::core::engine::regex::Message::Clear);
                } else {
                  let _ = regex_tx.send(crate::core::engine::regex::Message::Solve(
                    crate::core::engine::regex::EventData {
                      text: grid.text_contents(),
                      pattern,
                      flags: state.flags.to_flag_str().to_string(),
                      grid_width: grid.grid.width,
                    },
                  ));
                }
              }
            }
            Focus::Grid => {
              if key.code == KeyCode::Esc {
                state.focus = Focus::RegexInput;
              } else if !cmd_mgr.dispatch_key(key, &mut state, &mut grid, &mut should_quit) {
                crate::view::grid::handle_key_event(&mut grid, key);
              }
            }
          }

          if should_quit {
            break;
          }
        }
        Event::Mouse(mouse) => {
          // Switch to grid focus on any click so mouse always works.
          use crossterm::event::{MouseButton, MouseEventKind};
          if matches!(
            mouse.kind,
            MouseEventKind::Down(MouseButton::Left)
              | MouseEventKind::Down(MouseButton::Right)
              | MouseEventKind::Down(MouseButton::Middle)
          ) {
            state.focus = Focus::Grid;
          }
          crate::view::grid::handle_mouse_event(
            &mut grid,
            mouse,
            PADDING_X,
            2 + PADDING_Y + CONSOLE_HEIGHT,
          );
        }
        Event::Resize(w, h) => {
          renderer.resize(w, h);
          state.resize(w, h);
          let grid_h = h.saturating_sub(CONSOLE_HEIGHT + 2 + PADDING_Y * 2);
          grid.resize(crate::core::geom::Vec2::new(
            w.saturating_sub(PADDING_X * 2) as usize,
            grid_h as usize,
          ));
        }
        _ => {}
      }
    }

    // Drain UI updates from the playhead thread.
    while let Ok(update) = ui_rx.try_recv() {
      // Handle symspell-specific variants that need grid + state access.
      match &update {
        crate::core::playhead::UIUpdate::TmpAppendSpace => {
          sym_state.handle_buf_append_space(&mut state);
          continue;
        }
        crate::core::playhead::UIUpdate::TmpAppend(idx) => {
          sym_state.handle_buf_append(&grid, &mut state, *idx);
          continue;
        }
        crate::core::playhead::UIUpdate::RplCycle(area) => {
          let area = *area;
          sym_state.handle_rpl_cycle(&mut grid, &mut state, area);
          continue;
        }
        _ => {}
      }
      apply_ui_update(update, &mut state);
    }

    grid.playhead_ui = state.playhead_ui.clone();
    grid.is_canvas_focused = matches!(state.focus, Focus::Grid);

    // Advance symspell animation if one is running.
    apply_sym_anim_tick(&sym_state, &mut grid, &mut state, &regex_tx);

    draw_frame(&state, &grid, renderer.current_mut());
    renderer.flush(&mut std::io::stdout())?;
  }
  Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_frame(
  state: &crate::app_state::AppState,
  grid: &crate::view::grid::GridEditor,
  buf: &mut crate::terminal::buffer::ScreenBuffer,
) {
  use crate::terminal::cell::Color;
  use crate::view::console::draw_console;
  use crate::view::consts::{CONSOLE_HEIGHT, PADDING_X, PADDING_Y};
  use crate::view::printer::{apply_style, CellStyle};

  buf.clear();
  let w = buf.width;

  let bx0 = PADDING_X.saturating_sub(1);
  let bx1 = w.saturating_sub(PADDING_X);
  let by0 = PADDING_Y;
  let by1 = 1 + PADDING_Y + CONSOLE_HEIGHT;
  let focused = matches!(state.focus, crate::app_state::Focus::RegexInput);
  let border_col = if focused {
    Color::Rgb(255, 255, 255)
  } else {
    Color::Rgb(60, 60, 60)
  };
  let bstyle = CellStyle {
    fg: border_col,
    bg: Color::Reset,
    reverse: false,
  };

  for x in bx0..=bx1 {
    let top_ch = if x == bx0 {
      '┌'
    } else if x == bx1 {
      '┐'
    } else {
      '─'
    };
    let bot_ch = if x == bx0 {
      '└'
    } else if x == bx1 {
      '┘'
    } else {
      '─'
    };
    if let Some(c) = buf.get_mut(x, by0) {
      apply_style(c, top_ch, bstyle);
    }
    if let Some(c) = buf.get_mut(x, by1) {
      apply_style(c, bot_ch, bstyle);
    }
  }
  for y in by0 + 1..by1 {
    if let Some(c) = buf.get_mut(bx0, y) {
      apply_style(c, '│', bstyle);
    }
    if let Some(c) = buf.get_mut(bx1, y) {
      apply_style(c, '│', bstyle);
    }
  }

  draw_console(state, buf, PADDING_X, 1 + PADDING_Y, w, CONSOLE_HEIGHT);
  grid.draw_to_buf(buf, PADDING_X, 2 + PADDING_Y + CONSOLE_HEIGHT);
  // draw_menubar(state, buf, 0);
}
