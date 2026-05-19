/// Exports functions that JavaScript drives:
///   wasm_init(cols, rows)   - set up the whole application
///   wasm_step(elapsed_ms)   - advance one frame (~60 fps)
///   wasm_send_key(key_str)  - forward a key string from xterm.js
///   wasm_render()           - returns the ANSI string to write to xterm.js
///   wasm_resize(cols, rows) - notify the backend of a terminal resize
use std::cell::RefCell;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use wasm_bindgen::prelude::*;

use crate::app::initialize_components;
use crate::app_state::{apply_ui_update, AppState, Focus};
use crate::core::engine::regex::RegexCache;
use crate::core::engine::symspell::SymSpellState;
use crate::core::playhead::{Message as PlayheadMessage, Playhead, UIUpdate};
use crate::core::timing::metronome::{Message as MetronomeMessage, Metronome};
use crate::terminal::buffer::ScreenBuffer;
use crate::terminal::cell::Color as TermColor;
use crate::view::grid::GridEditor;
use crate::view::menubar::set_grid_contents;

struct BackendState {
  buf: ScreenBuffer,
}

impl BackendState {
  fn new(cols: u16, rows: u16) -> Self {
    Self {
      buf: ScreenBuffer::new(cols, rows),
    }
  }

  fn resize(&mut self, cols: u16, rows: u16) {
    self.buf.resize(cols, rows);
  }

  fn render_ansi(&self) -> String {
    let cols = self.buf.width as usize;
    let rows = self.buf.height as usize;
    let mut out = String::with_capacity(cols * rows * 8);
    out.push_str("\x1b[?25l\x1b[H");

    let mut prev_fg: Option<TermColor> = None;
    let mut prev_bg: Option<TermColor> = None;
    let mut prev_reverse: Option<bool> = None;

    for row in 0..rows {
      for col in 0..cols {
        let cell = &self.buf.cells[row * cols + col];
        let need_color = prev_fg.map_or(true, |f| f != cell.fg)
          || prev_bg.map_or(true, |b| b != cell.bg)
          || prev_reverse.map_or(true, |r| r != cell.reverse);
        if need_color {
          if cell.reverse {
            out.push_str("\x1b[0m\x1b[7m");
          } else {
            out.push_str("\x1b[0m");
            out.push_str(&ansi_color(cell.fg, cell.bg));
          }
          prev_fg = Some(cell.fg);
          prev_bg = Some(cell.bg);
          prev_reverse = Some(cell.reverse);
        }
        let ch = if cell.ch == '\0' { ' ' } else { cell.ch };
        out.push(ch);
      }
      if row + 1 < rows {
        out.push_str("\r\n");
      }
    }
    out.push_str("\x1b[0m\x1b[?25l");
    out
  }
}

fn ansi_color(fg: TermColor, bg: TermColor) -> String {
  format!("{}{}", ansi_fg(fg), ansi_bg(bg))
}

fn ansi_fg(c: TermColor) -> String {
  match c {
    TermColor::Reset => "\x1b[39m".to_string(),
    TermColor::Rgb(r, g, b) => format!("\x1b[38;2;{r};{g};{b}m"),
    TermColor::Indexed(i) if i < 8 => format!("\x1b[{}m", 30 + i),
    TermColor::Indexed(i) if i < 16 => format!("\x1b[{}m", 90 + (i - 8)),
    TermColor::Indexed(i) => format!("\x1b[38;5;{}m", i),
  }
}

fn ansi_bg(c: TermColor) -> String {
  match c {
    TermColor::Reset => "\x1b[49m".to_string(),
    TermColor::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
    TermColor::Indexed(i) if i < 8 => format!("\x1b[{}m", 40 + i),
    TermColor::Indexed(i) if i < 16 => format!("\x1b[{}m", 100 + (i - 8)),
    TermColor::Indexed(i) => format!("\x1b[48;5;{}m", i),
  }
}

enum WasmKey {
  Char(char),
  Ctrl(char),
  Alt(char),
  Enter,
  Esc,
  Backspace,
  Tab,
  Up,
  Down,
  Left,
  Right,
  Home,
  End,
  Delete,
  PageUp,
  PageDown,
}

