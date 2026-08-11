use std::mem;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;

use ringbuffer::{AllocRingBuffer, RingBuffer};
use symspell_rs::SymSpell;

use crate::core::{consts, engine::regex};
use crate::view::grid::GridEditor;
use crate::view::rect::Rect;

#[derive(Clone, Debug)]
pub enum RplPendingState {
  Empty,
  Waiting(String, Rect),
  Armed(String, Rect),
}

pub struct TextRevealAnimState {
  pub sym_text: String,
  pub area: Rect,
  pub base_text: String,
  pub frame: usize,
  pub total_frames: usize,
}

/// Snapshot tuple passed from the loop thread into the cb_sink closure.
/// (sym_text, area, base_text, frame, total_frames)
pub type AnimTick = (String, Rect, String, usize, usize);

pub struct SymSpellState {
  buf_buf: Arc<Mutex<AllocRingBuffer<char>>>,
  rpl_state: Arc<Mutex<RplPendingState>>,
  text_reveal_anim: Arc<Mutex<Option<TextRevealAnimState>>>,
  symspell: Arc<Mutex<Option<SymSpell>>>,
}

impl Default for SymSpellState {
  fn default() -> Self {
    Self::new()
  }
}

impl SymSpellState {
  pub fn new() -> Self {
    let symspell: Arc<Mutex<Option<SymSpell>>> = Arc::new(Mutex::new(None));

    #[cfg(not(target_arch = "wasm32"))]
    {
      let symspell_bg = Arc::clone(&symspell);
      thread::Builder::new()
        .name(consts::THREAD_NAME_SYMSPELL_LOADER.to_string())
        .spawn(move || {
          let mut inner = SymSpell::new(2, None, 7, 1);
          let _ = inner.load_dictionary(Path::new(consts::FREQ_DICT_PATH), 0, 1, " ");
          *symspell_bg.lock().unwrap() = Some(inner);
        })
        .expect("Failed to spawn symspell loader thread");
    }

    #[cfg(target_arch = "wasm32")]
    {
      const DICT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/data/frequency_dictionary_en_10k.txt"
      ));
      let mut inner = SymSpell::new(2, None, 7, 1);
      for line in DICT.lines() {
        inner.load_dictionary_line(line, 0, 1, " ");
      }
      *symspell.lock().unwrap() = Some(inner);
    }

    SymSpellState {
      buf_buf: Arc::new(Mutex::new(AllocRingBuffer::new(consts::TMP_BUF_SIZE))),
      rpl_state: Arc::new(Mutex::new(RplPendingState::Empty)),
      text_reveal_anim: Arc::new(Mutex::new(None)),
      symspell,
    }
  }

  /// Advance animation by one frame. Returns a snapshot if a frame should be
  /// rendered this tick, or `None` if the animation is idle.
  /// Called from the UI batch-processor loop thread each 16 ms tick.
  pub fn advance_anim_frame(&self) -> Option<AnimTick> {
    let mut guard = self.text_reveal_anim.lock().unwrap();
    let snap = guard.as_ref().map(|a| {
      (
        a.sym_text.clone(),
        a.area,
        a.base_text.clone(),
        a.frame,
        a.total_frames,
      )
    });
    if let Some(a) = guard.as_mut() {
      a.frame += 1;
      if a.frame >= a.total_frames {
        *guard = None;
      }
    }
    snap
  }

  /// Render one animation frame into the grid.
  /// On the final frame, re-runs the active regex pattern against the new text.
  pub fn render_anim_tick(
    &self,
    grid: &mut GridEditor,
    app_state: &mut crate::state::AppState,
    tick: AnimTick,
    regex_tx: &Sender<regex::Message>,
  ) {
    let (sym_text, area, base_text, frame, total_frames) = tick;
    let is_last = frame + 1 >= total_frames;
    let intermediate = compute_anim_frame(&base_text, &area, &sym_text, frame);

    grid.update_text_contents(&intermediate);
    grid.update_grid_src();
    let gw = grid.grid.width;

    if is_last {
      let pattern = app_state.line_editor.content().to_string();
      if !pattern.is_empty() {
        let _ = regex_tx.send(regex::Message::Solve(regex::EventData {
          text: intermediate,
          pattern,
          flags: "i".to_string(),
          grid_width: gw,
        }));
      }
    }
  }

  /// Append a space to the buffer and refresh the status field.
  pub fn handle_buf_append_space(&self, app_state: &mut crate::state::AppState) {
    let display = {
      let mut buf = self.buf_buf.lock().unwrap();
      let last = buf.back().copied();
      if last != Some(' ') {
        buf.enqueue(' ');
      }
      let s: String = buf.iter().collect();
      if s.is_empty() {
        "-".to_string()
      } else {
        s
      }
    };
    app_state.buf_status = display;
  }

  /// Append the character at grid index `idx` to the buffer, then run
  /// SymSpell on the accumulated word and update the status fields.
  pub fn handle_buf_append(
    &self,
    grid: &GridEditor,
    app_state: &mut crate::state::AppState,
    idx: usize,
  ) {
    let ch = grid.grid.data.get(idx).copied().unwrap_or('\0');

    if ch.is_alphabetic() {
      let new_tmp = {
        let mut buf = self.buf_buf.lock().unwrap();
        buf.enqueue(ch);
        buf.iter().collect::<String>()
      };
      app_state.buf_status = new_tmp.clone();
      let sym_result = {
        let ss = self.symspell.lock().unwrap();
        match ss.as_ref() {
          None => String::new(),
          Some(ss) => ss
            .lookup_compound(new_tmp.trim(), 2, &None, false)
            .into_iter()
            .next()
            .map(|s| s.term)
            .unwrap_or_default(),
        }
      };
      if !sym_result.is_empty() {
        app_state.sym_status = sym_result;
      }
    }
  }

  /// Advance the replacement state machine and, when a suggestion becomes
  /// armed, kick off character-by-character animation.
  pub fn handle_rpl_cycle(
    &self,
    grid: &mut GridEditor,
    app_state: &mut crate::state::AppState,
    old_area: Rect,
  ) {
    let sym = app_state.sym_status.clone();

    let mut state = self.rpl_state.lock().unwrap();
    let apply_opt: Option<(String, Rect)> = match mem::replace(&mut *state, RplPendingState::Empty)
    {
      RplPendingState::Armed(armed_sym, armed_area) => {
        *state = if !sym.is_empty() && sym != "-" {
          RplPendingState::Waiting(sym, old_area)
        } else {
          RplPendingState::Empty
        };
        Some((armed_sym, armed_area))
      }
      RplPendingState::Waiting(w_sym, w_area) => {
        *state = RplPendingState::Armed(w_sym, w_area);
        None
      }
      RplPendingState::Empty => {
        *state = if !sym.is_empty() && sym != "-" {
          RplPendingState::Waiting(sym, old_area)
        } else {
          RplPendingState::Empty
        };
        None
      }
    };

    app_state.rpl_status = match &*state {
      RplPendingState::Armed(s, _) => format!("[armed] {}", s),
      RplPendingState::Waiting(s, _) => s.clone(),
      RplPendingState::Empty => "-".to_string(),
    };
    drop(state);

    if let Some((sym_text, area)) = apply_opt {
      let base_text = grid.text_contents();
      let total_frames = sym_text.chars().count() * 5 + 1;
      *self.text_reveal_anim.lock().unwrap() = Some(TextRevealAnimState {
        sym_text,
        area,
        base_text,
        frame: 0,
        total_frames,
      });
    }
  }
}

