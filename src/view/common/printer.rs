use cursive::theme::ColorStyle;
use cursive::theme::ColorType;
use cursive::theme::Style;
use cursive::utils::span::SpannedString;
use cursive::Printer;
use cursive::Vec2;

use crate::view::common::playhead_handler::PlayheadUI;

use crate::core::{consts, engine::regex::Match};
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
      grid_v_splits,
      grid_h_splits,
      ..
    } = playhead_ui;

    let active_absolute_pos = playhead_pos.saturating_add(actived_pos);

    // Compute the active channel region for dimming
    let v = (*grid_v_splits).max(1);
    let h = (*grid_h_splits).max(1);

    //            x0    x1
    //            ↓     ↓
    //  col:  0     1     2     3
    //      ┌─────┬─────┬─────┬─────┐
    //      │  1  │  2  │  3  │  4  │  row 0
    // y0 → ├─────╔═════╗─────┬─────┤
    //      │  5  ║  6  ║  7  │  8  │  row 1
    // y1 → └─────╚═════╝─────┴─────┘
    let channel_bounds: Option<(usize, usize, usize, usize)> = if v > 1 || h > 1 {
      let col_w = (self.width / v).max(1);
      let row_h = (self.height / h).max(1);
      let curr_col = (playhead_pos.x / col_w).min(v.saturating_sub(1));
      let curr_row = (playhead_pos.y / row_h).min(h.saturating_sub(1));
      let x1 = if curr_col + 1 >= v {
        self.width
      } else {
        (curr_col + 1) * col_w
      };
      let y1 = if curr_row + 1 >= h {
        self.height
      } else {
        (curr_row + 1) * row_h
      };
      Some((curr_col * col_w, x1, curr_row * row_h, y1))
    } else {
      None
    };

    // Standard row-major order: iterate rows (y) then columns (x)
    for y in 0..self.height {
      for x in 0..self.width {
        let cell_index = x + y * self.width;
        let pos = (x, y);
        let is_in_playhead_area = playhead_area.contains(pos.into());
        let is_active_pos = active_absolute_pos.eq(&pos);
        let is_on_crosshair_vertical = x == active_absolute_pos.x && !is_active_pos;

        let is_in_active_channel = match channel_bounds {
          Some((x0, x1, y0, y1)) => x >= x0 && x < x1 && y >= y0 && y < y1,
          None => true,
        };

        // Column-strip bounds (x only)
        // shares the same vertical channels (e.g. ch2 + ch6 are same x-strip)
        let is_in_active_col_strip = match channel_bounds {
          Some((x0, x1, _, _)) => x >= x0 && x < x1,
          None => true,
        };

        let is_regex_match = text_matcher
          .as_ref()
          .map(|m| m.contains_key(&cell_index))
          .unwrap_or(false);

        let style = if is_in_active_col_strip {
          if *sweep_mode && is_regex_match && !is_in_playhead_area {
            Style::highlight()
          } else if is_in_active_channel {
            self.calculate_cell_style(cell_index, text_matcher)
          } else {
            Style::from_color_style(ColorStyle::front(ColorType::rgb(50, 50, 50)))
          }
        } else {
          Style::from_color_style(ColorStyle::front(ColorType::rgb(50, 50, 50)))
        };
        let display_char = self.get_display_char(x, y);
        printer.print_styled(pos, &SpannedString::styled(display_char, style));

        if is_on_crosshair_vertical && *sweep_mode {
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