fn parse_key(s: &str) -> Vec<WasmKey> {
  let mut out = Vec::new();
  match s {
    "\r" | "\n" => out.push(WasmKey::Enter),
    "\x1b" => out.push(WasmKey::Esc),
    "\x7f" | "\x08" => out.push(WasmKey::Backspace),
    "\t" => out.push(WasmKey::Tab),
    "\x1b[A" | "\x1bOA" => out.push(WasmKey::Up),
    "\x1b[B" | "\x1bOB" => out.push(WasmKey::Down),
    "\x1b[C" | "\x1bOC" => out.push(WasmKey::Right),
    "\x1b[D" | "\x1bOD" => out.push(WasmKey::Left),
    "\x1b[H" | "\x1bOH" => out.push(WasmKey::Home),
    "\x1b[F" | "\x1bOF" => out.push(WasmKey::End),
    "\x1b[3~" => out.push(WasmKey::Delete),
    "\x1b[5~" => out.push(WasmKey::PageUp),
    "\x1b[6~" => out.push(WasmKey::PageDown),
    s => {
      let bytes = s.as_bytes();
      if bytes.len() == 1 {
        let b = bytes[0];
        if (1..=26).contains(&b) {
          out.push(WasmKey::Ctrl((b'a' + b - 1) as char));
        } else if b >= 0x20 {
          out.push(WasmKey::Char(b as char));
        }
      } else if bytes.len() >= 2 && bytes[0] == b'\x1b' {
        let rest = &s[1..];
        let rest_bytes = rest.as_bytes();
        if rest_bytes.len() == 1 {
          let b = rest_bytes[0];
          if (1..=26).contains(&b) {
            out.push(WasmKey::Alt((b'a' + b - 1) as char));
          } else if b >= 0x20 && b < 0x7f {
            out.push(WasmKey::Alt(b as char));
          }
        }
      } else {
        for ch in s.chars() {
          if !ch.is_control() {
            out.push(WasmKey::Char(ch));
          }
        }
      }
    }
  }
  out
}

// -------- Background tick context --------

struct WasmCtx {
  playhead: Arc<Playhead>,
  regex_tx: Sender<crate::core::engine::regex::Message>,
  sym_state: Arc<SymSpellState>,
  ui_rx: std::sync::mpsc::Receiver<UIUpdate>,
  pending_ui: Vec<UIUpdate>,
  regex_handler: crate::core::engine::regex::RegExpHandler,
  regex_cache: RegexCache,
  metronome: Metronome,
  metronome_tx: Sender<MetronomeMessage>,
  midi: crate::core::io::midi::Midi,
  clock_accum_ms: f64,
  clock_tick: usize,
  current_tempo: Arc<std::sync::Mutex<usize>>,
  committed_tempo: usize,
}

impl WasmCtx {
  fn tick(&mut self, elapsed_ms: f64) {
    {
      let current = *self.current_tempo.lock().unwrap();
      if current != self.committed_tempo {
        self.committed_tempo = current;
        use num_rational::Ratio;
        let _ = self
          .metronome_tx
          .send(MetronomeMessage::Tempo(Ratio::from_integer(current as i64)));
      }
    }

    self.metronome.wasm_tick();

    if self.metronome.wasm_is_playing() {
      let bpm = self.metronome.wasm_current_bpm().max(1.0);
      let ms_per_tick = 60_000.0 / (bpm * 16.0);
      self.clock_accum_ms += elapsed_ms;
      while self.clock_accum_ms >= ms_per_tick {
        self.clock_accum_ms -= ms_per_tick;
        self.clock_tick += 1;
        let _ = self
          .metronome
          .playhead_tx
          .send(PlayheadMessage::SetActivePos(self.clock_tick));
      }
    }

    self.playhead.wasm_tick();

    while let Ok(update) = self.ui_rx.try_recv() {
      self.pending_ui.push(update);
    }

    self.regex_handler.wasm_tick(&mut self.regex_cache);
    self.midi.wasm_tick(self.clock_tick);
  }
}

struct WasmUiCtx {
  state: AppState,
  grid: GridEditor,
  backend: BackendState,
  cmd_mgr: crate::core::command::register::CommandManager,
}

thread_local! {
  static CTX: RefCell<Option<WasmCtx>> = RefCell::new(None);
  static UI: RefCell<Option<WasmUiCtx>> = RefCell::new(None);
  static ANSI_OUTPUT: RefCell<String> = RefCell::new(String::new());
}

