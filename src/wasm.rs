/// WASM entry-point for anupars.
///
/// Exports four functions that JavaScript drives:
///   wasm_init(cols, rows)   – set up the whole application
///   wasm_step(elapsed_ms)   – advance one frame (~60 fps)
///   wasm_send_key(key_str)  – forward a key string from xterm.js
///   wasm_render()           – returns the ANSI string to write to xterm.js
///   wasm_resize(cols, rows) – notify the backend of a terminal resize
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use cursive::event::{Event, Key, MouseButton, MouseEvent};
use cursive::style::{BaseColor, Color, ColorPair, Effect};
use cursive::Vec2;
use wasm_bindgen::prelude::*;

use crate::app::{initialize_components, setup_ui};
use crate::core::engine::regex::RegexCache;
use crate::core::playhead::{Message as PlayheadMessage, Playhead};
use crate::core::timing::metronome::Metronome;

// ─── Thread-local staging buffers (declared early, used everywhere) ───────────

thread_local! {
  /// Events pushed by `wasm_send_key`, drained by `poll_event`.
  static EVENT_STAGE: RefCell<VecDeque<Event>> = RefCell::new(VecDeque::new());
  /// Pending resize (cols, rows) written by `wasm_resize`.
  static RESIZE_STAGE: RefCell<Option<(usize, usize)>> = RefCell::new(None);
  /// Last rendered ANSI frame, read by `wasm_render`.
  static ANSI_OUTPUT: RefCell<String> = RefCell::new(String::new());
}

// ─── Backend state ────────────────────────────────────────────────────────────

#[derive(Clone)]
struct ScreenCell {
  ch: char,
  fg: Color,
  bg: Color,
  reverse: bool,
}

impl Default for ScreenCell {
  fn default() -> Self {
    ScreenCell {
      ch: ' ',
      fg: Color::Dark(BaseColor::White),
      bg: Color::Dark(BaseColor::Black),
      reverse: false,
    }
  }
}

struct BackendState {
  cols: usize,
  rows: usize,
  cells: Vec<ScreenCell>,
  cursor: Vec2,
  fg: Color,
  bg: Color,
  reverse: bool,
}

impl BackendState {
  fn new(cols: usize, rows: usize) -> Self {
    Self {
      cols,
      rows,
      cells: vec![ScreenCell::default(); cols * rows],
      cursor: Vec2::zero(),
      fg: Color::Dark(BaseColor::White),
      bg: Color::Dark(BaseColor::Black),
      reverse: false,
    }
  }

  fn resize(&mut self, cols: usize, rows: usize) {
    self.cols = cols;
    self.rows = rows;
    self.cells = vec![ScreenCell::default(); cols * rows];
    self.cursor = Vec2::zero();
  }

  fn cell_idx(&self, x: usize, y: usize) -> Option<usize> {
    if x < self.cols && y < self.rows {
      Some(y * self.cols + x)
    } else {
      None
    }
  }

