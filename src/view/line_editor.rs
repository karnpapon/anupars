use crate::terminal::buffer::ScreenBuffer;
use crate::view::printer::{apply_style, CellStyle};

pub enum LineEditorAction {
  None,
  Submit(String),
  Changed,
}

pub struct LineEditor {
  pub buf: String,
  pub cursor: usize,
}

impl Default for LineEditor {
  fn default() -> Self {
    Self::new()
  }
}

impl LineEditor {
  pub fn new() -> Self {
    Self {
      buf: String::new(),
      cursor: 0,
    }
  }

  pub fn set_content(&mut self, s: &str) {
    self.buf = s.to_string();
    self.cursor = self.buf.len();
  }

  pub fn content(&self) -> &str {
    &self.buf
  }

  fn prev_char_boundary(&self) -> usize {
    let mut i = self.cursor;
    loop {
      i -= 1;
      if self.buf.is_char_boundary(i) {
        return i;
      }
    }
  }

  fn next_char_boundary(&self) -> usize {
    let mut i = self.cursor + 1;
    while i <= self.buf.len() && !self.buf.is_char_boundary(i) {
      i += 1;
    }
    i
  }

  /// Handle a crossterm KeyEvent and return what happened.
  #[cfg(not(target_arch = "wasm32"))]
  pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> LineEditorAction {
    use crossterm::event::{KeyCode, KeyModifiers};

    match (key.modifiers, key.code) {
      (KeyModifiers::NONE | KeyModifiers::SHIFT, KeyCode::Char(c)) => {
        self.buf.insert(self.cursor, c);
        self.cursor += c.len_utf8();
        LineEditorAction::Changed
      }
      (_, KeyCode::Backspace) => {
        if self.cursor > 0 {
          let prev = self.prev_char_boundary();
          self.buf.drain(prev..self.cursor);
          self.cursor = prev;
          LineEditorAction::Changed
        } else {
          LineEditorAction::None
        }
      }
      (_, KeyCode::Delete) => {
        if self.cursor < self.buf.len() {
          let next = self.next_char_boundary();
          self.buf.drain(self.cursor..next);
          LineEditorAction::Changed
        } else {
          LineEditorAction::None
        }
      }
      (_, KeyCode::Left) => {
        if self.cursor > 0 {
          self.cursor = self.prev_char_boundary();
        }
        LineEditorAction::None
      }
      (_, KeyCode::Right) => {
        if self.cursor < self.buf.len() {
          self.cursor = self.next_char_boundary();
        }
        LineEditorAction::None
      }
      (_, KeyCode::Home) | (KeyModifiers::CONTROL, KeyCode::Char('a')) => {
        self.cursor = 0;
        LineEditorAction::None
      }
      (_, KeyCode::End) | (KeyModifiers::CONTROL, KeyCode::Char('e')) => {
        self.cursor = self.buf.len();
        LineEditorAction::None
      }
      (KeyModifiers::CONTROL, KeyCode::Char('u')) => {
        self.buf.drain(..self.cursor);
        self.cursor = 0;
        LineEditorAction::Changed
      }
      (KeyModifiers::CONTROL, KeyCode::Char('k')) => {
        self.buf.truncate(self.cursor);
        LineEditorAction::Changed
      }
      (_, KeyCode::Enter) => LineEditorAction::Submit(self.buf.clone()),
      _ => LineEditorAction::None,
    }
  }

  /// Draw the editor into `buf` at `(x, y)` with the given `width`.
  /// The visible window scrolls to keep the cursor in view.
  /// When `focused` is true the cursor position is highlighted.
  pub fn draw(&self, buf: &mut ScreenBuffer, x: u16, y: u16, width: u16, focused: bool) {
    use crate::terminal::cell::Color;

    let chars: Vec<char> = self.buf.chars().collect();
    let cursor_char = self.buf[..self.cursor].chars().count();

    let visible_start = if cursor_char >= width as usize {
      cursor_char + 1 - width as usize
    } else {
      0
    };

    let text_style = CellStyle {
      fg: Color::Rgb(0, 0, 0),
      bg: Color::Rgb(255, 255, 255),
      reverse: false,
    };
    let cursor_style = CellStyle {
      fg: Color::Rgb(255, 255, 255),
      bg: Color::Rgb(0, 0, 0),
      reverse: false,
    };
    let empty_style = CellStyle {
      fg: Color::Rgb(0, 0, 0),
      bg: Color::Rgb(255, 255, 255),
      reverse: false,
    };

    for col in 0..width {
      let char_idx = visible_start + col as usize;
      let is_cursor = focused && char_idx == cursor_char;
      let ch = chars.get(char_idx).copied();

      let (draw_ch, style) = match ch {
        Some(c) if is_cursor => (c, cursor_style),
        Some(c) => (c, text_style),
        None if is_cursor => ('_', cursor_style),
        None => ('_', empty_style),
      };

      if let Some(cell) = buf.get_mut(x + col, y) {
        apply_style(cell, draw_ch, style);
      }
    }
  }
}
