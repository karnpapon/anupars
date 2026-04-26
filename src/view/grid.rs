use std::sync::mpsc::Sender;

use crate::core::geom::Vec2;

use ringbuffer::RingBuffer;

use crate::core::playhead::queue::EVENT_OPERATORS;
use crate::core::playhead::queue::QUEUE_OPERATORS;
use crate::core::playhead::PlayheadUI;
use crate::core::{consts, playhead::queue::PendingJumpPosition};
use crate::terminal::buffer::ScreenBuffer;
use crate::terminal::cell::Color;
use crate::view::printer::{apply_style, CellStyle, Matrix};

use consts::BASE_OCTAVE;
use consts::KEYBOARD_MARGIN_BOTTOM;
use consts::KEYBOARD_MARGIN_LEFT;
use consts::KEYBOARD_MARGIN_TOP;
use consts::NOTE_NAMES;
use consts::QUEUE_MARGIN_RIGHT;

use crate::core::engine::regex;
use crate::core::playhead::{Direction, Message as PlayheadMessage};

pub struct GridEditor {
  size: Vec2,
  pub playhead_tx: Sender<PlayheadMessage>,
  pub grid: Matrix<char>,
  pub text_contents: Option<String>,
  pub playhead_ui: PlayheadUI,
  pub show_keyboard: bool,
  pub is_canvas_focused: bool,
  pub regex_tx: Option<Sender<regex::Message>>,
  pub last_pattern: String,
  pub last_flag_str: String,
  pub sgr_leak_state: u8,
}

impl GridEditor {
  pub fn new(playhead_tx: Sender<PlayheadMessage>) -> GridEditor {
    GridEditor {
      size: Vec2::zero(),
      playhead_tx,
      grid: Matrix::new(0, 0, '\0'),
      text_contents: None,
      playhead_ui: PlayheadUI::new(),
      show_keyboard: true,
      is_canvas_focused: false,
      regex_tx: None,
      last_pattern: String::new(),
      last_flag_str: String::new(),
      sgr_leak_state: 0,
    }
  }

  /// Map Y position to MIDI note information (for left keyboard)
  /// Y increases downward, so higher Y = lower note (inverted keyboard)
  pub fn y_to_note_left(&self, y: usize) -> (f32, u8, &'static str) {
    let total_rows = self.grid.height;
    if total_rows == 0 {
      return (0.0, BASE_OCTAVE, "C");
    }

    let (note_index, octave) = self.playhead_ui.scale_mode_left.pos_to_scale_note(
      y,
      total_rows,
      BASE_OCTAVE,
      self.playhead_ui.scale_root_left.to_root_offset(),
    );

    (
      note_index,
      octave,
      NOTE_NAMES[note_index.round() as usize % 12],
    )
  }

  /// Map Y position to MIDI note information (for top keyboard)
  /// Y increases downward, so higher Y = lower note (inverted keyboard)
  pub fn y_to_note_top(&self, y: usize) -> (f32, u8, &'static str) {
    let total_rows = self.grid.height;
    if total_rows == 0 {
      return (0.0, BASE_OCTAVE, "C");
    }

    let (note_index, octave) = self.playhead_ui.scale_mode_top.pos_to_scale_note(
      y,
      total_rows,
      BASE_OCTAVE,
      self.playhead_ui.scale_root_top.to_root_offset(),
    );

    (
      note_index,
      octave,
      NOTE_NAMES[note_index.round() as usize % 12],
    )
  }

  pub fn update_text_contents(&mut self, contents: &str) {
    let contents = contents.replace("\r\n", "\n").replace("\r", "\n");
    self.text_contents = Some(contents);
  }

