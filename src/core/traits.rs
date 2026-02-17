use cursive::{
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  Printer, Vec2,
};

use crate::view::common::canvas_editor::MarkerUI;

use super::{consts, regex::Match};
use std::collections::HashMap;

#[derive(Clone, Default, Debug)]
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

  #[allow(dead_code)]
  pub fn set_rect(&mut self, width: usize, height: usize, item: T) {
    for y in 0..height {
      for x in 0..width {
        self.set(x, y, item);
      }
    }
  }

  pub fn index_to_xy(&self, index: &usize) -> Vec2 {
    let x = index % self.width;
    let y = index / self.width;
    (x, y).into()
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
    pos.x.is_multiple_of(consts::GRID_ROW_SPACING) && pos.y.is_multiple_of(consts::GRID_COL_SPACING)
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
  /// Calculate the style for a cell based on text matching
  fn calculate_cell_style(
    &self,
    cell_index: usize,
    text_matcher: &Option<HashMap<usize, Match>>,
  ) -> Style {
    if let Some(matcher) = text_matcher {
      if matcher.contains_key(&cell_index) {
        return Style::highlight();
      }
    }
    Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100)))
  }

  /// Get the display character for a cell
  fn get_display_char(&self, x: usize, y: usize) -> String {
    self
      .get(x, y)
      .unwrap()
      .display_char((x, y).into())
      .to_string()
  }

  /// Render the active marker position
  fn render_active_marker(
    &self,
    printer: &Printer,
    pos: (usize, usize),
    cell_index: usize,
    text_matcher: &Option<HashMap<usize, Match>>,
  ) {
    printer.print_styled(pos, &SpannedString::styled('>', Style::none()));

    if let Some(matcher) = text_matcher {
      if matcher.contains_key(&cell_index) {
        printer.print_styled(pos, &SpannedString::styled('@', Style::none()));
      }
    }
  }

  /// Render a cell inside the marker area
  fn render_marker_area_cell(
    &self,
    printer: &Printer,
    x: usize,
    y: usize,
    cell_index: usize,
    marker_ui: &MarkerUI,
  ) {
    let display_char = self.get_display_char(x, y);
    printer.print_styled(
      (x, y),
      &SpannedString::styled(display_char, Style::highlight()),
    );

    if let Some(matcher) = &marker_ui.text_matcher {
      if matcher.contains_key(&cell_index) {
        let mut regex_indexes = marker_ui.regex_indexes.lock().unwrap();
        regex_indexes.insert(cell_index);

        // Retain only indexes within marker bounds
        let marker_pos = marker_ui.marker_pos;
        let marker_end = marker_pos + marker_ui.marker_area.size();
        regex_indexes.retain(|&index| {
          let index_pos = self.index_to_xy(&index);
          index_pos.fits(marker_pos) && index_pos.fits_in(marker_end)
        });

        printer.print_styled((x, y), &SpannedString::styled('*', Style::highlight()));
      }
    }
  }

  /// Print the matrix to the given printer with marker UI highlighting
  pub fn print(&self, printer: &Printer, marker_ui: &MarkerUI) {
    let MarkerUI {
      text_matcher,
      marker_pos,
      marker_area,
      actived_pos,
      ..
    } = marker_ui;

    // Standard row-major order: iterate rows (y) then columns (x)
    for y in 0..self.height {
      for x in 0..self.width {
        let cell_index = x + y * self.width;
        let pos = (x, y);
        let is_in_marker_area = marker_area.contains(pos.into());
        let is_active_pos = marker_pos.saturating_add(actived_pos).eq(&pos);

        // Render default cell with style
        let style = self.calculate_cell_style(cell_index, text_matcher);
        let display_char = self.get_display_char(x, y);
        printer.print_styled(pos, &SpannedString::styled(display_char, style));

        // Render marker-specific overlays
        if is_in_marker_area {
          if is_active_pos {
            self.render_active_marker(printer, pos, cell_index, text_matcher);
          } else {
            self.render_marker_area_cell(printer, x, y, cell_index, marker_ui);
          }
        }
      }
    }
  }
}
