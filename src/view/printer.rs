use cursive::theme::ColorStyle;
use cursive::theme::ColorType;
use cursive::theme::Style;
use cursive::utils::span::SpannedString;
use cursive::Printer;
use cursive::Vec2;

use crate::core::consts;
use crate::core::engine::regex::Match;
use crate::core::playhead::tilt::TiltMode;
use crate::view::playhead_handler::PlayheadUI;
use std::collections::HashMap;

#[derive(Clone, Default, Debug)]
pub struct Matrix<T> {
  pub data: Vec<T>,
  pub width: usize,
  pub height: usize,
}

impl<T: Copy> Matrix<T> {
  pub fn new(width: usize, height: usize, default: T) -> Matrix<T> {
    Matrix {
      data: vec![default; width * height],
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
      '\0' if self.should_rest(pos) => ':',
      '\0' => '.',
      c => c,
    }
  }
}

/// Precomputed tilt/channel geometry for one `Matrix::print` call.
struct TiltMetrics {
  playhead_col: usize,
  playhead_row: usize,
  col_w: usize,
  row_h: usize,
}

/// Tilt-adjusted sweep band for a single row.
struct RowSweep {
  band_start: usize,
  band_end: usize,
  /// x-coordinate of the crosshair; `usize::MAX` when this row has no active band.
  crosshair_x: usize,
}

impl RowSweep {
  /// Sentinel for a row whose diagonal falls outside the grid.
  const INACTIVE: Self = Self {
    band_start: 0,
    band_end: 0,
    crosshair_x: usize::MAX,
  };

  fn in_band(&self, x: usize) -> bool {
    x >= self.band_start && x < self.band_end
  }

  fn is_crosshair(&self, x: usize) -> bool {
    x == self.crosshair_x
  }
}

struct CellStyleContext<'a> {
  x: usize,
  y: usize,
  is_regex_match: bool,
  sweep: &'a RowSweep,
  channel_bounds: Option<(usize, usize, usize, usize)>,
  is_in_playhead_area: bool,
  sweep_mode: bool,
  v: usize,
  h: usize,
}

fn dim_style() -> Style {
  Style::from_color_style(ColorStyle::front(ColorType::rgb(50, 50, 50)))
}

fn normal_style(is_match: bool) -> Style {
  if is_match {
    Style::highlight()
  } else {
    Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100)))
  }
}

/// Returns the `Match` whose span `[i, i+l)` covers `cell`, or `None`.
fn match_covering(matcher: &HashMap<usize, Match>, cell: usize) -> Option<&Match> {
  matcher
    .values()
    .find(|m| cell >= m.i && cell < m.i + m.l.max(1))
}

impl<T: Printable + Copy> Matrix<T> {
  fn get_display_char(&self, x: usize, y: usize) -> char {
    self.get(x, y).unwrap().display_char((x, y).into())
  }

  /// Render the active (cursor) playhead cell.
  fn render_active_playhead(
    &self,
    printer: &Printer,
    pos: (usize, usize),
    cell_index: usize,
    text_matcher: &Option<HashMap<usize, Match>>,
  ) {
    printer.print_styled(
      pos,
      &SpannedString::styled(consts::PLAYHEAD_CHAR, Style::none()),
    );
    if let Some(matcher) = text_matcher {
      if let Some(m) = match_covering(matcher, cell_index) {
        let symbol = if m.l <= 1 {
          consts::PLAYHEAD_MATCH_CHAR
        } else {
          consts::MATCH_GROUP_CHAR
        };
        printer.print_styled(pos, &SpannedString::styled(symbol, Style::none()));
      }
    }
  }

  /// Render a non-cursor cell inside the playhead area.
  fn render_playhead_area_cell(
    &self,
    printer: &Printer,
    x: usize,
    y: usize,
    cell_index: usize,
    playhead_ui: &PlayheadUI,
  ) {
    let ch = self.get_display_char(x, y);
    printer.print_styled((x, y), &SpannedString::styled(ch, Style::highlight()));

    if let Some(matcher) = &playhead_ui.text_matcher {
      if let Some(m) = match_covering(matcher, cell_index) {
        let mut regex_indexes = playhead_ui.regex_indexes.lock().unwrap();
        regex_indexes.insert(cell_index);

        let playhead_pos = playhead_ui.playhead_pos;
        let playhead_end = playhead_pos + playhead_ui.playhead_area.size();
        regex_indexes.retain(|&index| {
          let p = self.index_to_xy(&index);
          p.fits(playhead_pos) && p.fits_in(playhead_end)
        });

        let symbol = if m.l <= 1 {
          consts::TRIGGER_CHAR
        } else {
          consts::MATCH_GROUP_CHAR
        };
        printer.print_styled((x, y), &SpannedString::styled(symbol, Style::highlight()));
      }
    }
  }

  /// Channel-dimming bounds for the active playhead channel.
  /// Returns `None` when there is only one channel (no dimming required).
  //
  //            x0    x1
  //            ↓     ↓
  //  col:  0     1     2     3
  //      ┌─────┬─────┬─────┬─────┐
  //      │  1  │  2  │  3  │  4  │  row 0
  // y0 → ├─────╔═════╗─────┬─────┤
  //      │  5  ║  6  ║  7  │  8  │  row 1
  // y1 → └─────╚═════╝─────┴─────┘
  fn channel_bounds(
    &self,
    playhead_pos: Vec2,
    v: usize,
    h: usize,
  ) -> Option<(usize, usize, usize, usize)> {
    if v <= 1 && h <= 1 {
      return None;
    }
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
  }