/// Initialise the application. Call once before anything else.
#[wasm_bindgen]
pub fn wasm_init(cols: u32, rows: u32) {
  console_error_panic_hook::set_once();

  let mut components = initialize_components();

  let playhead = components.playhead;
  let sym_state = Arc::clone(&playhead.sym_state);
  let regex_tx = components.regex_tx.clone();
  let regex_handler = components.regex_handler;
  let metronome_tx = components.metronome.tx.clone();
  let metronome = components.metronome;
  let midi = components.midi;
  let ui_rx = components.ui_rx;
  let current_tempo = Arc::clone(&components.current_tempo);
  let committed_tempo = *current_tempo.lock().unwrap();

  let ctx = WasmCtx {
    playhead,
    regex_tx: regex_tx.clone(),
    sym_state,
    ui_rx,
    pending_ui: Vec::new(),
    regex_handler,
    regex_cache: RegexCache::new(),
    metronome,
    metronome_tx,
    midi,
    clock_accum_ms: 0.0,
    clock_tick: 0,
    current_tempo,
    committed_tempo,
  };

  use crate::view::consts::{CONSOLE_HEIGHT, PADDING_X, PADDING_Y};
  let playhead_tx = components.playhead_tx.clone();
  let mut grid = GridEditor::new(playhead_tx);
  grid.regex_tx = Some(regex_tx);
  let init_w = cols as u16;
  let init_h = rows as u16;
  let grid_w = init_w.saturating_sub(PADDING_X * 2) as usize;
  let grid_h = init_h.saturating_sub(CONSOLE_HEIGHT + 2 + PADDING_Y * 2) as usize;
  grid.resize(crate::core::geom::Vec2::new(grid_w, grid_h));

  let mut state = AppState::default();
  state.resize(cols as u16, rows as u16);
  state.focus = Focus::Grid;
  state.midi_status = ctx.midi.out_device_name();
  grid.is_canvas_focused = true;

  set_grid_contents(&mut grid, crate::core::consts::MANIFESTO_TEXT.to_string());

  let ui_ctx = WasmUiCtx {
    state,
    grid,
    backend: BackendState::new(cols as u16, rows as u16),
    cmd_mgr: components.cmd_mgr,
  };

  CTX.with(|c| *c.borrow_mut() = Some(ctx));
  UI.with(|u| *u.borrow_mut() = Some(ui_ctx));
}

/// Advance one frame. Call at ~60 fps from `requestAnimationFrame`.
#[wasm_bindgen]
pub fn wasm_step(elapsed_ms: f64) {
  CTX.with(|c| {
    if let Some(ctx) = c.borrow_mut().as_mut() {
      ctx.tick(elapsed_ms);
    }
  });

  let maybe_ui = CTX.with(|c| {
    c.borrow_mut().as_mut().map(|ctx| {
      (
        std::mem::take(&mut ctx.pending_ui),
        Arc::clone(&ctx.sym_state),
        ctx.regex_tx.clone(),
      )
    })
  });

  if let Some((pending_ui, sym_state, regex_tx)) = maybe_ui {
    UI.with(|u| {
      if let Some(ui) = u.borrow_mut().as_mut() {
        for update in pending_ui {
          match &update {
            UIUpdate::TmpAppendSpace => {
              sym_state.handle_buf_append_space(&mut ui.state);
              continue;
            }
            UIUpdate::TmpAppend(idx) => {
              sym_state.handle_buf_append(&ui.grid, &mut ui.state, *idx);
              continue;
            }
            UIUpdate::RplCycle(area) => {
              let area = *area;
              sym_state.handle_rpl_cycle(&mut ui.grid, &mut ui.state, area);
              continue;
            }
            _ => {}
          }
          apply_ui_update(update, &mut ui.state);
        }

        // Sync grid's PlayheadUI from AppState.
        ui.grid.playhead_ui = ui.state.playhead_ui.clone();
        ui.grid.playhead_ui.focus_mode =
          crate::core::consts::FOCUS_MODE.load(std::sync::atomic::Ordering::Relaxed);

        // Advance symspell animation.
        if let Some(tick) = sym_state.advance_anim_frame() {
          sym_state.render_anim_tick(&mut ui.grid, &mut ui.state, tick, &regex_tx);
        }

        // Draw frame into backend buffer.
        draw_wasm_frame(&ui.state, &ui.grid, &mut ui.backend.buf);

        // Render ANSI.
        let ansi = ui.backend.render_ansi();
        ANSI_OUTPUT.with(|a| *a.borrow_mut() = ansi);
      }
    });
  }
}

