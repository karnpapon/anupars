use cursive::theme::ColorStyle;
use cursive::theme::ColorType;
use cursive::theme::Style;
use cursive::utils::span::SpannedString;
use cursive::Printer;
use cursive::Vec2;

use crate::view::common::playhead::PlayheadUI;

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

  /// Render the active playhead position
  fn render_active_playhead(
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

  /// Render a cell inside the playhead area
  fn render_playhead_area_cell(
    &self,
    printer: &Printer,
    x: usize,
    y: usize,
    cell_index: usize,
    playhead_ui: &PlayheadUI,
  ) {
    let display_char = self.get_display_char(x, y);
    printer.print_styled(
      (x, y),
      &SpannedString::styled(display_char, Style::highlight()),
    );

    if let Some(matcher) = &playhead_ui.text_matcher {
      if matcher.contains_key(&cell_index) {
        let mut regex_indexes = playhead_ui.regex_indexes.lock().unwrap();
        regex_indexes.insert(cell_index);

        // Retain only indexes within playhead bounds
        let playhead_pos = playhead_ui.playhead_pos;
        let playhead_end = playhead_pos + playhead_ui.playhead_area.size();
        regex_indexes.retain(|&index| {
          let index_pos = self.index_to_xy(&index);
          index_pos.fits(playhead_pos) && index_pos.fits_in(playhead_end)
        });

        printer.print_styled((x, y), &SpannedString::styled('*', Style::highlight()));
      }
    }
  }

  /// Print the matrix to the given printer with playhead UI highlighting
  pub fn print(&self, printer: &Printer, playhead_ui: &PlayheadUI) {
    let PlayheadUI {
      text_matcher,
      playhead_pos,
      playhead_area,
      actived_pos,
      sweep_mode,
      ..
    } = playhead_ui;

    // Calculate absolute active position for crosshair
    let active_absolute_pos = playhead_pos.saturating_add(actived_pos);

    // Standard row-major order: iterate rows (y) then columns (x)
    for y in 0..self.height {
      for x in 0..self.width {
        let cell_index = x + y * self.width;
        let pos = (x, y);
        let is_in_playhead_area = playhead_area.contains(pos.into());
        let is_active_pos = active_absolute_pos.eq(&pos);
        let is_on_crosshair_vertical = x == active_absolute_pos.x && !is_active_pos;
        // let is_on_crosshair_horizontal = y == active_absolute_pos.y && !is_active_pos;

        // Render default cell with style
        let style = self.calculate_cell_style(cell_index, text_matcher);
        let display_char = self.get_display_char(x, y);
        printer.print_styled(pos, &SpannedString::styled(display_char, style));

        // Render crosshair lines at active position
        if is_on_crosshair_vertical && *sweep_mode {
          // Check if this position matches regex and is outside playhead area
          let is_regex_match = text_matcher
            .as_ref()
            .map(|m| m.contains_key(&cell_index))
            .unwrap_or(false);

          let crosshair_char = if is_regex_match && !is_in_playhead_area {
            ("@", Style::highlight())
          } else {
            (
              "|",
              Style::from(ColorStyle::front(ColorType::rgb(80, 80, 80))),
            )
          };

          printer.print_styled(
            pos,
            &SpannedString::styled(crosshair_char.0, crosshair_char.1),
          );
          // } else if is_on_crosshair_horizontal {
          //   let crosshair_style = Style::from(ColorStyle::front(ColorType::rgb(80, 80, 80)));
          //   printer.print_styled(pos, &SpannedString::styled("-", crosshair_style));
        }

        // Render playhead-specific overlays
        if is_in_playhead_area {
          if is_active_pos {
            self.render_active_playhead(printer, pos, cell_index, text_matcher);
          } else {
            self.render_playhead_area_cell(printer, x, y, cell_index, playhead_ui);
          }
        }
      }
    }
  }
}