  pub fn update_grid_src(&mut self) {
    if self.text_contents.as_ref().is_none() {
      return;
    };

    let cols: usize = self.grid.width;
    let rows: usize = self.grid.height;

    for y in 0..rows {
      for x in 0..cols {
        self.grid.set(x, y, '\0');
      }
    }

    let mut x = 0;
    let mut y = 0;

    // Iterate through characters, preserving newlines
    for ch in self.text_contents.as_ref().unwrap().chars() {
      if y >= rows {
        break; // Stop if we've filled all rows
      }

      if ch == '\n' {
        // Fill rest of current row with '\0' (will render as rest/dot)
        while x < cols {
          self.grid.set(x, y, '\0');
          x += 1;
        }
        x = 0;
        y += 1;
      } else {
        // Place character at current position.
        // Spaces are treated as empty cells ('\0') so they render as '.'
        // rather than an invisible blank, consistent with padding cells.
        let cell = if ch == ' ' { '\0' } else { ch };
        if x < cols {
          self.grid.set(x, y, cell);
          x += 1;

          // If we've reached end of row, move to next row
          if x >= cols {
            x = 0;
            y += 1;
          }
        }
      }
    }
  }

  fn grid_size_xy(&self) -> Vec2 {
    Vec2::new(self.grid.width, self.grid.height)
  }

  pub fn start_aim_if_needed(&mut self) {
    if self.playhead_ui.aimed_area.is_none() {
      let _ = self.playhead_tx.send(PlayheadMessage::StartAim());
    }
  }

  pub fn update_aim(&self, dir: Direction, step: usize) -> bool {
    let grid_size = self.grid_size_xy();
    let _ = self
      .playhead_tx
      .send(PlayheadMessage::UpdateAim(dir, grid_size, step));
    false
  }

  pub fn commit_or_move(&mut self, dir: Direction) -> bool {
    if self.playhead_ui.aimed_area.is_some() {
      let _ = self.playhead_tx.send(PlayheadMessage::CommitAim());
      return false;
    }
    let grid_size = self.grid_size_xy();
    let _ = self.playhead_tx.send(PlayheadMessage::Move(dir, grid_size));
    false
  }

  pub fn alt_action(&self, dir: Direction, step: usize) -> bool {
    let grid_size = self.grid_size_xy();
    if self.playhead_ui.aimed_area.is_some() {
      let _ = self
        .playhead_tx
        .send(PlayheadMessage::UpdateAim(dir, grid_size, step));
    } else {
      let _ = self.playhead_tx.send(PlayheadMessage::Leap(dir, grid_size));
    }
    false
  }

  pub fn scale_action(&self, dir: (i32, i32)) -> bool {
    let _ = self.playhead_tx.send(PlayheadMessage::Scale(dir));
    false
  }