fn draw_wasm_frame(state: &AppState, grid: &GridEditor, buf: &mut ScreenBuffer) {
  use crate::terminal::cell::Color;
  use crate::view::console::draw_console;
  use crate::view::consts::{CONSOLE_HEIGHT, PADDING_X, PADDING_Y};
  use crate::view::menubar::draw_menubar;
  use crate::view::printer::{apply_style, canvas, draw_dialog, white, CellStyle};

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

  if state.console_view == crate::app_state::ConsoleView::Waveform {
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

  if state.show_docs {
    draw_dialog(
      buf,
      state.width,
      state.height,
      &["docs", "", "coming soon...", "", "press any key to close"],
    );
  }
}

/// Map a WasmKey to the binding string used in binding.rs (e.g. "Space", "Ctrl+f").
/// Returns None for keys handled elsewhere (Esc, Ctrl+b, grid nav keys).
fn wasm_key_to_binding(key: &WasmKey) -> Option<&'static str> {
  match key {
    WasmKey::Char(' ') => Some("Space"),
    WasmKey::Char('q') => Some("q"),
    WasmKey::Char('>') => Some(">"),
    WasmKey::Char('<') => Some("<"),
    WasmKey::Char('}') => Some("}"),
    WasmKey::Char('{') => Some("{"),
    WasmKey::Char('~') => Some("~"),
    WasmKey::Char('!') => Some("!"),
    WasmKey::Char('i') => Some("i"),
    WasmKey::Char('p') => Some("p"),
    WasmKey::Char('I') => Some("I"),
    WasmKey::Char('P') => Some("P"),
    WasmKey::Char('|') => Some("Pipe"),
    WasmKey::Char('@') => Some("@"),
    WasmKey::Char('[') => Some("["),
    WasmKey::Char(']') => Some("]"),
    WasmKey::Char('+') => Some("Shift+Plus"),
    WasmKey::Char('_') => Some("Shift+Underscore"),
    WasmKey::Char('=') => Some("Equal"),
    WasmKey::Char('-') => Some("Minus"),
    WasmKey::Ctrl('f') => Some("Ctrl+f"),
    WasmKey::Ctrl('r') => Some("Ctrl+r"),
    WasmKey::Ctrl('d') => Some("Ctrl+d"),
    WasmKey::Ctrl('p') => Some("Ctrl+p"),
    WasmKey::Ctrl('a') => Some("Ctrl+a"),
    WasmKey::Ctrl('u') => Some("Ctrl+u"),
    WasmKey::Ctrl('e') => Some("Ctrl+e"),
    WasmKey::Ctrl('n') => Some("Ctrl+n"),
    WasmKey::Ctrl('s') => Some("Ctrl+s"),
    WasmKey::Ctrl('o') => Some("Ctrl+o"),
    WasmKey::Ctrl('y') => Some("Ctrl+y"),
    WasmKey::Ctrl('z') => Some("Ctrl+z"),
    WasmKey::Ctrl('t') => Some("Ctrl+t"),
    _ => None,
  }
}

/// Map a WasmKey to the platform-independent MenuNavKey for menu navigation.
fn wasm_key_to_menu_nav(key: &WasmKey) -> crate::view::menubar::MenuNavKey {
  use crate::view::menubar::MenuNavKey;
  match key {
    WasmKey::Left => MenuNavKey::Left,
    WasmKey::Right => MenuNavKey::Right,
    WasmKey::Up => MenuNavKey::Up,
    WasmKey::Down => MenuNavKey::Down,
    WasmKey::Enter => MenuNavKey::Enter,
    WasmKey::Esc => MenuNavKey::Esc,
    _ => MenuNavKey::Other,
  }
}

/// Forward keyboard input from xterm.js.
#[wasm_bindgen]
pub fn wasm_send_key(key: String) {
  let keys = parse_key(&key);
  UI.with(|u| {
    if let Some(ui) = u.borrow_mut().as_mut() {
      let mut should_quit = false;
      for wkey in keys {
        dispatch_wasm_key(wkey, ui, &mut should_quit);
      }
    }
  });
}

