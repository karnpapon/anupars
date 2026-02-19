use std::{
  collections::{BTreeSet, HashMap},
  sync::{mpsc::Sender, Arc, Mutex},
};

use cursive::{
  event::{Event, EventResult, Key, MouseButton, MouseEvent},
  theme::{ColorStyle, ColorType, Style},
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView},
  Printer, Vec2,
};

use crate::core::{consts, rect::Rect, regex::Match, traits::Matrix};

use consts::{BASE_OCTAVE, KEYBOARD_MARGIN_LEFT, KEYBOARD_MARGIN_TOP, NOTE_NAMES};

use super::marker::{self, Direction, Message};

pub struct MarkerUI {
  pub marker_area: Rect,
  pub marker_pos: Vec2,
  pub actived_pos: Vec2,
  pub text_matcher: Option<HashMap<usize, Match>>,
  pub regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
}

pub struct CanvasEditor {
  size: Vec2,
  pub marker_tx: Sender<Message>,
  pub grid: Matrix<char>,
  pub text_contents: Option<String>,
  pub marker_ui: MarkerUI,
  pub show_keyboard: bool,
  pub scale_mode_left: crate::core::scale::ScaleMode,
  pub scale_mode_top: crate::core::scale::ScaleMode,
}

impl MarkerUI {
  fn new() -> Self {
    MarkerUI {
      marker_area: Rect::from_point(Vec2::zero()),
      marker_pos: Vec2::zero(),
      actived_pos: Vec2::zero(),
      text_matcher: None,
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
    }
  }
}

impl CanvasEditor {
  pub fn new(marker_tx: Sender<marker::Message>) -> CanvasEditor {
    CanvasEditor {
      size: Vec2::zero(),
      marker_tx,
      grid: Matrix::new(0, 0, '\0'),
      text_contents: None,
      marker_ui: MarkerUI::new(),
      show_keyboard: true,
      scale_mode_left: crate::core::scale::ScaleMode::default(),
      scale_mode_top: crate::core::scale::ScaleMode::default(),
    }
  }

  /// Map Y position to MIDI note information (for left keyboard)
  /// Y increases downward, so higher Y = lower note (inverted keyboard)
  pub fn y_to_note_left(&self, y: usize) -> (u8, u8, &'static str) {
    let total_rows = self.grid.height;
    if total_rows == 0 {
      return (0, BASE_OCTAVE, "C");
    }

    let (note_index, octave) = self
      .scale_mode_left
      .y_to_scale_note(y, total_rows, BASE_OCTAVE);

    (note_index, octave, NOTE_NAMES[note_index as usize])
  }

  /// Map Y position to MIDI note information (for top keyboard)
  /// Y increases downward, so higher Y = lower note (inverted keyboard)
  pub fn y_to_note_top(&self, y: usize) -> (u8, u8, &'static str) {
    let total_rows = self.grid.height;
    if total_rows == 0 {
      return (0, BASE_OCTAVE, "C");
    }

    let (note_index, octave) = self
      .scale_mode_top
      .y_to_scale_note(y, total_rows, BASE_OCTAVE);

    (note_index, octave, NOTE_NAMES[note_index as usize])
  }

  /// Draw the keyboard visualization on the top margin
  fn draw_keyboard_top(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.height == 0 || self.grid.width == 0 {
      return;
    }

    for x in 0..self.grid.width {
      let y_pos = x % self.grid.height;
      let (note_index, octave, note_name) = self.y_to_note_top(y_pos);

      let is_black_key = matches!(note_index, 1 | 3 | 6 | 8 | 10); // C#, D#, F#, G#, A#

      let text_color = ColorType::rgb(100, 100, 100);

      let style = if note_name == "C" {
        Style::from(ColorStyle::new(
          ColorType::rgb(0, 0, 0),
          ColorType::rgb(100, 100, 100),
        ))
      } else {
        Style::from(ColorStyle::front(text_color))
      };

      printer.with_style(style, |printer| {
        if is_black_key {
          printer.print((x, 0), "#");
        } else if note_name == "C" {
          printer.print((x, 0), " ");
          printer.print((x, 1), &octave.to_string());
        } else {
          printer.print((x, 0), "━");
        }
      });
    }
  }

  /// Draw the keyboard visualization on the left margin
  fn draw_keyboard_left(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.height == 0 {
      return;
    }

    // Draw note names vertically
    for y in 0..self.grid.height {
      let (note_index, octave, note_name) = self.y_to_note_left(y);

      // Determine text color based on note (white/black keys)
      let is_black_key = matches!(note_index, 1 | 3 | 6 | 8 | 10); // C#, D#, F#, G#, A#

      let text_color = if is_black_key {
        ColorType::rgb(50, 50, 50)
      } else {
        ColorType::rgb(100, 100, 100)
      };

      let style = Style::from(ColorStyle::front(text_color));

      // Format note label (e.g., "C3", "D#4")
      let label = format!("{}{}", note_name, octave);

      // Use ┣ for C notes to mark octaves, otherwise use ┃
      let symbol = if note_name == "C" { "┣" } else { "┃" };

      printer.with_style(style, |printer| {
        printer.print((0, y), &label);
        printer.print((2, y), symbol);
      });
    }
  }

