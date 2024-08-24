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
  sync::{Arc, Mutex},
  usize,
};

use crate::core::config;

#[derive(Clone)]
struct Matrix<T> {
  data: Vec<T>,
  width: usize,
  height: usize,
}

impl<T: Copy> Matrix<T> {
  fn new(width: usize, height: usize, default: T) -> Matrix<T> {
    let mut data: Vec<T> = Vec::with_capacity(width * height);
    for _ in 0..width * height {
      data.push(default);
    }
    Matrix {
      data,
      width,
      height,
    }
  }

  fn get(&self, x: usize, y: usize) -> T {
    self.data[x + y * self.width]
  }

  fn set(&mut self, x: usize, y: usize, item: T) {
    self.data[x + y * self.width] = item;
  }

  fn set_rect(&mut self, width: usize, height: usize, item: T) {
    for i in 0..height {
      for j in 0..width {
        self.set(j, i, item);
      }
    }
  }
}

trait Printable {
  fn display_char(&self, pos: cursive::XY<usize>) -> char;
}

impl Printable for char {
  fn display_char(&self, pos: cursive::XY<usize>) -> char {
    match *self {
      '\0' => {
        if pos.x % config::GRID_ROW_SPACING == 0 && pos.y % config::GRID_COL_SPACING == 0 {
          '+'
        } else {
          '.'
        }
      }
      _ => *self,
    }
  }
}

impl<T: Printable + Copy> Matrix<T> {
  fn print(&self, printer: &Printer) {
    for y in 0..self.width {
      for x in 0..self.height {
        printer.print_styled(
          (y, x),
          &SpannedString::styled(
            self.get(y, x).display_char((x, y).into()).to_string(),
            Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
          ),
        );
      }
    }
  }
}

#[derive(Clone)]
pub struct CanvasBase {
  size: Vec2,
  grid: Arc<Mutex<Matrix<char>>>,
  text_contents: Option<String>,
}

impl CanvasBase {
  pub fn new() -> CanvasBase {
    CanvasBase {
      size: Vec2::new(0, 0),
      grid: Arc::new(Mutex::new(Matrix::new(0, 0, '\0'))),
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
    self.grid = Arc::new(Mutex::new(Matrix::new(size.x, size.y, '\0')));
    self.size = size;
    self.grid().set_rect(size.x, size.y, '\0');
  }

  pub fn update_text_contents(&mut self, contents: &str) {
    self.text_contents = Some(String::from(contents));
  }

  pub fn update_grid_src(&mut self) {
    if self.text_contents.as_ref().is_none() {
      return;
    };

    let mut vec = self.grid();
    let rows: usize = vec.width;
    let cols: usize = vec.height;

    for row in 0..rows {
      for col in 0..cols {
        if let Some(char) = self
          .text_contents
          .as_ref()
          .unwrap()
          .chars()
          .nth(col + (row * cols))
        {
          vec.set(row, col, char);
        }
      }
    }
  }

  fn grid(&self) -> Matrix<char> {
    self.grid.lock().unwrap().clone()
  }
}

fn draw(canvas: &CanvasBase, printer: &Printer) {
  if canvas.size > Vec2::new(0, 0) {
    canvas.grid().print(printer);
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