fn dispatch_wasm_key(key: WasmKey, ui: &mut WasmUiCtx, should_quit: &mut bool) {
  use crate::core::consts;
  use crate::core::playhead::Direction;
  use crate::view::menubar::{handle_menu_nav, MenuAction};

  // Dismiss dialogs on any key.
  if ui.state.show_about || ui.state.show_docs {
    ui.state.show_about = false;
    ui.state.show_docs = false;
    return;
  }

  match ui.state.focus {
    Focus::RegexInput => {
      let changed = match &key {
        WasmKey::Esc => {
          ui.state.focus = Focus::Grid;
          ui.grid.is_canvas_focused = true;
          false
        }
        WasmKey::Char(c) => {
          matches!(
            ui.state.line_editor.insert_char(*c),
            crate::view::line_editor::LineEditorAction::Changed
          )
        }
        WasmKey::Backspace => {
          matches!(
            ui.state.line_editor.backspace(),
            crate::view::line_editor::LineEditorAction::Changed
          )
        }
        WasmKey::Delete => {
          matches!(
            ui.state.line_editor.delete_forward(),
            crate::view::line_editor::LineEditorAction::Changed
          )
        }
        WasmKey::Left => {
          ui.state.line_editor.move_left();
          false
        }
        WasmKey::Right => {
          ui.state.line_editor.move_right();
          false
        }
        WasmKey::Home => {
          ui.state.line_editor.move_home();
          false
        }
        WasmKey::End => {
          ui.state.line_editor.move_end();
          false
        }
        WasmKey::Ctrl('a') => {
          ui.state.line_editor.move_home();
          false
        }
        WasmKey::Ctrl('e') => {
          ui.state.line_editor.move_end();
          false
        }
        WasmKey::Ctrl('u') => {
          matches!(
            ui.state.line_editor.kill_before_cursor(),
            crate::view::line_editor::LineEditorAction::Changed
          )
        }
        WasmKey::Ctrl('k') => {
          matches!(
            ui.state.line_editor.kill_after_cursor(),
            crate::view::line_editor::LineEditorAction::Changed
          )
        }
        _ => false,
      };

      if changed {
        let pattern = ui.state.line_editor.content().to_string();
        if let Some(ref tx) = ui.grid.regex_tx {
          if pattern.is_empty() {
            let _ = tx.send(crate::core::engine::regex::Message::Clear);
          } else {
            let _ = tx.send(crate::core::engine::regex::Message::Solve(
              crate::core::engine::regex::EventData {
                text: ui.grid.text_contents(),
                pattern,
                flags: ui.state.flags.to_flag_str().to_string(),
                grid_width: ui.grid.grid.width,
              },
            ));
          }
        }
      }
    }

    Focus::Menu => {
      // Ctrl+b while menu is open closes the menubar.
      if matches!(&key, WasmKey::Ctrl('b')) {
        ui.state.show_menubar = false;
        ui.state.focus = Focus::Grid;
        ui.state.menu.visible = false;
        ui.state.menu.active_menu = None;
        return;
      }

      let nav = wasm_key_to_menu_nav(&key);
      let action = handle_menu_nav(&mut ui.state.menu, nav);
      let mut close_after = false;
      match action {
        MenuAction::None => {}
        MenuAction::Close => {
          ui.state.show_menubar = false;
          ui.state.focus = Focus::Grid;
        }
        MenuAction::Quit => *should_quit = true,
        MenuAction::ToggleClock => {
          use std::sync::atomic::Ordering;
          let was = consts::CLOCK_ENABLED.load(Ordering::Relaxed);
          consts::CLOCK_ENABLED.store(!was, Ordering::Relaxed);
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
        MenuAction::ReleaseAll => close_after = true,
        MenuAction::ClearQueue => {
          let _ = ui
            .cmd_mgr
            .playhead_tx()
            .send(crate::core::playhead::Message::ClearQueue());
          close_after = true;
        }
        MenuAction::InsertFile => close_after = true,
        MenuAction::About => {
          ui.state.show_about = true;
          close_after = true;
        }
        MenuAction::ShowDocs => {
          ui.state.show_docs = true;
          close_after = true;
        }
        MenuAction::MidiOutputSelected(_) | MenuAction::MidiInputSelected(_) => {
          close_after = true;
        }
        MenuAction::ScaleLeftSelected(idx) => {
          use crate::core::tonal::scale::ScaleMode;
          if let Some(mode) = ScaleMode::all().get(idx) {
            let _ = ui
              .cmd_mgr
              .playhead_tx()
              .send(crate::core::playhead::Message::SetScaleModeLeft(*mode));
          }
          close_after = true;
        }
        MenuAction::ScaleTopSelected(idx) => {
          use crate::core::tonal::scale::ScaleMode;
          if let Some(mode) = ScaleMode::all().get(idx) {
            let _ = ui
              .cmd_mgr
              .playhead_tx()
              .send(crate::core::playhead::Message::SetScaleModeTop(*mode));
          }
          close_after = true;
        }
        MenuAction::ScaleRootSelected(idx) => {
          use crate::core::tonal::scale::ScaleRoot;
          if let Some(root) = ScaleRoot::all().get(idx) {
            let _ = ui
              .cmd_mgr
              .playhead_tx()
              .send(crate::core::playhead::Message::SetScaleRootTop(*root));
          }
          close_after = true;
        }
      }
      if close_after {
        ui.state.show_menubar = false;
        ui.state.menu.active_menu = None;
        ui.state.focus = Focus::Grid;
      }
    }

    Focus::Grid => {
      // Ctrl+b toggles menubar (handled before cmd_mgr, mirrors native behavior).
      if matches!(&key, WasmKey::Ctrl('b')) {
        ui.state.show_menubar = !ui.state.show_menubar;
        if ui.state.show_menubar {
          ui.state.focus = Focus::Menu;
          ui.state.menu.visible = true;
          ui.state.menu.active_menu = None;
          ui.state.menu.focused_tab = 0;
        } else {
          ui.state.menu.visible = false;
          ui.state.menu.active_menu = None;
        }
        return;
      }

      // Esc switches to RegexInput (same as native Grid focus handler).
      if matches!(&key, WasmKey::Esc) {
        ui.state.focus = Focus::RegexInput;
        ui.grid.is_canvas_focused = false;
        return;
      }

      // '0' toggles the waveform console view.
      if matches!(&key, WasmKey::Char('0')) {
        ui.state.console_view = ui.state.console_view.cycle();
        return;
      }

      // '^' toggles Streaming Pitch Bend when in Waveform view.
      if matches!(&key, WasmKey::Char('^'))
        && ui.state.console_view == crate::app_state::ConsoleView::Waveform
      {
        let was = crate::core::consts::SYNTH_ENABLED.load(std::sync::atomic::Ordering::Relaxed);
        crate::core::consts::SYNTH_ENABLED.store(!was, std::sync::atomic::Ordering::Relaxed);
        return;
      }

      // Try command bindings (Space, q, >, <, Ctrl+f, Ctrl+r, etc.).
      if let Some(key_str) = wasm_key_to_binding(&key) {
        if ui
          .cmd_mgr
          .dispatch_command_str(key_str, &mut ui.state, &mut ui.grid, should_quit)
        {
          return;
        }
      }

      // Grid navigation and split-char keys.
      match &key {
        WasmKey::Char('h') => {
          ui.grid.commit_or_move(Direction::Left);
        }
        WasmKey::Char('j') => {
          ui.grid.commit_or_move(Direction::Down);
        }
        WasmKey::Char('k') => {
          ui.grid.commit_or_move(Direction::Up);
        }
        WasmKey::Char('l') => {
          ui.grid.commit_or_move(Direction::Right);
        }
        WasmKey::Char('H') => {
          ui.grid.scale_action((-1, 0));
        }
        WasmKey::Char('J') => {
          ui.grid.scale_action((0, -1));
        }
        WasmKey::Char('K') => {
          ui.grid.scale_action((0, 1));
        }
        WasmKey::Char('L') => {
          ui.grid.scale_action((1, 0));
        }
        WasmKey::Alt('h') => {
          ui.grid
            .alt_action(Direction::Left, consts::MOVE_X_STEP_SIZE);
        }
        WasmKey::Alt('j') => {
          ui.grid
            .alt_action(Direction::Down, consts::MOVE_Y_STEP_SIZE);
        }
        WasmKey::Alt('k') => {
          ui.grid.alt_action(Direction::Up, consts::MOVE_Y_STEP_SIZE);
        }
        WasmKey::Alt('l') => {
          ui.grid
            .alt_action(Direction::Right, consts::MOVE_X_STEP_SIZE);
        }
        WasmKey::Ctrl('h') => {
          ui.grid.start_aim_if_needed();
          ui.grid.update_aim(Direction::Left, 1);
        }
        WasmKey::Ctrl('j') => {
          ui.grid.start_aim_if_needed();
          ui.grid.update_aim(Direction::Down, 1);
        }
        WasmKey::Ctrl('k') => {
          ui.grid.start_aim_if_needed();
          ui.grid.update_aim(Direction::Up, 1);
        }
        WasmKey::Ctrl('l') => {
          ui.grid.start_aim_if_needed();
          ui.grid.update_aim(Direction::Right, 1);
        }
        WasmKey::Char(c) if ('1'..='7').contains(c) => {
          ui.grid.sgr_leak_state = 0;
          ui.grid.handle_split_char(*c);
        }
        _ => {
          ui.grid.sgr_leak_state = 0;
        }
      }
    }
  }
}

/// Returns the ANSI string to write to `terminal.write()`.
#[wasm_bindgen]
pub fn wasm_render() -> String {
  ANSI_OUTPUT.with(|a| a.borrow().clone())
}

/// Forward a mouse event from the browser.
/// `kind` - 0=press, 1=hold/drag, 2=release
/// `button` - 0=left, 1=middle, 2=right
#[wasm_bindgen]
pub fn wasm_send_mouse(kind: u8, _button: u8, col: u32, row: u32) {
  use crate::view::consts::{CONSOLE_HEIGHT, PADDING_X, PADDING_Y};

  UI.with(|u| {
    if let Some(ui) = u.borrow_mut().as_mut() {
      let panel_x = PADDING_X as usize;
      let panel_y = (2 + PADDING_Y + CONSOLE_HEIGHT) as usize;
      let adj_col = (col as usize).saturating_sub(panel_x.saturating_sub(1));
      let adj_row = (row as usize).saturating_sub(panel_y);
      let position = crate::core::geom::Vec2::new(adj_col, adj_row);
      let offset = crate::core::geom::Vec2::zero();
      match kind {
        0 => {
          ui.grid.handle_mouse_press(offset, position);
        }
        1 => {
          ui.grid.handle_mouse_hold(offset, position);
        }
        _ => {}
      }
    }
  });
}

/// Notify the backend of a terminal resize.
#[wasm_bindgen]
pub fn wasm_resize(cols: u32, rows: u32) {
  use crate::view::consts::{CONSOLE_HEIGHT, PADDING_X, PADDING_Y};
  let w = cols as u16;
  let h = rows as u16;
  let grid_w = w.saturating_sub(PADDING_X * 2) as usize;
  let grid_h = h.saturating_sub(CONSOLE_HEIGHT + 2 + PADDING_Y * 2) as usize;
  UI.with(|u| {
    if let Some(ui) = u.borrow_mut().as_mut() {
      ui.backend.resize(w, h);
      ui.state.resize(w, h);
      ui.grid.resize(crate::core::geom::Vec2::new(grid_w, grid_h));
    }
  });
}

/// Set the regex input and trigger pattern matching.
#[wasm_bindgen]
pub fn wasm_set_input(pattern: String) {
  UI.with(|u| {
    if let Some(ui) = u.borrow_mut().as_mut() {
      ui.state.line_editor.set_content(&pattern);
      if !pattern.is_empty() {
        if let Some(ref tx) = ui.grid.regex_tx {
          let _ = tx.send(crate::core::engine::regex::Message::Solve(
            crate::core::engine::regex::EventData {
              text: ui.grid.text_contents(),
              pattern,
              flags: ui.state.flags.to_flag_str().to_string(),
              grid_width: ui.grid.grid.width,
            },
          ));
        }
      }
    }
  });
}

/// Load file contents into the grid.
#[wasm_bindgen]
pub fn wasm_load_file(contents: String) {
  UI.with(|u| {
    if let Some(ui) = u.borrow_mut().as_mut() {
      set_grid_contents(&mut ui.grid, contents);
    }
  });
}

/// Pop one raw MIDI message from the output queue.
#[wasm_bindgen]
pub fn wasm_take_midi_message() -> Option<Vec<u8>> {
  CTX.with(|c| {
    c.borrow_mut()
      .as_mut()
      .and_then(|ctx| ctx.midi.out_queue.pop_front())
  })
}
