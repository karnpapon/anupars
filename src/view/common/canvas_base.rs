use cursive::{
  direction::Direction,
  event::{Event, EventResult},
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView},
  Printer, Vec2,
};
use std::{collections::HashMap, usize};

use crate::core::{
  config,
  regex::Match,
  traits::{Matrix},
};

#[derive(Clone)]
pub struct CanvasBase {
  size: Vec2,
  grid: Matrix<char>,
  text_contents: Option<String>,
  text_matcher: Option<HashMap<usize, Match>>,
}

impl CanvasBase {
  pub fn new() -> CanvasBase {
    CanvasBase {
      size: Vec2::zero(),
      grid: Matrix::new(0, 0, '\0'),
      text_contents: None,
      text_matcher: None,
    }
  }

  pub fn build() -> ResizedView<ResizedView<NamedView<Canvas<CanvasBase>>>> {
    Canvas::new(CanvasBase::new())
      .with_draw(draw)
      .with_layout(layout)
      .with_on_event(on_event)
      .with_take_focus(take_focus)
      .with_name(config::canvas_base_section_view)
      .full_height()
      .full_width()
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Matrix::new(size.x, size.y, '\0');
    self.size = size;
  }

  pub fn set_text_matcher(&mut self, text_matcher: Option<HashMap<usize, Match>>) {
    self.text_matcher = text_matcher
  }

  pub fn text_contents(&self) -> String {
    self
      .text_contents
      .as_ref()
      .unwrap_or(&"".to_string())
      .to_string()
  }

  pub fn update_text_contents(&mut self, contents: &str) {
    self.text_contents = Some(String::from(contents));
  }

  pub fn clear(&mut self) {
    self.grid = Matrix::new(self.size.x, self.size.y, '\0');
  }

  pub fn update_grid_src(&mut self) {
    if self.text_contents.as_ref().is_none() {
      return;
    };

    let rows: usize = self.grid.width;
    let cols: usize = self.grid.height;

    for row in 0..rows {
      for col in 0..cols {
        if let Some(char) = self
          .text_contents
          .as_ref()
          .unwrap()
          .chars()
          .nth(col + (row * cols))
        {
          self.grid.set(col, row, char);
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
}

fn draw(canvas: &CanvasBase, printer: &Printer) {
  if canvas.size > Vec2::ZERO {
    canvas.grid.print(printer, &canvas.text_matcher);
  }
}

fn layout(canvas: &mut CanvasBase, size: Vec2) {
  if canvas.size == Vec2::ZERO {
    canvas.resize(size)
  }
}

fn take_focus(_: &mut CanvasBase, _: Direction) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(_: &mut CanvasBase, _: Event) -> EventResult {
  EventResult::Consumed(None)
}
