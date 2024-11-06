use std::collections::HashMap;

use cursive::{
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  Printer,
};

use crate::view::common::canvas_editor::MarkerUI;

use super::{config, regex::Match};

#[derive(Clone, Default, Debug)]
pub struct Matrix<T> {
  pub data: Vec<T>,
  pub width: usize,
  pub height: usize,
  // pub
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
    self.data[x + y * self.height] = item;
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
  fn should_rest(&self, _pos: cursive::XY<usize>) -> bool {
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
        true => ':',
        false => '.',
      },
      _ => *self,
    }
  }
}

impl<T: Printable + Copy> Matrix<T> {
  pub fn print(
    &self,
    printer: &Printer,
    highlighter: &Option<HashMap<usize, Match>>,
    marker_ui: &MarkerUI,
  ) {
    for y in 0..self.width {
      for x in 0..self.height {
        let style = if highlighter.is_some() {
          let hl = highlighter.as_ref().unwrap();
          if hl.get(&(y + x * self.width)).is_some() {
            Style::highlight()
          } else {
            Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100)))
          }
        } else {
          Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100)))
        };

        printer.print_styled(
          (y, x),
          &SpannedString::styled(
            self
              .get(y, x)
              .unwrap()
              .display_char((x, y).into())
              .to_string(),
            style,
          ),
        );

        if (y, x) == (marker_ui.marker_pos.x, marker_ui.marker_pos.y) {
          printer.print_styled(
            marker_ui.marker_pos,
            &SpannedString::styled('>', Style::highlight()),
          );
        } else if marker_ui.marker_area.contains((y, x).into()) {
          printer.print_styled(
            (y, x),
            &SpannedString::styled(
              self
                .get(y, x)
                .unwrap()
                .display_char((x, y).into())
                .to_string(),
              Style::highlight(),
            ),
          );
        } else {
          //
        }
      }
    }
  }
}