/// Build one intermediate frame of the animation.
///
/// Each character occupies 5 frames at 16 ms each (~80 ms per char):
///   frames 0-2 → '_'   (placeholder)
///   frames 3-4 → '▌'   (block cursor)
///   thereafter  → final character (locked in)
///
/// Characters reveal strictly left-to-right. Characters not yet reached keep
/// the original content from `base_text` (via `splice_text_at_area`'s early-stop
/// behaviour). Spaces skip the animation phases and settle immediately.
fn compute_anim_frame(base_text: &str, area: &Rect, sym_text: &str, frame: usize) -> String {
  let sym_chars: Vec<char> = sym_text.chars().collect();
  let n = sym_chars.len();
  let char_idx = frame / 5;
  let sub_phase = (frame % 5) / 3; // 0 = '_' (frames 0-2), 1 = '▌' (frames 3-4)

  if char_idx >= n {
    return splice_text_at_area(base_text, area, sym_text);
  }

  let intermediate: String = sym_chars[..=char_idx]
    .iter()
    .enumerate()
    .map(|(i, &c)| {
      if i < char_idx || c == ' ' {
        c
      } else if sub_phase == 0 {
        '_'
      } else {
        '▌'
      }
    })
    .collect();

  splice_text_at_area(base_text, area, &intermediate)
}

/// Splice `sym` into `text` at the rectangle described by `area`.
/// Characters in `sym` overwrite the corresponding cells; cells beyond the
/// length of `sym` keep their original content (no padding or truncation).
fn splice_text_at_area(text: &str, area: &Rect, sym: &str) -> String {
  let mut lines: Vec<String> = text.split('\n').map(str::to_string).collect();
  let sym_chars: Vec<char> = sym.chars().collect();
  let top = area.top_left.y;
  let left = area.top_left.x;
  let h = area.height();
  let w = area.width();
  'outer: for row in 0..h {
    let line_idx = top + row;
    if line_idx >= lines.len() {
      lines.resize(line_idx + 1, String::new());
    }
    let line = &mut lines[line_idx];
    let row_start = row * w;
    if row_start >= sym_chars.len() {
      break 'outer;
    }
    while line.chars().count() < left + w {
      line.push('\0');
    }
    let mut chars: Vec<char> = line.chars().collect();
    for col in 0..w {
      let sym_idx = row_start + col;
      if sym_idx >= sym_chars.len() {
        break;
      }
      chars[left + col] = sym_chars[sym_idx];
    }
    *line = chars.into_iter().collect();
  }
  lines.join("\n")
}