  pub fn build(
    marker_tx: Sender<marker::Message>,
  ) -> ResizedView<ResizedView<NamedView<Canvas<CanvasEditor>>>> {
    Canvas::new(CanvasEditor::new(marker_tx))
      .with_draw(draw)
      .with_layout(layout)
      .with_on_event(on_event)
      .with_take_focus(take_focus)
      .with_name(consts::canvas_editor_section_view)
      .full_height()
      .full_width()
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
        // Move to next row
        x = 0;
        y += 1;
      } else {
        // Place character at current position
        if x < cols {
          self.grid.set(x, y, ch);
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

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Matrix::new(size.x, size.y, '\0');
    self.size = size;
    // Update grid width and height for precise timing calculations and note mapping
    let _ = self.marker_tx.send(Message::SetGridSize(size.x, size.y));
    // Ensure marker stays within new bounds
    let _ = self.marker_tx.send(Message::Move(Direction::Idle, size));
  }

  pub fn clear_contents(&mut self) {
    self.grid = Matrix::new(self.size.x, self.size.y, '\0');
  }

  pub fn text_contents(&self) -> String {
    self
      .text_contents
      .as_ref()
      .unwrap_or(&"".to_string())
      .to_string()
  }
}

fn draw(canvas: &CanvasEditor, printer: &Printer) {
  if canvas.show_keyboard {
    // Draw top keyboard visualization
    let top_keyboard_printer = printer.offset((KEYBOARD_MARGIN_LEFT, 0));
    canvas.draw_keyboard_top(&top_keyboard_printer);

    // Draw left keyboard visualization
    let left_keyboard_printer = printer.offset((0, KEYBOARD_MARGIN_TOP));
    canvas.draw_keyboard_left(&left_keyboard_printer);

    // Draw corner symbol where keyboards meet
    let style = Style::from(ColorStyle::front(ColorType::rgb(200, 200, 200)));
    printer.with_style(style, |printer| {
      printer.print((0, 0), "");
    });
  }

  // Offset the grid to make room for both keyboards
  let x_offset = if canvas.show_keyboard {
    KEYBOARD_MARGIN_LEFT
  } else {
    0
  };
  let y_offset = if canvas.show_keyboard {
    KEYBOARD_MARGIN_TOP
  } else {
    0
  };
  let grid_printer = printer.offset((x_offset, y_offset));

  canvas.grid.print(&grid_printer, &canvas.marker_ui);
}

fn layout(canvas: &mut CanvasEditor, size: Vec2) {
  // Resize canvas when size changes (initialization or terminal resize)
  if canvas.size != size {
    canvas.resize(size);
    // Update grid content if text is already loaded
    if canvas.text_contents.is_some() {
      canvas.update_grid_src();
    }
  }
}

fn take_focus(
  _: &mut CanvasEditor,
  _: cursive::direction::Direction,
) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(canvas: &mut CanvasEditor, event: Event) -> EventResult {
  match event {
    Event::Refresh => EventResult::consumed(),
    Event::Key(Key::Left) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Left, canvas.size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Key(Key::Right) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Right, canvas.size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Key(Key::Up) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Up, canvas.size))
        .unwrap();
      EventResult::consumed()
    }
    Event::Key(Key::Down) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Down, canvas.size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Press(_btn),
    } => {
      // Adjust position to account for keyboard margins
      let x_offset = if canvas.show_keyboard {
        KEYBOARD_MARGIN_LEFT
      } else {
        0
      };
      let y_offset = if canvas.show_keyboard {
        KEYBOARD_MARGIN_TOP
      } else {
        0
      };

      let adjusted_position = Vec2::new(
        position.x.saturating_sub(x_offset),
        position.y.saturating_sub(y_offset),
      );

      canvas
        .marker_tx
        .send(Message::SetCurrentPos(adjusted_position, offset))
        .unwrap();
      canvas
        .marker_tx
        .send(Message::Move(Direction::Idle, canvas.size))
        .unwrap();
      canvas
        .marker_tx
        .send(Message::UpdateInfoStatusView())
        .unwrap();

      EventResult::consumed()
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Hold(MouseButton::Left),
    } => {
      // TODO: not sure why these (`MouseEvent::Hold`) sometimes being called twice (bug?) !?
      // need more investigate on this

      // Adjust position to account for keyboard margins
      let x_offset = if canvas.show_keyboard {
        KEYBOARD_MARGIN_LEFT
      } else {
        0
      };
      let y_offset = if canvas.show_keyboard {
        KEYBOARD_MARGIN_TOP
      } else {
        0
      };

      let pos_x = position.x.saturating_sub(x_offset + 1);
      let pos_y = position.y.saturating_sub(offset.y + y_offset);

      canvas
        .marker_tx
        .send(Message::SetGridArea((pos_x, pos_y).into()))
        .unwrap();

      EventResult::Ignored
    }
    _ => EventResult::Ignored,
  }
}