  /// Precompute tilt sweep geometry shared across every row in this print call.
  fn tilt_metrics(&self, playhead_pos: Vec2, v: usize, h: usize) -> TiltMetrics {
    if v > 1 || h > 1 {
      let col_w = (self.width / v).max(1);
      let row_h = (self.height / h).max(1);
      TiltMetrics {
        playhead_col: (playhead_pos.x / col_w).min(v.saturating_sub(1)),
        playhead_row: (playhead_pos.y / row_h).min(h.saturating_sub(1)),
        col_w,
        row_h,
      }
    } else {
      TiltMetrics {
        playhead_col: 0,
        playhead_row: 0,
        col_w: self.width.max(1),
        row_h: self.height.max(1),
      }
    }
  }

  /// Compute the tilt-adjusted sweep band for row `y`.
  fn row_sweep(
    &self,
    metrics: &TiltMetrics,
    tilt_mode: &TiltMode,
    active_x: usize,
    v: usize,
    h: usize,
    y: usize,
  ) -> RowSweep {
    let row_idx = (y / metrics.row_h).min(h.saturating_sub(1));
    match tilt_mode.sweep_col_for_row(metrics.playhead_col, metrics.playhead_row, row_idx, v, h) {
      None => RowSweep::INACTIVE,
      Some(col_idx) => {
        let start = col_idx * metrics.col_w;
        let end = if col_idx + 1 >= v {
          self.width
        } else {
          start + metrics.col_w
        };
        RowSweep {
          band_start: start,
          band_end: end,
          crosshair_x: start + (active_x % metrics.col_w.max(1)),
        }
      }
    }
  }

  /// Base display style for a cell, before focus-dimming or playhead overlays are applied.
  fn base_cell_style(&self, ctx: &CellStyleContext) -> Style {
    let in_strip = if ctx.v > 1 || ctx.h > 1 {
      ctx.sweep.in_band(ctx.x)
    } else {
      true
    };
    if !in_strip {
      return dim_style();
    }
    if ctx.sweep_mode && ctx.is_regex_match && !ctx.is_in_playhead_area {
      return Style::highlight();
    }
    let in_channel = ctx
      .channel_bounds
      .map(|(x0, x1, y0, y1)| ctx.x >= x0 && ctx.x < x1 && ctx.y >= y0 && ctx.y < y1)
      .unwrap_or(true);
    if in_channel {
      normal_style(ctx.is_regex_match)
    } else {
      dim_style()
    }
  }

  /// Print to the given printer with playhead UI highlighting.
  pub fn print(&self, printer: &Printer, playhead_ui: &PlayheadUI) {
    let PlayheadUI {
      text_matcher,
      playhead_pos,
      playhead_area,
      actived_pos,
      aimed_area,
      sweep_mode,
      tilt_mode,
      grid_v_splits,
      grid_h_splits,
      focus_mode,
      ..
    } = playhead_ui;

    let active_absolute_pos = playhead_pos.saturating_add(actived_pos);
    let v = (*grid_v_splits).max(1);
    let h = (*grid_h_splits).max(1);

    let bounds = self.channel_bounds(*playhead_pos, v, h);
    let metrics = self.tilt_metrics(*playhead_pos, v, h);

    for y in 0..self.height {
      let sweep = self.row_sweep(&metrics, tilt_mode, active_absolute_pos.x, v, h, y);

      for x in 0..self.width {
        let cell_index = x + y * self.width;
        let pos = (x, y);
        let is_in_playhead_area = playhead_area.contains(pos.into());
        let is_active_pos = active_absolute_pos.eq(&pos);
        let is_regex_match = text_matcher
          .as_ref()
          .map(|m| match_covering(m, cell_index).is_some())
          .unwrap_or(false);

        let ch = self.get_display_char(x, y);

        let style = self.base_cell_style(&CellStyleContext {
          x,
          y,
          is_regex_match,
          sweep: &sweep,
          channel_bounds: bounds,
          is_in_playhead_area,
          sweep_mode: *sweep_mode,
          v,
          h,
        });
        printer.print_styled(pos, &SpannedString::styled(ch, style));

        // sweep mode
        if sweep.is_crosshair(x) && !is_active_pos && *sweep_mode {
          let (crosshair_ch, crosshair_style) = if is_regex_match && !is_in_playhead_area {
            ('|', Style::highlight())
          } else {
            (
              '|',
              Style::from_color_style(ColorStyle::front(ColorType::rgb(80, 80, 80))),
            )
          };
          printer.print_styled(pos, &SpannedString::styled(crosshair_ch, crosshair_style));
        }

        // focus mode
        if *focus_mode && !is_in_playhead_area {
          let in_x_range = x >= playhead_area.left() && x < playhead_area.right();
          if !in_x_range {
            printer.print_styled(pos, &SpannedString::styled(ch, dim_style()));
          }
        }

        if is_in_playhead_area {
          if is_active_pos {
            self.render_active_playhead(printer, pos, cell_index, text_matcher);
          } else {
            self.render_playhead_area_cell(printer, x, y, cell_index, playhead_ui);
          }
        }

        // aimed area (for Ctrl+hjkl aiming) - render AFTER playhead to show overlay
        if let Some(aimed_rect) = aimed_area {
          if aimed_rect.contains(pos.into()) {
            let aimed_style = Style::from_color_style(ColorStyle::new(
              ColorType::rgb(255, 255, 255),
              ColorType::rgb(50, 50, 50),
            ));
            printer.print_styled(pos, &SpannedString::styled(ch, aimed_style));
          }
        }
      }
    }
  }
}
