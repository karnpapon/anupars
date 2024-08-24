use cursive::{
  direction::Direction,
  event::{Event, EventResult},
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView},
  Printer, Vec2,
};
use std::{
  mem,
  sync::{Arc, Mutex},
  usize,
};

use crate::core::config;

#[derive(Clone)]
pub struct CanvasBase {
  grid_row_spacing: usize,
  grid_col_spacing: usize,
  size: Vec2,
  grid: Arc<Mutex<Vec<Vec<char>>>>,
  text_contents: Option<String>,
}

impl CanvasBase {
  pub fn new() -> CanvasBase {
    CanvasBase {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      grid: Arc::new(Mutex::new(vec![])),
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

  pub fn set_char_at(&mut self, x: usize, y: usize, glyph: char) {
    let mut vec = self.grid.lock().unwrap();
    vec[x][y] = glyph;
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Arc::new(Mutex::new(vec![vec!['\0'; size.x]; size.y]));
    self.size = size;
    self.set_empty_char();
  }

  pub fn set_empty_char(&mut self) {
    let vec = self.grid();

    for (x, row) in vec.iter().enumerate() {
      for (y, &value) in row.iter().enumerate() {
        let display_value = match value {
          '\0' => {
            if x % self.grid_row_spacing == 0 && y % self.grid_col_spacing == 0 {
              '+'
            } else {
              '.'
            }
          }
          _ => value,
        };

        self.set_char_at(x, y, display_value);
      }
    }
  }

  pub fn update_text_contents(&mut self, contents: &str) {
    self.text_contents = Some(String::from(contents));
  }

  pub fn update_grid_src(&mut self) {
    let mut vec = self.grid();
    if let Some(tc) = self.text_contents.as_ref() {
      let rows: usize = vec.len();
      let cols: usize = vec[0].len();

      for row in 0..rows {
        for col in 0..cols {
          if let Some(char) = tc.chars().nth(col + (row * cols)) {
            _ = mem::replace(&mut vec[row][col], char);
            // self.set_char_at(row, col, char);
          }
        }
      }
    }
  }

  pub fn grid(&self) -> Vec<Vec<char>> {
    self.grid.lock().unwrap().clone()
  }
}

fn draw(canvas: &CanvasBase, printer: &Printer) {
  let grid = canvas.grid();

  if canvas.size > Vec2::new(0, 0) {
    for (x, row) in grid.iter().enumerate() {
      for (y, &value) in row.iter().enumerate() {
        printer.print_styled(
          (y, x),
          &SpannedString::styled(
            value.to_string(),
            Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
          ),
        );
      }
    }
  }
}

fn layout(canvas: &mut CanvasBase, size: Vec2) {
  canvas.resize(size)
}

fn take_focus(canvas: &mut CanvasBase, _: Direction) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(canvas: &mut CanvasBase, _: Event) -> EventResult {
  EventResult::Consumed(None)
}
