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

use consts::{
  BASE_OCTAVE, KEYBOARD_MARGIN_BOTTOM, KEYBOARD_MARGIN_LEFT, KEYBOARD_MARGIN_TOP, NOTE_NAMES,
};

use super::marker::{self, Direction, Message};

pub struct MarkerUI {
  pub marker_area: Rect,
  pub marker_pos: Vec2,
  pub actived_pos: Vec2,
  pub text_matcher: Option<HashMap<usize, Match>>,
  pub regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  pub reverse_mode: bool,
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
  pub reverse_mode: bool,
}

impl MarkerUI {
  fn new() -> Self {
    MarkerUI {
      marker_area: Rect::from_point(Vec2::zero()),
      marker_pos: Vec2::zero(),
      actived_pos: Vec2::zero(),
      text_matcher: None,
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      reverse_mode: false,
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
      reverse_mode: false,
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

    let abs_active_x = self.marker_ui.marker_pos.x + self.marker_ui.actived_pos.x;
    for x in 0..self.grid.width {
      let y_pos = x % self.grid.height;
      let (note_index, octave, note_name) = self.y_to_note_top(y_pos);

      let is_black_key = matches!(note_index, 1 | 3 | 6 | 8 | 10); // C#, D#, F#, G#, A#

      let style = if x == abs_active_x {
        if note_name == "C" {
          Style::from(ColorStyle::new(
            ColorType::rgb(0, 0, 0),
            ColorType::rgb(255, 255, 255),
          ))
        } else {
          Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)))
        }
      } else if note_name == "C" {
        Style::from(ColorStyle::new(
          ColorType::rgb(0, 0, 0),
          ColorType::rgb(100, 100, 100),
        ))
      } else {
        Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)))
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

  fn draw_keyboard_left(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.height == 0 {
      return;
    }

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
      let symbol = if note_name == "C" { "┣" } else { "┃" };

      printer.with_style(style, |printer| {
        printer.print((0, y), &label);
        printer.print((2, y), symbol);
      });
    }
  }

  fn draw_stack_operators_bottom(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.width == 0 {
      return;
    }

    let operators = ['P', 'S', 'O']; // Push, Swap, pOp
    let spacing = 10;
    let pattern_width = spacing + 1;

    let style = Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)));

    printer.with_style(style, |printer| {
      for x in 0..self.grid.width {
        printer.print((x, 0), "─");
      }
    });

    let abs_active_x = self.marker_ui.marker_pos.x + self.marker_ui.actived_pos.x;
    if abs_active_x < self.grid.width {
      let arrow_style = Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)));
      printer.with_style(arrow_style, |printer| {
        printer.print((abs_active_x, 0), "v");
      });
    }

    let mut x = 0;
    let mut op_index = 0;

    let abs_active_x = self.marker_ui.marker_pos.x + self.marker_ui.actived_pos.x;
    let regex_indexes = self.marker_ui.regex_indexes.lock().unwrap();
    let is_regex_match_x = regex_indexes.iter().any(|&idx| {
      let x_pos = idx % self.grid.width;
      x_pos == abs_active_x
    });
    while x < self.grid.width {
      let op = operators[op_index % operators.len()];
      let is_active = x == abs_active_x;
      let style = if is_active && is_regex_match_x {
        Style::from(ColorStyle::new(
          ColorType::rgb(0, 0, 0),
          ColorType::rgb(255, 255, 255),
        ))
      } else if is_active {
        Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)))
      } else {
        Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)))
      };
      printer.with_style(style, |printer| {
        printer.print((x, 1), &op.to_string());
      });

      x += pattern_width;
      op_index += 1;
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
    let grid_width = if self.show_keyboard {
      size.x.saturating_sub(KEYBOARD_MARGIN_LEFT)
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
    // Update grid width and height for precise timing calculations and note mapping
    let _ = self
      .marker_tx
      .send(Message::SetGridSize(grid_width, grid_height));
    // Ensure marker stays within new bounds
    let _ = self.marker_tx.send(Message::Move(
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
}

fn draw(canvas: &CanvasEditor, printer: &Printer) {
  if canvas.show_keyboard {
    let top_keyboard_printer = printer.offset((KEYBOARD_MARGIN_LEFT, 0));
    canvas.draw_keyboard_top(&top_keyboard_printer);

    let left_keyboard_printer = printer.offset((0, KEYBOARD_MARGIN_TOP));
    canvas.draw_keyboard_left(&left_keyboard_printer);

    let bottom_y = KEYBOARD_MARGIN_TOP + canvas.grid.height;
    let bottom_operators_printer = printer.offset((KEYBOARD_MARGIN_LEFT, bottom_y));
    canvas.draw_stack_operators_bottom(&bottom_operators_printer);
  }

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
  if canvas.size != size {
    canvas.resize(size);
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
      let grid_size = (canvas.grid.width, canvas.grid.height).into();
      canvas
        .marker_tx
        .send(Message::Move(Direction::Left, grid_size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Key(Key::Right) => {
      let grid_size = (canvas.grid.width, canvas.grid.height).into();
      canvas
        .marker_tx
        .send(Message::Move(Direction::Right, grid_size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Key(Key::Up) => {
      let grid_size = (canvas.grid.width, canvas.grid.height).into();
      canvas
        .marker_tx
        .send(Message::Move(Direction::Up, grid_size))
        .unwrap();
      EventResult::consumed()
    }
    Event::Key(Key::Down) => {
      let grid_size = (canvas.grid.width, canvas.grid.height).into();
      canvas
        .marker_tx
        .send(Message::Move(Direction::Down, grid_size))
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
      let grid_size = (canvas.grid.width, canvas.grid.height).into();
      canvas
        .marker_tx
        .send(Message::Move(Direction::Idle, grid_size))
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