  fn render_ansi(&self) -> String {
    let mut out = String::with_capacity(self.cols * self.rows * 8);
    // hide cursor, move to top-left
    out.push_str("\x1b[?25l\x1b[H");

    let mut prev_fg: Option<Color> = None;
    let mut prev_bg: Option<Color> = None;
    let mut prev_reverse: Option<bool> = None;

    for row in 0..self.rows {
      for col in 0..self.cols {
        let cell = &self.cells[row * self.cols + col];
        let need_color = prev_fg.map_or(true, |f| f != cell.fg)
          || prev_bg.map_or(true, |b| b != cell.bg)
          || prev_reverse.map_or(true, |r| r != cell.reverse);
        if need_color {
          if cell.reverse {
            // Reset all then enable reverse video - the terminal inverts its
            // own default colours, giving white-bg/dark-text for the cursor.
            out.push_str("\x1b[0m\x1b[7m");
          } else {
            // Reset (clears any lingering reverse) then set explicit colours.
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
      if row + 1 < self.rows {
        out.push_str("\r\n");
      }
    }
    out.push_str("\x1b[0m\x1b[?25h");
    out
  }
}

// Color helpers

fn ansi_color(fg: Color, bg: Color) -> String {
  format!("{}{}", ansi_fg(fg), ansi_bg(bg))
}

fn base_idx(b: BaseColor) -> u8 {
  match b {
    BaseColor::Black => 0,
    BaseColor::Red => 1,
    BaseColor::Green => 2,
    BaseColor::Yellow => 3,
    BaseColor::Blue => 4,
    BaseColor::Magenta => 5,
    BaseColor::Cyan => 6,
    BaseColor::White => 7,
  }
}

fn ansi_fg(c: Color) -> String {
  match c {
    Color::Dark(b) => format!("\x1b[{}m", 30 + base_idx(b)),
    Color::Light(b) => format!("\x1b[{}m", 90 + base_idx(b)),
    Color::Rgb(r, g, b) => format!("\x1b[38;2;{r};{g};{b}m"),
    Color::RgbLowRes(r, g, b) => format!(
      "\x1b[38;5;{}m",
      16u16 + 36 * r as u16 + 6 * g as u16 + b as u16
    ),
    _ => "\x1b[39m".to_string(),
  }
}

fn ansi_bg(c: Color) -> String {
  match c {
    Color::Dark(b) => format!("\x1b[{}m", 40 + base_idx(b)),
    Color::Light(b) => format!("\x1b[{}m", 100 + base_idx(b)),
    Color::Rgb(r, g, b) => format!("\x1b[48;2;{r};{g};{b}m"),
    Color::RgbLowRes(r, g, b) => format!(
      "\x1b[48;5;{}m",
      16u16 + 36 * r as u16 + 6 * g as u16 + b as u16
    ),
    _ => "\x1b[49m".to_string(),
  }
}

// ─── Custom cursive Backend ───────────────────────────────────────────────────

struct XtermJsBackend {
  state: RefCell<BackendState>,
}

impl XtermJsBackend {
  fn new(cols: usize, rows: usize) -> Self {
    Self {
      state: RefCell::new(BackendState::new(cols, rows)),
    }
  }
}

impl cursive::backend::Backend for XtermJsBackend {
  fn poll_event(&mut self) -> Option<Event> {
    // Apply any pending resize first
    if let Some((cols, rows)) = RESIZE_STAGE.with(|s| s.borrow_mut().take()) {
      self.state.borrow_mut().resize(cols, rows);
    }
    // Drain staged events from wasm_send_key
    EVENT_STAGE.with(|e| e.borrow_mut().pop_front())
  }

  fn set_title(&mut self, _title: String) {}

  fn refresh(&mut self) {
    let rendered = self.state.borrow().render_ansi();
    ANSI_OUTPUT.with(|a| *a.borrow_mut() = rendered);
  }

  fn has_colors(&self) -> bool {
    true
  }

  fn screen_size(&self) -> Vec2 {
    let s = self.state.borrow();
    Vec2::new(s.cols, s.rows)
  }

  fn move_to(&self, pos: Vec2) {
    self.state.borrow_mut().cursor = pos;
  }

  fn print(&self, text: &str) {
    let mut s = self.state.borrow_mut();
    let cx = s.cursor.x;
    let cy = s.cursor.y;
    let fg = s.fg;
    let bg = s.bg;
    let reverse = s.reverse;
    let n = text.chars().count();
    for (i, ch) in text.chars().enumerate() {
      if let Some(idx) = s.cell_idx(cx + i, cy) {
        s.cells[idx] = ScreenCell {
          ch,
          fg,
          bg,
          reverse,
        };
      }
    }
    s.cursor.x += n;
  }

  fn clear(&self, color: Color) {
    let mut s = self.state.borrow_mut();
    let fg = s.fg;
    for cell in s.cells.iter_mut() {
      *cell = ScreenCell {
        ch: ' ',
        fg,
        bg: color,
        reverse: false,
      };
    }
    s.cursor = Vec2::zero();
  }

  fn set_color(&self, colors: ColorPair) -> ColorPair {
    let mut s = self.state.borrow_mut();
    let prev = ColorPair {
      front: s.fg,
      back: s.bg,
    };
    s.fg = colors.front;
    s.bg = colors.back;
    prev
  }

  fn set_effect(&self, effect: Effect) {
    if effect == Effect::Reverse {
      self.state.borrow_mut().reverse = true;
    }
  }

  fn unset_effect(&self, effect: Effect) {
    if effect == Effect::Reverse {
      self.state.borrow_mut().reverse = false;
    }
  }
  fn name(&self) -> &str {
    "xterm.js"
  }
}

// ─── WASM application context ─────────────────────────────────────────────────

struct WasmCtx {
  playhead: Arc<Playhead>,
  regex_tx: Sender<crate::core::engine::regex::Message>,
  regex_handler: crate::core::engine::regex::RegExpHandler,
  regex_cache: RegexCache,
  metronome: Metronome,
  midi: crate::core::io::midi::Midi,
  clock_accum_ms: f64,
  clock_tick: usize,
}

impl WasmCtx {
  fn tick(&mut self, elapsed_ms: f64) {
    // 1. Drain metronome control messages (StartStop, Tempo, …)
    self.metronome.wasm_tick();

    // 2. Advance internal clock if playing
    if self.metronome.wasm_is_playing() {
      let bpm = self.metronome.wasm_current_bpm().max(1.0);
      let ms_per_tick = 60_000.0 / (bpm * 16.0); // 16 ticks per beat
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

    // 3. Drain the playhead message queue
    self.playhead.wasm_tick();

    // 4. Process queued UI updates → push closures to cb_sink
    self.playhead.wasm_tick_ui(&self.regex_tx);

    // 5. Drain the regex handler
    self.regex_handler.wasm_tick(&mut self.regex_cache);

    // 6. Process MIDI messages and fire pending note-offs
    self.midi.wasm_tick(self.clock_tick);
  }
}

// ─── Thread-local runner storage ─────────────────────────────────────────────

thread_local! {
  static RUNNER: RefCell<Option<cursive::CursiveRunner<cursive::Cursive>>> =
    RefCell::new(None);
  static CTX: RefCell<Option<WasmCtx>> = RefCell::new(None);
}

// ─── Key parsing ──────────────────────────────────────────────────────────────

fn parse_key(s: &str) -> Vec<Event> {
  let mut out = Vec::new();
  match s {
    "\r" | "\n" => out.push(Event::Key(Key::Enter)),
    "\x1b" => out.push(Event::Key(Key::Esc)),
    "\x7f" | "\x08" => out.push(Event::Key(Key::Backspace)),
    "\t" => out.push(Event::Key(Key::Tab)),
    "\x1b[A" | "\x1bOA" => out.push(Event::Key(Key::Up)),
    "\x1b[B" | "\x1bOB" => out.push(Event::Key(Key::Down)),
    "\x1b[C" | "\x1bOC" => out.push(Event::Key(Key::Right)),
    "\x1b[D" | "\x1bOD" => out.push(Event::Key(Key::Left)),
    "\x1b[H" | "\x1bOH" => out.push(Event::Key(Key::Home)),
    "\x1b[F" | "\x1bOF" => out.push(Event::Key(Key::End)),
    "\x1b[3~" => out.push(Event::Key(Key::Del)),
    "\x1b[5~" => out.push(Event::Key(Key::PageUp)),
    "\x1b[6~" => out.push(Event::Key(Key::PageDown)),
    "\x1b[2~" => out.push(Event::Key(Key::Ins)),
    // Alt+arrow keys (xterm modifier format)
    "\x1b[1;3A" => out.push(Event::Alt(Key::Up)),
    "\x1b[1;3B" => out.push(Event::Alt(Key::Down)),
    "\x1b[1;3C" => out.push(Event::Alt(Key::Right)),
    "\x1b[1;3D" => out.push(Event::Alt(Key::Left)),
    s => {
      let bytes = s.as_bytes();
      if bytes.len() == 1 {
        let b = bytes[0];
        if b >= 1 && b <= 26 {
          // Ctrl+letter
          out.push(Event::CtrlChar((b'a' + b - 1) as char));
        } else if b >= 0x20 {
          out.push(Event::Char(b as char));
        }
      } else if bytes.len() >= 2 && bytes[0] == b'\x1b' {
        // Alt+key: xterm.js sends ESC followed by the key sequence
        let rest = &s[1..];
        let rest_bytes = rest.as_bytes();
        if rest_bytes.len() == 1 {
          let b = rest_bytes[0];
          if b >= 1 && b <= 26 {
            // Alt+Ctrl+letter
            out.push(Event::AltChar((b'a' + b - 1) as char));
          } else if b >= 0x20 && b < 0x7f {
            // Alt+printable ASCII
            out.push(Event::AltChar(b as char));
          }
        } else {
          // Alt+multi-char: parse the inner sequence and wrap in Alt
          for ev in parse_key(rest) {
            match ev {
              Event::Key(k) => out.push(Event::Alt(k)),
              Event::Char(c) => out.push(Event::AltChar(c)),
              other => out.push(other),
            }
          }
        }
      } else {
        // Multi-byte UTF-8 (emoji, accented chars, etc.)
        for ch in s.chars() {
          if !ch.is_control() {
            out.push(Event::Char(ch));
          }
        }
      }
    }
  }
  out
}

// ─── Public WASM API ──────────────────────────────────────────────────────────

/// Initialise the application. Call once before anything else.
/// `cols` and `rows` should match your xterm.js terminal dimensions.
#[wasm_bindgen]
pub fn wasm_init(cols: u32, rows: u32) {
  console_error_panic_hook::set_once();

  let mut components = initialize_components();
  setup_ui(&mut components);

  // Extract parts needed for the WASM context
  let playhead = components.playhead;
  let regex_tx = components.regex_handler.tx.clone();
  let regex_handler = components.regex_handler;
  let metronome = components.metronome;
  let midi = components.midi;

  // Build the runner with our custom backend
  let backend = Box::new(XtermJsBackend::new(cols as usize, rows as usize));
  let runner = components.cursive.into_runner(backend);

  let ctx = WasmCtx {
    playhead,
    regex_tx,
    regex_handler,
    regex_cache: RegexCache::new(),
    metronome,
    midi,
    clock_accum_ms: 0.0,
    clock_tick: 0,
  };

  RUNNER.with(|r| *r.borrow_mut() = Some(runner));
  CTX.with(|c| *c.borrow_mut() = Some(ctx));
}

/// Advance one frame. Call at ~60 fps from `requestAnimationFrame`.
/// `elapsed_ms` - milliseconds since the previous call.
#[wasm_bindgen]
pub fn wasm_step(elapsed_ms: f64) {
  // Tick all background processing (clock, playhead, regex, UI queue)
  CTX.with(|c| {
    if let Some(ctx) = c.borrow_mut().as_mut() {
      ctx.tick(elapsed_ms);
    }
  });

  // Let cursive process all pending events + cb_sink callbacks, then redraw
  RUNNER.with(|r| {
    if let Some(runner) = r.borrow_mut().as_mut() {
      runner.process_events();
      runner.refresh();
    }
  });
}

/// Forward keyboard input from xterm.js `terminal.onData(data => wasm_send_key(data))`.
/// Events are staged and consumed on the next `wasm_step` call.
#[wasm_bindgen]
pub fn wasm_send_key(key: String) {
  let events = parse_key(&key);
  EVENT_STAGE.with(|e| e.borrow_mut().extend(events));
}

/// Returns the ANSI byte-string to write to `terminal.write()`.
/// Call after `wasm_step`.
#[wasm_bindgen]
pub fn wasm_render() -> String {
  ANSI_OUTPUT.with(|a| a.borrow().clone())
}

/// Forward a mouse event from the browser.
/// `kind`   – 0 = press, 1 = hold/drag, 2 = release
/// `button` – 0 = left, 1 = middle, 2 = right
/// `col`, `row` – terminal cell coordinates (0-based)
#[wasm_bindgen]
pub fn wasm_send_mouse(kind: u8, button: u8, col: u32, row: u32) {
  let btn = match button {
    1 => MouseButton::Middle,
    2 => MouseButton::Right,
    _ => MouseButton::Left,
  };
  let event = match kind {
    0 => MouseEvent::Press(btn),
    1 => MouseEvent::Hold(btn),
    2 => MouseEvent::Release(btn),
    _ => return,
  };
  EVENT_STAGE.with(|e| {
    e.borrow_mut().push_back(Event::Mouse {
      offset: Vec2::zero(),
      position: Vec2::new(col as usize, row as usize),
      event,
    });
  });
}

/// Notify the backend of a terminal resize.
#[wasm_bindgen]
pub fn wasm_resize(cols: u32, rows: u32) {
  RESIZE_STAGE.with(|s| *s.borrow_mut() = Some((cols as usize, rows as usize)));
}

/// Set the regex input field and trigger pattern matching.
/// Equivalent to the user typing into the "RGXP" input in the console.
#[wasm_bindgen]
pub fn wasm_set_input(pattern: String) {
  RUNNER.with(|r| {
    if let Some(runner) = r.borrow_mut().as_mut() {
      let cb = runner.call_on_name(
        crate::view::consts::regex_input_unit_view,
        |v: &mut cursive::views::EditView| v.set_content(pattern),
      );
      if let Some(cb) = cb {
        cb(runner);
      }
    }
  });
}

/// Load text content into the grid editor (WASM file picker replacement).
/// Call this from JS after reading a file with `showOpenFilePicker`.
#[wasm_bindgen]
pub fn wasm_load_file(contents: String) {
  RUNNER.with(|r| {
    if let Some(runner) = r.borrow_mut().as_mut() {
      crate::view::menubar::set_contents(runner, contents);
    }
  });
}

/// Pop one raw MIDI message (3 bytes) from the output queue.
/// Returns `undefined` when the queue is empty.
/// JS: `let msg; while ((msg = wasm_take_midi_message()) !== undefined) midiOut.send(msg);`
#[wasm_bindgen]
pub fn wasm_take_midi_message() -> Option<Vec<u8>> {
  CTX.with(|c| {
    c.borrow_mut()
      .as_mut()
      .and_then(|ctx| ctx.midi.out_queue.pop_front())
  })
}
