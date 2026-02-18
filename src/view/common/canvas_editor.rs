use std::{
  collections::{BTreeSet, HashMap},
  sync::{mpsc::Sender, Arc, Mutex},
};

use cursive::{
  event::{Event, EventResult, Key, MouseButton, MouseEvent},
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView},
  Printer, Vec2,
};

use crate::core::{consts, rect::Rect, regex::Match, traits::Matrix};

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
    // Normalize newlines to \n only
    let contents = contents.replace("\r\n", "\n").replace("\r", "\n");
    self.text_contents = Some(contents);
  }

  pub fn update_grid_src(&mut self) {
    if self.text_contents.as_ref().is_none() {
      return;
    };

    let cols: usize = self.grid.width;
    let rows: usize = self.grid.height;

    // Clear the grid first
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

  // pub fn update_grid_src(&mut self) {
  //   if self.text_contents.as_ref().is_none() {
  //     return;
  //   };

  //   let rows: usize = self.grid.width;
  //   let cols: usize = self.grid.height;

  //   let mut newline_idx_offset = 0;
  //   let mut prev_char_idx = 0;
  //   let mut mod_idx_offset = 0;
  //   let mut mod_idx_counter = 0;

  //   // TODO: clean up the mess. mostly, handling newline char logic.
  //   for row in 0..rows {
  //     for col in 0..cols {
  //       let char_idx = col + (row * cols);
  //       if let Some(char) = self.text_contents.as_ref().unwrap().chars().nth(char_idx) {
  //         if char == '\n' || char == '\r' {
  //           let line_pos = (char_idx - prev_char_idx) % rows;
  //           let placeholder_chars = rows - line_pos;
  //           for c in 0..placeholder_chars {
  //             self.grid.set(
  //               col + c + newline_idx_offset - prev_char_idx - mod_idx_offset,
  //               row,
  //               '\0'.display_char((char_idx + c + mod_idx_counter % rows, mod_idx_counter).into()),
  //             );
  //           }

  //           newline_idx_offset += line_pos + placeholder_chars;
  //           prev_char_idx = (char_idx + 1) % rows;
  //           mod_idx_counter += 1;
  //           if char_idx / rows > 0 {
  //             mod_idx_offset += rows;
  //           } else {
  //             mod_idx_offset = 0;
  //           }
  //         } else {
  //           self.grid.set(
  //             col + newline_idx_offset - prev_char_idx - mod_idx_offset,
  //             row,
  //             char,
  //           );
  //         }
  //       }
  //     }
  //   }
  // }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Matrix::new(size.x, size.y, '\0');
    self.size = size;
    // Update grid width for precise timing calculations
    let _ = self.marker_tx.send(Message::SetGridSize(size.x));
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
  canvas.grid.print(printer, &canvas.marker_ui);
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
      canvas
        .marker_tx
        .send(Message::SetCurrentPos(position, offset))
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

      let pos_x = position.x.abs_diff(1);
      let pos_y = position.y.abs_diff(offset.y);

      canvas
        .marker_tx
        .send(Message::SetGridArea((pos_x, pos_y).into()))
        .unwrap();

      EventResult::Ignored
    }
    _ => EventResult::Ignored,
  }
}
