use cursive::{
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  Printer,
};

use super::config;

#[derive(Clone, Default)]
pub struct Matrix<T> {
  pub data: Vec<T>,
  pub width: usize,
  pub height: usize,
}

impl<T: Copy> Matrix<T> {
  pub fn new(width: usize, height: usize, default: T) -> Matrix<T> {
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

  pub fn get(&self, x: usize, y: usize) -> Option<&T> {
    self.data.get(x + y * self.width)
  }

  pub fn set(&mut self, x: usize, y: usize, item: T) {
    self.data[x + y * self.width] = item;
  }

  pub fn set_rect(&mut self, width: usize, height: usize, item: T) {
    for i in 0..height {
      for j in 0..width {
        self.set(j, i, item);
      }
    }
  }
}

pub trait Printable {
  fn display_char(&self, pos: cursive::XY<usize>) -> char;
  fn should_rest(&self, pos: cursive::XY<usize>) -> bool {
    false
  }
}

impl Printable for char {
  fn should_rest(&self, pos: cursive::XY<usize>) -> bool {
    pos.x % config::GRID_ROW_SPACING == 0 && pos.y % config::GRID_COL_SPACING == 0
  }

  fn display_char(&self, pos: cursive::XY<usize>) -> char {
    match *self {
      '\0' => match self.should_rest(pos) {
        true => '+',
        false => '.',
      },
      _ => *self,
    }
  }
}

impl<T: Printable + Copy> Matrix<T> {
  pub fn print(&self, printer: &Printer) {
    for y in 0..self.width {
      for x in 0..self.height {
        printer.print_styled(
          (y, x),
          &SpannedString::styled(
            self
              .get(y, x)
              .unwrap()
              .display_char((x, y).into())
              .to_string(),
            Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
          ),
        );
      }
    }
  }
}
