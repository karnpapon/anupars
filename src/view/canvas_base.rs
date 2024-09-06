use cursive::{
  direction::Direction,
  event::{Event, EventResult},
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView},
  Printer, Vec2,
};
use std::usize;

use crate::core::{config, traits::Matrix};

#[derive(Clone)]
pub struct CanvasBase {
  size: Vec2,
  grid: Matrix<char>,
  text_contents: Option<String>,
}

impl CanvasBase {
  pub fn new() -> CanvasBase {
    CanvasBase {
      size: Vec2::zero(),
      grid: Matrix::new(0, 0, '\0'),
      text_contents: None,
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

    let mut newline_pos_offset = 0;
    let mut prev_char_pos = 0;
    let splitted_text_contents = self
      .text_contents
      .as_ref()
      .unwrap()
      .split('\n')
      .collect::<Vec<&str>>();

    for row in 0..rows {
      for col in 0..cols {
        let char_pos = col + (row * cols);
        let line_pos = (char_pos - prev_char_pos) % rows;
        let placeholder_chars = rows - line_pos;
        if let Some(char) = self.text_contents.as_ref().unwrap().chars().nth(char_pos) {
          if char == '\n' || char == '\r' {
            for c in 0..placeholder_chars {
              self
                .grid
                .set(col + newline_pos_offset + c - prev_char_pos, row, '█');
            }

            newline_pos_offset += line_pos + placeholder_chars;
            prev_char_pos = char_pos + 1;
          } else {
            self
              .grid
              .set(col + newline_pos_offset - prev_char_pos, row, char);
          }
        }
      }
    }
  }
}

fn draw(canvas: &CanvasBase, printer: &Printer) {
  if canvas.size > Vec2::ZERO {
    canvas.grid.print(printer);
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