  pub fn handle_mouse_press(&mut self, offset: Vec2, position: Vec2) -> bool {
    let x_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_LEFT
    } else {
      0
    };
    let y_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_TOP
    } else {
      0
    };

    let adjusted_position = Vec2::new(
      position.x.saturating_sub(x_offset),
      position.y.saturating_sub(y_offset),
    );

    self
      .playhead_tx
      .send(PlayheadMessage::SetCurrentPos(adjusted_position, offset))
      .unwrap();
    let grid_size = (self.grid.width, self.grid.height).into();
    self
      .playhead_tx
      .send(PlayheadMessage::Move(Direction::Idle, grid_size))
      .unwrap();
    self
      .playhead_tx
      .send(PlayheadMessage::UpdateInfoStatusView())
      .unwrap();

    true
  }

  pub fn handle_mouse_hold(&mut self, offset: Vec2, position: Vec2) -> bool {
    // Adjust position to account for keyboard margins
    let x_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_LEFT
    } else {
      0
    };
    let y_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_TOP
    } else {
      0
    };

    let pos_x = position.x.saturating_sub(x_offset + 1);
    let pos_y = position.y.saturating_sub(offset.y + y_offset);

    // prevent dragging above or left of the origin flips the area.
    let origin = self.playhead_ui.playhead_pos;
    let clamped_x = pos_x.max(origin.x + 1);
    let clamped_y = pos_y.max(origin.y + 1);

    self
      .playhead_tx
      .send(PlayheadMessage::SetGridArea((clamped_x, clamped_y).into()))
      .unwrap();

    false
  }

  pub fn handle_split_char(&mut self, c: char) -> bool {
    let (v, h): (usize, usize) = match c {
      '0' => (1, 1),
      '1' => (1, 1),
      '2' => (2, 1),
      '3' => (3, 1),
      '4' => (4, 1),
      '5' => (4, 2),
      '6' => (4, 3),
      '7' => (4, 4),
      _ => (1, 1),
    };
    self.playhead_ui.grid_v_splits = v;
    self.playhead_ui.grid_h_splits = h;
    let _ = self.playhead_tx.send(PlayheadMessage::SetGridSplits(v, h));

    true
  }

  pub fn resize(&mut self, size: Vec2) {
    let grid_width = if self.show_keyboard {
      size
        .x
        .saturating_sub(KEYBOARD_MARGIN_LEFT + QUEUE_MARGIN_RIGHT)
    } else {
      size.x
    };

    let grid_height = if self.show_keyboard {
      size
        .y
        .saturating_sub(KEYBOARD_MARGIN_TOP + KEYBOARD_MARGIN_BOTTOM)
    } else {
      size.y
    };

    self.grid = Matrix::new(grid_width, grid_height, '\0');
    self.size = size;
    // Repopulate grid from cached text after dimension change
    self.update_grid_src();
    // Update grid width and height for precise timing calculations and note mapping
    let _ = self
      .playhead_tx
      .send(PlayheadMessage::SetGridSize(grid_width, grid_height));
    // Ensure playhead stays within new bounds
    let _ = self.playhead_tx.send(PlayheadMessage::Move(
      Direction::Idle,
      (grid_width, grid_height).into(),
    ));
  }

  pub fn clear_contents(&mut self) {
    self.grid = Matrix::new(self.grid.width, self.grid.height, '\0');
  }

  pub fn text_contents(&self) -> String {
    self
      .text_contents
      .as_ref()
      .unwrap_or(&"".to_string())
      .to_string()
  }

  fn draw_keyboard_top_to_buf(&self, buf: &mut ScreenBuffer, x_off: u16, y_off: u16) {
    if !self.show_keyboard || self.grid.height == 0 || self.grid.width == 0 {
      return;
    }
    let abs_active_x = self.playhead_ui.playhead_pos.x + self.playhead_ui.actived_pos.x;
    for x in 0..self.grid.width {
      let y_pos = x % self.grid.height;
      let (note_index, octave, note_name) = self.y_to_note_top(y_pos);
      let is_black_key = matches!(note_index.round() as u8, 1 | 3 | 6 | 8 | 10);

      let (fg, bg) = if x == abs_active_x {
        if note_name == "C" {
          (Color::Rgb(0, 0, 0), Color::Rgb(255, 255, 255))
        } else {
          (Color::Rgb(255, 255, 255), Color::Reset)
        }
      } else if note_name == "C" {
        (Color::Rgb(0, 0, 0), Color::Rgb(100, 100, 100))
      } else {
        (Color::Rgb(100, 100, 100), Color::Reset)
      };
      let style = CellStyle {
        fg,
        bg,
        reverse: false,
      };

      let cx = x_off + x as u16;
      if is_black_key {
        if let Some(c) = buf.get_mut(cx, y_off) {
          apply_style(c, '#', style);
        }
      } else if note_name == "C" {
        if let Some(c) = buf.get_mut(cx, y_off) {
          apply_style(c, ' ', style);
        }
        if let Some(c) = buf.get_mut(cx, y_off + 1) {
          apply_style(c, octave.to_string().chars().next().unwrap_or(' '), style);
        }
      } else if let Some(c) = buf.get_mut(cx, y_off) {
        apply_style(c, '━', style);
      }
    }
  }

  fn draw_keyboard_left_to_buf(&self, buf: &mut ScreenBuffer, x_off: u16, y_off: u16) {
    if !self.show_keyboard || self.grid.height == 0 {
      return;
    }
    for y in 0..self.grid.height {
      let (note_index, octave, note_name) = self.y_to_note_left(y);
      let is_black_key = matches!(note_index.round() as u8, 1 | 3 | 6 | 8 | 10);
      let r = if is_black_key { 50u8 } else { 100u8 };
      let style = CellStyle::fg_rgb(r, r, r);

      // always write exactly 3 columns so shorter names (e.g. "F3") don't
      // leave stale chars from a previously longer name (e.g. "F#3") at col 2
      let label = format!("{:<3}", format!("{}{}", note_name, octave));
      for (i, ch) in label.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + i as u16, y_off + y as u16) {
          apply_style(c, ch, style);
        }
      }
      let dots = ":".repeat(KEYBOARD_MARGIN_LEFT.saturating_sub(6));
      for (i, ch) in dots.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + 3 + i as u16, y_off + y as u16) {
          apply_style(c, ch, style);
        }
      }
      let sym = if note_name == "C" { '┣' } else { '┃' };
      if let Some(c) = buf.get_mut(x_off + (KEYBOARD_MARGIN_LEFT - 2) as u16, y_off + y as u16) {
        apply_style(c, sym, style);
      }
    }
  }

  fn draw_queue_right_to_buf(&self, buf: &mut ScreenBuffer, x_off: u16, y_off: u16) {
    if !self.show_keyboard || self.grid.height == 0 {
      return;
    }
    let total_height = self.grid.height;
    let half_height = total_height / 2;
    let style = CellStyle::fg_rgb(100, 100, 100);

    // Event queue (EVQ) - top half, bottom-aligned
    let event_queue = self.playhead_ui.queue_manager.event_queue.lock().unwrap();
    let evq_items: Vec<String> = event_queue
      .iter()
      .map(|op| op.get_event_name().to_string())
      .collect();
    drop(event_queue);

    let evq_start_y = if evq_items.len() >= half_height.saturating_sub(2) {
      0
    } else {
      half_height.saturating_sub(evq_items.len() + 2)
    };

    for y in 0..=(evq_start_y.min(half_height.saturating_sub(1))) {
      let c = if y % 2 == 0 { 50u8 } else { 70u8 };
      let ph_style = CellStyle::fg_rgb(c, c, c);
      for x in 1..QUEUE_MARGIN_RIGHT {
        if let Some(cell) = buf.get_mut(x_off + x as u16, y_off + y as u16) {
          apply_style(
            cell,
            consts::QUEUE_PLACEHOLDER_SYMBOL
              .chars()
              .next()
              .unwrap_or('·'),
            ph_style,
          );
        }
      }
    }
    for (idx, item) in evq_items.iter().enumerate() {
      if idx + 2 >= half_height {
        break;
      }
      let row = half_height - 2 - idx;
      for (i, ch) in item.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + 3 + i as u16, y_off + row as u16) {
          apply_style(c, ch, style);
        }
      }
    }
    for (i, ch) in "EVNTQ".chars().enumerate() {
      if let Some(c) = buf.get_mut(x_off + 3 + i as u16, y_off + (half_height - 1) as u16) {
        apply_style(c, ch, style);
      }
    }

    // Separator between EVQ and OPQ
    let center_x = (QUEUE_MARGIN_RIGHT / 2) + 1;
    for x in 0..QUEUE_MARGIN_RIGHT {
      let ch = if x == center_x { 'v' } else { ' ' };
      if let Some(c) = buf.get_mut(x_off + x as u16, y_off + half_height as u16) {
        apply_style(c, ch, style);
      }
    }

    // Operator queue (OPQ) - bottom half, bottom-aligned
    let operator_queue = self
      .playhead_ui
      .queue_manager
      .operator_queue
      .lock()
      .unwrap();
    let opq_items: Vec<String> = operator_queue
      .iter()
      .map(|item| format!("{}", item))
      .collect();
    drop(operator_queue);

    let opq_start_y = half_height;
    let opq_display_start_y = if opq_items.len() >= total_height.saturating_sub(opq_start_y + 2) {
      opq_start_y + 1
    } else {
      total_height.saturating_sub(opq_items.len() + 1)
    };

    for y in (opq_start_y + 1)..opq_display_start_y {
      let c = if y % 2 == 0 { 70u8 } else { 50u8 };
      let ph_style = CellStyle::fg_rgb(c, c, c);
      for x in 1..QUEUE_MARGIN_RIGHT {
        if let Some(cell) = buf.get_mut(x_off + x as u16, y_off + y as u16) {
          apply_style(
            cell,
            consts::QUEUE_PLACEHOLDER_SYMBOL
              .chars()
              .next()
              .unwrap_or('·'),
            ph_style,
          );
        }
      }
    }
    for (idx, item) in opq_items.iter().enumerate() {
      if idx + 2 >= total_height {
        break;
      }
      let row = total_height - 2 - idx;
      if row < opq_start_y + 1 {
        break;
      }
      for (i, ch) in item.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + 3 + i as u16, y_off + row as u16) {
          apply_style(c, ch, style);
        }
      }
    }
    for (i, ch) in "OPRTQ".chars().enumerate() {
      if let Some(c) = buf.get_mut(x_off + 3 + i as u16, y_off + (total_height - 1) as u16) {
        apply_style(c, ch, style);
      }
    }

    // Vertical separator
    for y in 0..total_height {
      let c = if y % 2 == 0 { 50u8 } else { 70u8 };
      let vs_style = CellStyle::fg_rgb(c, c, c);
      for (i, ch) in " ┃ ".chars().enumerate() {
        if let Some(cell) = buf.get_mut(x_off + i as u16, y_off + y as u16) {
          apply_style(cell, ch, vs_style);
        }
      }
    }

    // Pending jump indicator
    let pending = self
      .playhead_ui
      .queue_manager
      .pending_jump_position
      .lock()
      .unwrap();
    let (pending_str, pr, pg, pb) = match *pending {
      PendingJumpPosition::Empty => (None, 60u8, 60u8, 60u8),
      PendingJumpPosition::Waiting(x, y) => (Some(format!("{},{}", x, y)), 100, 100, 100),
      PendingJumpPosition::Armed(x, y) => (Some(format!("{},{}", x, y)), 255, 255, 255),
    };
    drop(pending);
    if let Some(s) = pending_str {
      let ps = CellStyle::fg_rgb(pr, pg, pb);
      for (i, ch) in s.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + 3 + i as u16, y_off + total_height as u16) {
          apply_style(c, ch, ps);
        }
      }
    }
  }

  fn draw_queue_operators_bottom_to_buf(&self, buf: &mut ScreenBuffer, x_off: u16, y_off: u16) {
    if !self.show_keyboard || self.grid.width == 0 {
      return;
    }
    let style = CellStyle::fg_rgb(100, 100, 100);

    // Separator line
    for x in 0..self.grid.width {
      if let Some(c) = buf.get_mut(x_off + x as u16, y_off) {
        apply_style(c, '─', style);
      }
    }

    let abs_active_x = self.playhead_ui.playhead_pos.x + self.playhead_ui.actived_pos.x;
    if abs_active_x < self.grid.width {
      let arrow_style = CellStyle::fg_rgb(255, 255, 255);
      if let Some(c) = buf.get_mut(x_off + abs_active_x as u16, y_off) {
        apply_style(c, 'v', arrow_style);
      }
    }

    let regex_indexes = self.playhead_ui.regex_indexes.lock().unwrap();
    let is_regex_match_x = regex_indexes
      .iter()
      .any(|&idx| idx % self.grid.width == abs_active_x);
    drop(regex_indexes);

    // Queue operators row
    let mut x = 0;
    let mut queue_index = 0;
    while x < self.grid.width {
      let is_event_position = x % consts::EVENT_OP_SPACING == 0;
      let display_char = if is_event_position {
        '-'
      } else if !self.playhead_ui.accumulation_mode {
        ' '
      } else {
        QUEUE_OPERATORS[queue_index % QUEUE_OPERATORS.len()]
          .to_string()
          .chars()
          .next()
          .unwrap_or(' ')
      };
      let is_active = x == abs_active_x;
      let op_style = if is_active && is_regex_match_x {
        CellStyle::fg_bg_rgb(0, 0, 0, 255, 255, 255)
      } else if is_active {
        CellStyle::fg_rgb(255, 255, 255)
      } else {
        CellStyle::fg_rgb(100, 100, 100)
      };
      if let Some(c) = buf.get_mut(x_off + x as u16, y_off + 1) {
        apply_style(c, display_char, op_style);
      }
      x += consts::QUEUE_OP_SPACING;
      queue_index += 1;
    }

    // Event operators overlay
    if self.playhead_ui.event_operator_mode {
      let mut x = 0;
      let mut event_index = 0;
      while x < self.grid.width {
        let op_ch = EVENT_OPERATORS[event_index % EVENT_OPERATORS.len()]
          .to_string()
          .chars()
          .next()
          .unwrap_or(' ');
        let is_active = x == abs_active_x;
        let ev_style = if is_active && is_regex_match_x {
          CellStyle::fg_bg_rgb(0, 0, 0, 255, 255, 255)
        } else if is_active {
          CellStyle::fg_rgb(255, 255, 255)
        } else {
          CellStyle::fg_rgb(150, 150, 150)
        };
        if let Some(c) = buf.get_mut(x_off + x as u16, y_off + 1) {
          apply_style(c, op_ch, ev_style);
        }
        x += consts::EVENT_OP_SPACING;
        event_index += 1;
      }
    }
  }

  /// Draw the entire grid panel into `buf` at offset `(x_off, y_off)`.
  pub fn draw_to_buf(&self, buf: &mut ScreenBuffer, x_off: u16, y_off: u16) {
    if self.show_keyboard {
      self.draw_keyboard_top_to_buf(buf, x_off + KEYBOARD_MARGIN_LEFT as u16, y_off);

      // Scale mode/root labels
      let pui = &self.playhead_ui;
      let root_note_top = pui.scale_root_top;
      let scale_mode_top = pui.scale_mode_top;
      let scale_mode_left = pui.scale_mode_left;
      let scale_root_left = pui.scale_root_left;

      let active_col = if !self.is_canvas_focused {
        Color::Rgb(100, 100, 100)
      } else {
        Color::Rgb(255, 255, 255)
      };
      let inactive_col = Color::Rgb(100, 100, 100);
      let (top_color, left_color) = if !self.is_canvas_focused {
        (inactive_col, inactive_col)
      } else if pui.keyboard_top_active {
        (active_col, inactive_col)
      } else {
        (inactive_col, active_col)
      };

      let top_label = format!("{} {}", scale_mode_top.short_name(), root_note_top.name());
      for (i, ch) in top_label.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + i as u16, y_off) {
          apply_style(
            c,
            ch,
            CellStyle {
              fg: top_color,
              bg: Color::Reset,
              reverse: false,
            },
          );
        }
      }
      let left_label = format!(
        "{} {}",
        scale_mode_left.short_name(),
        scale_root_left.name()
      );
      for (i, ch) in left_label.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + i as u16, y_off + 2) {
          apply_style(
            c,
            ch,
            CellStyle {
              fg: left_color,
              bg: Color::Reset,
              reverse: false,
            },
          );
        }
      }

      // Keyboard focus indicator
      let keyboard_label = if pui.keyboard_top_active {
        "[  \u{2227}  ]"
      } else {
        "[  \u{2228}  ]"
      };
      let label_col = if self.is_canvas_focused {
        Color::Rgb(200, 200, 200)
      } else {
        Color::Rgb(100, 100, 100)
      };
      for (i, ch) in keyboard_label.chars().enumerate() {
        if let Some(c) = buf.get_mut(x_off + i as u16, y_off + 1) {
          apply_style(
            c,
            ch,
            CellStyle {
              fg: label_col,
              bg: Color::Reset,
              reverse: false,
            },
          );
        }
      }

      // Left keyboard
      self.draw_keyboard_left_to_buf(buf, x_off, y_off + KEYBOARD_MARGIN_TOP as u16);

      // Bottom operators
      let bottom_y = y_off + KEYBOARD_MARGIN_TOP as u16 + self.grid.height as u16;
      self.draw_queue_operators_bottom_to_buf(buf, x_off + KEYBOARD_MARGIN_LEFT as u16, bottom_y);

      // Bottom corners
      let corner_style = CellStyle::fg_rgb(100, 100, 100);
      if let Some(c) = buf.get_mut(x_off + (KEYBOARD_MARGIN_LEFT - 2) as u16, bottom_y) {
        apply_style(c, '┗', corner_style);
      }
      if let Some(c) = buf.get_mut(
        x_off + (KEYBOARD_MARGIN_LEFT + self.grid.width + 1) as u16,
        bottom_y,
      ) {
        apply_style(c, '┛', corner_style);
      }

      // Right queue display
      let right_x = x_off + (KEYBOARD_MARGIN_LEFT + self.grid.width) as u16;
      self.draw_queue_right_to_buf(buf, right_x, y_off + KEYBOARD_MARGIN_TOP as u16);
    }

    let gx = x_off
      + if self.show_keyboard {
        KEYBOARD_MARGIN_LEFT as u16
      } else {
        0
      };
    let gy = y_off
      + if self.show_keyboard {
        KEYBOARD_MARGIN_TOP as u16
      } else {
        0
      };

    // Main grid cells
    self.grid.print(buf, gx, gy, &self.playhead_ui);

    // Channel separators
    let pui = &self.playhead_ui;
    let col_w = if pui.grid_v_splits >= 2 {
      self.grid.width / pui.grid_v_splits
    } else {
      0
    };
    let row_h = if pui.grid_h_splits >= 2 {
      self.grid.height / pui.grid_h_splits
    } else {
      0
    };
    let sep_xs: Vec<usize> = if pui.grid_v_splits >= 2 {
      (1..pui.grid_v_splits)
        .map(|i| col_w * i)
        .filter(|&x| x < self.grid.width)
        .collect()
    } else {
      vec![]
    };
    let sep_ys: Vec<usize> = if pui.grid_h_splits >= 2 {
      (1..pui.grid_h_splits)
        .map(|i| row_h * i)
        .filter(|&y| y < self.grid.height)
        .collect()
    } else {
      vec![]
    };
    let playhead_area = pui.playhead_area;
    let crosshair_abs_x = if pui.sweep_movement.is_some() {
      pui.sweep_x
    } else {
      pui.playhead_pos.x + pui.actived_pos.x
    };
    let is_special = |x: usize, y: usize| -> bool {
      let is_crosshair =
        pui.sweep_mode && pui.sweep_row_mode.is_row_active(y) && x == crosshair_abs_x;
      let is_drone = pui.drone_mode && x == playhead_area.left() + pui.drone_x;
      let is_match = pui
        .text_matcher
        .as_ref()
        .is_some_and(|m| m.contains_key(&(y * self.grid.width + x)));
      is_crosshair || is_drone || is_match
    };
    let sep_style = CellStyle::fg_rgb(100, 100, 100);
    for &sep_x in &sep_xs {
      for y in 0..self.grid.height {
        if playhead_area.contains((sep_x, y).into()) || is_special(sep_x, y) {
          continue;
        }
        let ch = if sep_ys.contains(&y) { '┼' } else { '│' };
        if let Some(c) = buf.get_mut(gx + sep_x as u16, gy + y as u16) {
          apply_style(c, ch, sep_style);
        }
      }
    }
    for &sep_y in &sep_ys {
      for x in 0..self.grid.width {
        if playhead_area.contains((x, sep_y).into()) || is_special(x, sep_y) {
          continue;
        }
        let ch = if sep_xs.contains(&x) { '┼' } else { '─' };
        if let Some(c) = buf.get_mut(gx + x as u16, gy + sep_y as u16) {
          apply_style(c, ch, sep_style);
        }
      }
    }
  }
}

