use std::fmt;

use crate::core::command::register::CommandManager;
use crate::core::consts;
use crate::core::engine::mod_matrix::{DiceDest, ModSource};
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
#[allow(clippy::too_many_arguments)]
pub fn run_event_loop(
  ui_rx: std::sync::mpsc::Receiver<crate::core::playhead::UIUpdate>,
  playhead_tx: std::sync::mpsc::Sender<crate::core::playhead::Message>,
  cmd_mgr: &CommandManager,
  sym_state: std::sync::Arc<crate::core::engine::symspell::SymSpellState>,
  regex_tx: std::sync::mpsc::Sender<crate::core::engine::regex::Message>,
  midi_tx: std::sync::mpsc::Sender<midi::Message>,
  renderer: &mut crate::terminal::Renderer,
  midi_output_devices: Vec<String>,
  midi_input_devices: Vec<String>,
  initial_midi_device: String,
) -> std::io::Result<()> {
  use crate::app_state::{apply_ui_update, AppState, Focus};
  use crate::view::consts::{CONSOLE_HEIGHT, PADDING_X, PADDING_Y};
  use crate::view::grid::GridEditor;
  use crate::view::menubar::{handle_menu_key, set_grid_contents, MenuAction};
  use crate::view::ui_processor::apply_sym_anim_tick;
  use crossterm::event::{poll, read, Event, KeyCode, KeyModifiers};
  use std::time::Duration;

  let mut state = AppState::default();
  state.menu.midi_output_devices = midi_output_devices;
  state.menu.midi_input_devices = midi_input_devices;
  state.midi_status = initial_midi_device;
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
  state.grid_width = grid.grid.width;

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

          if state.show_about {
            state.show_about = false;
            continue;
          }

          if state.show_docs {
            state.show_docs = false;
            continue;
          }

          if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('b') {
            state.show_menubar = !state.show_menubar;
            if state.show_menubar {
              state.focus = Focus::Menu;
              state.menu.visible = true;
              state.menu.active_menu = None;
              state.menu.focused_tab = 0;
            } else {
              state.focus = Focus::Grid;
              state.menu.visible = false;
              state.menu.active_menu = None;
            }
            continue;
          }

          match state.focus {
            Focus::Menu => {
              let action = handle_menu_key(&mut state.menu, key);
              let mut close_after = false;
              match action {
                MenuAction::None => {}
                MenuAction::Close => {
                  state.show_menubar = false;
                  state.focus = Focus::Grid;
                }
                MenuAction::Quit => {
                  should_quit = true;
                }
                MenuAction::ReleaseAll => {
                  let _ = midi_tx.send(midi::Message::ClearMsgConfig());
                  let _ = midi_tx.send(midi::Message::Panic());
                  close_after = true;
                }
                MenuAction::ClearQueue => {
                  let _ = playhead_tx.send(crate::core::playhead::Message::ClearQueue());
                  close_after = true;
                }
                MenuAction::ToggleClock => {
                  use std::sync::atomic::Ordering;
                  let was = consts::CLOCK_ENABLED.load(Ordering::Relaxed);
                  consts::CLOCK_ENABLED.store(!was, Ordering::Relaxed);
                  let _ = midi_tx.send(midi::Message::EnableClock(!was));
                }
                MenuAction::ToggleFocus => {
                  use std::sync::atomic::Ordering;
                  let was = consts::FOCUS_MODE.load(Ordering::Relaxed);
                  consts::FOCUS_MODE.store(!was, Ordering::Relaxed);
                }
                MenuAction::ToggleStreamingPB => {
                  use std::sync::atomic::Ordering;
                  let was = consts::SYNTH_ENABLED.load(Ordering::Relaxed);
                  consts::SYNTH_ENABLED.store(!was, Ordering::Relaxed);
                }
                MenuAction::ToggleClearStreamingMsg => {
                  use std::sync::atomic::Ordering;
                  let was = consts::SYNTH_CLEAR_MSG.load(Ordering::Relaxed);
                  consts::SYNTH_CLEAR_MSG.store(!was, Ordering::Relaxed);
                }
                MenuAction::InsertFile => {
                  // TODO: file picker integration
                  close_after = true;
                }
                MenuAction::About => {
                  state.show_about = true;
                  close_after = true;
                }
                MenuAction::ShowDocs => {
                  state.show_docs = true;
                  close_after = true;
                }
                MenuAction::MidiOutputSelected(idx) => {
                  let _ = midi_tx.send(midi::Message::SwitchDevice(idx));
                  if let Some(name) = state.menu.midi_output_devices.get(idx) {
                    state.midi_status = name.clone();
                  }
                  close_after = true;
                }
                MenuAction::MidiInputSelected(_idx) => {
                  // TODO: midi input switching not yet wired to a message
                  close_after = true;
                }
                MenuAction::ScaleLeftSelected(idx) => {
                  use crate::core::tonal::scale::ScaleMode;
                  if let Some(mode) = ScaleMode::all().get(idx) {
                    let _ =
                      playhead_tx.send(crate::core::playhead::Message::SetScaleModeLeft(*mode));
                  }
                  close_after = true;
                }
                MenuAction::ScaleTopSelected(idx) => {
                  use crate::core::tonal::scale::ScaleMode;
                  if let Some(mode) = ScaleMode::all().get(idx) {
                    let _ =
                      playhead_tx.send(crate::core::playhead::Message::SetScaleModeTop(*mode));
                  }
                  close_after = true;
                }
                MenuAction::ScaleRootSelected(idx) => {
                  use crate::core::tonal::scale::ScaleRoot;
                  if let Some(root) = ScaleRoot::all().get(idx) {
                    let _ =
                      playhead_tx.send(crate::core::playhead::Message::SetScaleRootTop(*root));
                  }
                  close_after = true;
                }
              }
              if close_after {
                state.show_menubar = false;
                state.menu.active_menu = None;
                state.focus = Focus::Grid;
              }
            }
            Focus::RegexInput => {
              if key.code == KeyCode::Esc {
                state.focus = Focus::Grid;
              } else if key.code == KeyCode::Tab {
                state.focus = state.focus.tab_next();
              } else if key.code == KeyCode::BackTab {
                state.focus = state.focus.tab_prev();
              } else {
                state.line_editor.handle_key(key);
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
            Focus::FlagCaseSensitive | Focus::FlagMultiline => {
              match key.code {
                KeyCode::Esc => state.focus = Focus::Grid,
                KeyCode::Tab => state.focus = state.focus.tab_next(),
                KeyCode::BackTab => state.focus = state.focus.tab_prev(),
                KeyCode::Char(' ') | KeyCode::Enter => {
                  if matches!(state.focus, Focus::FlagCaseSensitive) {
                    state.flags.case_sensitive = !state.flags.case_sensitive;
                  } else {
                    state.flags.multiline = !state.flags.multiline;
                  }
                  // re-solve with updated flags
                  let pattern = state.line_editor.content().to_string();
                  if !pattern.is_empty() {
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
                _ => {}
              }
            }
            Focus::ModMatrix { row, col } => {
              match key.code {
                KeyCode::Esc => state.focus = Focus::Grid,
                KeyCode::Tab => state.focus = state.focus.tab_next(),
                KeyCode::BackTab => state.focus = state.focus.tab_prev(),
                KeyCode::Up => state.focus = state.focus.mod_matrix_move(-1, 0),
                KeyCode::Down => state.focus = state.focus.mod_matrix_move(1, 0),
                KeyCode::Left => state.focus = state.focus.mod_matrix_move(0, -1),
                KeyCode::Right => state.focus = state.focus.mod_matrix_move(0, 1),
                KeyCode::Char(' ') | KeyCode::Enter => {
                  let src = ModSource::ALL[row as usize];
                  let dst = DiceDest::ALL[col as usize];
                  let next = match state.mod_matrix.get_amount(src, dst) {
                    None => 1.0,
                    Some(v) if v > 0.0 => -1.0,
                    _ => 0.0, // 0.0 removes the route
                  };
                  state.mod_matrix.set_route(src, dst, next);
                  if dst == DiceDest::Face {
                    grid.refresh_dice_effective_face(&state.mod_matrix);
                  }
                }
                KeyCode::Char('+') | KeyCode::Char('=') => {
                  let src = ModSource::ALL[row as usize];
                  let dst = DiceDest::ALL[col as usize];
                  let current = state.mod_matrix.get_amount(src, dst).unwrap_or(0.0);
                  let next = ((current + 0.1) * 10.0).round() / 10.0;
                  state.mod_matrix.set_route(src, dst, next.clamp(-1.0, 1.0));
                  if dst == DiceDest::Face {
                    grid.refresh_dice_effective_face(&state.mod_matrix);
                  }
                }
                KeyCode::Char('-') => {
                  let src = ModSource::ALL[row as usize];
                  let dst = DiceDest::ALL[col as usize];
                  let current = state.mod_matrix.get_amount(src, dst).unwrap_or(0.0);
                  let next = ((current - 0.1) * 10.0).round() / 10.0;
                  state.mod_matrix.set_route(src, dst, next.clamp(-1.0, 1.0));
                  if dst == DiceDest::Face {
                    grid.refresh_dice_effective_face(&state.mod_matrix);
                  }
                }
                _ => {}
              }
            }
            Focus::Grid => {
              if key.code == KeyCode::Esc {
                state.focus = Focus::RegexInput;
              } else if key.code == KeyCode::Char('0') {
                state.show_waveform_console = !state.show_waveform_console;
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
          use crossterm::event::{MouseButton, MouseEventKind};
          if matches!(
            mouse.kind,
            MouseEventKind::Down(MouseButton::Left)
              | MouseEventKind::Down(MouseButton::Right)
              | MouseEventKind::Down(MouseButton::Middle)
          ) {
            let console_y_off = 1 + PADDING_Y;
            if let Some(focus) = crate::view::console::hit_test_console(
              mouse.column,
              mouse.row,
              PADDING_X,
              console_y_off,
              state.width,
            ) {
              state.focus = focus;
            } else {
              state.focus = Focus::Grid;
            }
          }
          let grid_y_off = 2 + PADDING_Y + CONSOLE_HEIGHT;
          if mouse.row >= grid_y_off {
            crate::view::grid::handle_mouse_event(&mut grid, mouse, PADDING_X, grid_y_off);
          }
        }
        Event::Resize(w, h) => {
          renderer.resize(w, h);
          state.resize(w, h);
          let grid_h = h.saturating_sub(CONSOLE_HEIGHT + 2 + PADDING_Y * 2);
          grid.resize(crate::core::geom::Vec2::new(
            w.saturating_sub(PADDING_X * 2) as usize,
            grid_h as usize,
          ));
          state.grid_width = grid.grid.width;
        }
        _ => {}
      }
    }

    state.effective_bars_div = grid.dice_effective_bars_div;

    // Drain UI updates from the playhead thread.
    let prev_anchor_x = state.playhead_ui.playhead_pos.x;
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
    grid.playhead_ui.focus_mode = consts::FOCUS_MODE.load(std::sync::atomic::Ordering::Relaxed);
    grid.is_canvas_focused = matches!(state.focus, Focus::Grid);
    if state.playhead_ui.playhead_pos.x != prev_anchor_x {
      grid.refresh_dice_effective_face(&state.mod_matrix);
    }
    grid.apply_dice_scale_if_changed(&state.mod_matrix);

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
  use crate::view::menubar::draw_menubar;
  use crate::view::printer::{apply_style, canvas, white, CellStyle};

  buf.clear();
  let w = buf.width;

  let bx0 = PADDING_X.saturating_sub(1);
  let bx1 = w.saturating_sub(PADDING_X);
  let by0 = PADDING_Y;
  let by1 = 1 + PADDING_Y + CONSOLE_HEIGHT;
  let focused = matches!(state.focus, crate::app_state::Focus::RegexInput);
  let border_col = if focused { white() } else { canvas() };
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

  if state.show_waveform_console {
    crate::view::console::draw_waveform_console(
      state,
      buf,
      PADDING_X,
      1 + PADDING_Y,
      w,
      CONSOLE_HEIGHT,
    );
  } else {
    draw_console(state, buf, PADDING_X, 1 + PADDING_Y, w, CONSOLE_HEIGHT);
  }
  grid.draw_to_buf(buf, PADDING_X, 2 + PADDING_Y + CONSOLE_HEIGHT);
  if state.show_menubar {
    draw_menubar(state, buf, 0);
  }
  if state.show_about {
    draw_about_dialog(state, buf);
  }
  if state.show_docs {
    draw_docs_dialog(state, buf);
  }
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_about_dialog(
  state: &crate::app_state::AppState,
  buf: &mut crate::terminal::buffer::ScreenBuffer,
) {
  use crate::view::printer::draw_dialog;

  let mut lines: Vec<String> = vec![
    format!("{}  v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    String::new(),
  ];
  let words: Vec<&str> = env!("CARGO_PKG_DESCRIPTION").split_whitespace().collect();
  for chunk in words.chunks(6) {
    lines.push(chunk.join(" "));
  }
  lines.push(String::new());
  lines.push("press any key to close".to_string());

  let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
  draw_dialog(buf, state.width, state.height, &refs);
}

#[cfg(not(target_arch = "wasm32"))]
fn draw_docs_dialog(
  state: &crate::app_state::AppState,
  buf: &mut crate::terminal::buffer::ScreenBuffer,
) {
  use crate::view::printer::draw_dialog;

  draw_dialog(
    buf,
    state.width,
    state.height,
    &["docs", "", "coming soon...", "", "press any key to close"],
  );
}