/// Dispatch a crossterm key event to the appropriate grid action.
/// Returns true if the event was consumed.
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_key_event(canvas: &mut GridEditor, key: crossterm::event::KeyEvent) -> bool {
  use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

  if key.kind == KeyEventKind::Release {
    return false;
  }

  let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
  let alt = key.modifiers.contains(KeyModifiers::ALT);

  if ctrl {
    return match key.code {
      KeyCode::Char('h') => {
        canvas.start_aim_if_needed();
        canvas.update_aim(Direction::Left, 1)
      }
      KeyCode::Char('j') => {
        canvas.start_aim_if_needed();
        canvas.update_aim(Direction::Down, 1)
      }
      KeyCode::Char('k') => {
        canvas.start_aim_if_needed();
        canvas.update_aim(Direction::Up, 1)
      }
      KeyCode::Char('l') => {
        canvas.start_aim_if_needed();
        canvas.update_aim(Direction::Right, 1)
      }
      _ => false,
    };
  }

  if alt {
    return match key.code {
      KeyCode::Char('h') => canvas.alt_action(Direction::Left, consts::MOVE_X_STEP_SIZE),
      KeyCode::Char('j') => canvas.alt_action(Direction::Down, consts::MOVE_Y_STEP_SIZE),
      KeyCode::Char('k') => canvas.alt_action(Direction::Up, consts::MOVE_Y_STEP_SIZE),
      KeyCode::Char('l') => canvas.alt_action(Direction::Right, consts::MOVE_X_STEP_SIZE),
      _ => false,
    };
  }

  match key.code {
    KeyCode::Char('h') => canvas.commit_or_move(Direction::Left),
    KeyCode::Char('j') => canvas.commit_or_move(Direction::Down),
    KeyCode::Char('k') => canvas.commit_or_move(Direction::Up),
    KeyCode::Char('l') => canvas.commit_or_move(Direction::Right),
    KeyCode::Char('H') => canvas.scale_action((-1, 0)),
    KeyCode::Char('J') => canvas.scale_action((0, -1)),
    KeyCode::Char('K') => canvas.scale_action((0, 1)),
    KeyCode::Char('L') => canvas.scale_action((1, 0)),
    // macOS Option+h/j/k/l compose to these Unicode chars without an explicit Alt modifier
    #[cfg(target_os = "macos")]
    KeyCode::Char('˙') => canvas.alt_action(Direction::Left, consts::MOVE_X_STEP_SIZE),
    #[cfg(target_os = "macos")]
    KeyCode::Char('∆') => canvas.alt_action(Direction::Down, consts::MOVE_Y_STEP_SIZE),
    #[cfg(target_os = "macos")]
    KeyCode::Char('˚') => canvas.alt_action(Direction::Up, consts::MOVE_Y_STEP_SIZE),
    #[cfg(target_os = "macos")]
    KeyCode::Char('¬') => canvas.alt_action(Direction::Right, consts::MOVE_X_STEP_SIZE),
    // macOS Option+Shift+h/j/k/l
    #[cfg(target_os = "macos")]
    KeyCode::Char('Ó') => canvas.scale_action((-(consts::SCALE_X_STEP_SIZE as i32), 0)),
    #[cfg(target_os = "macos")]
    KeyCode::Char('Ô') => canvas.scale_action((0, -(consts::SCALE_Y_STEP_SIZE as i32))),
    #[cfg(target_os = "macos")]
    KeyCode::Char('\u{f8ff}') => canvas.scale_action((0, consts::SCALE_Y_STEP_SIZE as i32)),
    #[cfg(target_os = "macos")]
    KeyCode::Char('Ò') => canvas.scale_action((consts::SCALE_X_STEP_SIZE as i32, 0)),
    KeyCode::Char('[') => {
      canvas.sgr_leak_state = 1;
      false
    }
    KeyCode::Char('<') if canvas.sgr_leak_state == 1 => {
      canvas.sgr_leak_state = 2;
      true
    }
    KeyCode::Char(c) if canvas.sgr_leak_state == 2 && (c.is_ascii_digit() || c == ';') => true,
    KeyCode::Char('M') | KeyCode::Char('m') if canvas.sgr_leak_state == 2 => {
      canvas.sgr_leak_state = 0;
      true
    }
    KeyCode::Char(c) if ('1'..='7').contains(&c) => {
      canvas.sgr_leak_state = 0;
      canvas.handle_split_char(c)
    }
    _ => {
      canvas.sgr_leak_state = 0;
      false
    }
  }
}

/// Handle a crossterm mouse event. Returns true if consumed.
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_mouse_event(canvas: &mut GridEditor, event: crossterm::event::MouseEvent) -> bool {
  use crossterm::event::{MouseButton, MouseEventKind};

  let position = Vec2::new(event.column as usize, event.row as usize);
  let offset = Vec2::zero();

  match event.kind {
    MouseEventKind::Down(MouseButton::Left)
    | MouseEventKind::Down(MouseButton::Right)
    | MouseEventKind::Down(MouseButton::Middle) => canvas.handle_mouse_press(offset, position),
    MouseEventKind::Drag(MouseButton::Left) => canvas.handle_mouse_hold(offset, position),
    _ => false,
  }
}
