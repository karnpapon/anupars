use cursive::theme::ColorStyle;
use cursive::theme::ColorType;
use cursive::theme::Style;
use cursive::Printer;
use cursive::Vec2;

use crate::core::consts;
use crate::core::engine::regex::Match;
use crate::core::playhead::tilt::TiltMode;
use crate::core::playhead::types::SweepRowMode;
use crate::core::playhead::PlayheadUI;
use crate::view::consts::{EMPTY_CHAR, REST_CHAR};
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
      '\0' if self.should_rest(pos) => REST_CHAR,
      '\0' => EMPTY_CHAR,
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
  sweep_row_mode: SweepRowMode,
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

impl<T: Printable + Copy> Matrix<T> {
  fn get_display_char(&self, x: usize, y: usize) -> char {
    self.get(x, y).unwrap().display_char((x, y).into())
  }

  /// Channel-dimming bounds for the active playhead area.
  /// Returns `None` when there is only one channel (no dimming required).
  /// When the playhead area spans multiple channels the returned bounds are
  /// the union of every channel the area touches, so all spanned channels
  /// are rendered at normal brightness.
  //
  //            x0         x1
  //            ↓          ↓
  //  col:  0     1     2     3
  //      ┌─────┬─────┬─────┬─────┐
  //      │  1  │  2  │  3  │  4  │  row 0
  // y0 → ├─────╔═════╪═════╗─────┤
  //      │  5  ║  6  │  7  ║  8  │  row 1
  // y1 → └─────╚═════╧═════╝─────┘
  fn channel_bounds(
    &self,
    playhead_pos: Vec2,
    area_end: Vec2,
    v: usize,
    h: usize,
  ) -> Option<(usize, usize, usize, usize)> {
    if v <= 1 && h <= 1 {
      return None;
    }
    let col_w = (self.width / v).max(1);
    let row_h = (self.height / h).max(1);

    let head_col = (playhead_pos.x / col_w).min(v.saturating_sub(1));
    let head_row = (playhead_pos.y / row_h).min(h.saturating_sub(1));
    let tail_col = (area_end.x / col_w).min(v.saturating_sub(1));
    let tail_row = (area_end.y / row_h).min(h.saturating_sub(1));

    let min_col = head_col.min(tail_col);
    let max_col = head_col.max(tail_col);
    let min_row = head_row.min(tail_row);
    let max_row = head_row.max(tail_row);

    let x0 = min_col * col_w;
    let x1 = if max_col + 1 >= v {
      self.width
    } else {
      (max_col + 1) * col_w
    };
    let y0 = min_row * row_h;
    let y1 = if max_row + 1 >= h {
      self.height
    } else {
      (max_row + 1) * row_h
    };

    Some((x0, x1, y0, y1))
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
    // Use the column that active_x is currently in (not the area's head column) so the
    // crosshair follows the active position across channel boundaries instead of wrapping.
    let active_col = (active_x / metrics.col_w.max(1)).min(v.saturating_sub(1));
    match tilt_mode.sweep_col_for_row(active_col, metrics.playhead_row, row_idx, v, h) {
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
      let in_channel_x = ctx
        .channel_bounds
        .map(|(x0, x1, _, _)| ctx.x >= x0 && ctx.x < x1)
        .unwrap_or(false);
      ctx.sweep.in_band(ctx.x) || in_channel_x
    } else {
      true
    };
    if !in_strip {
      return dim_style();
    }
    if ctx.sweep_mode
      && ctx.is_regex_match
      && !ctx.is_in_playhead_area
      && ctx.sweep.in_band(ctx.x)
      && ctx.sweep_row_mode.is_row_active(ctx.y)
    {
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
      sweep_row_mode,
      drone_mode,
      drone_x,
      tilt_mode,
      grid_v_splits,
      grid_h_splits,
      focus_mode,
      ..
    } = playhead_ui;

    let active_absolute_pos = playhead_pos.saturating_add(actived_pos);
    let crosshair_abs_x = if playhead_ui.sweep_movement.is_some() {
      playhead_ui.sweep_x
    } else {
      active_absolute_pos.x
    };
    let v = (*grid_v_splits).max(1);
    let h = (*grid_h_splits).max(1);
    let total = self.width * self.height;

    let bounds = self.channel_bounds(*playhead_pos, playhead_area.bottom_right, v, h);
    let metrics = self.tilt_metrics(*playhead_pos, v, h);

    let style_dim = dim_style();
    let style_highlight = Style::highlight();
    let style_none = Style::none();
    let style_light = Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)));
    let style_xhair_dim = Style::from_color_style(ColorStyle::front(ColorType::rgb(80, 80, 80)));
    let style_aimed_bg = Style::from_color_style(ColorStyle::new(
      ColorType::rgb(255, 255, 255),
      ColorType::rgb(50, 50, 50),
    ));

    // Build a flat match-lookup map: cell_index -> &Match.
    let match_map: HashMap<usize, &Match> = text_matcher
      .as_ref()
      .map(|matcher| {
        let mut map = HashMap::new();
        for m in matcher.values() {
          let end = (m.i + m.l.max(1)).min(total);
          for idx in m.i..end {
            map.entry(idx).or_insert(m);
          }
        }
        map
      })
      .unwrap_or_default();

    let crosshair_nearest_match: HashMap<usize, usize> = {
      let mut map: HashMap<usize, (usize, usize)> = HashMap::new();
      for &idx in match_map.keys() {
        let mx = idx % self.width;
        let my = idx / self.width;
        let dist = (my as i32 - active_absolute_pos.y as i32).unsigned_abs() as usize;
        let entry = map.entry(mx).or_insert((dist, my));
        if dist < entry.0 {
          *entry = (dist, my);
        }
      }
      map.into_iter().map(|(x, (_, y))| (x, y)).collect()
    };

    // single str buff reused across all cells
    let mut tmp = String::with_capacity(4);
    let mut new_regex_in_area: Vec<usize> = Vec::new();

    for y in 0..self.height {
      let sweep = self.row_sweep(&metrics, tilt_mode, crosshair_abs_x, v, h, y);

      for x in 0..self.width {
        let cell_index = x + y * self.width;
        let pos = (x, y);
        let ch = self.get_display_char(x, y);

        let is_in_playhead_area = playhead_area.contains((x, y).into());
        let is_active_pos = x == active_absolute_pos.x && y == active_absolute_pos.y;
        let matched = match_map.get(&cell_index);
        let is_regex_match = matched.is_some();

        let mut final_ch = ch;
        let mut final_style = self.base_cell_style(&CellStyleContext {
          x,
          y,
          is_regex_match,
          sweep: &sweep,
          channel_bounds: bounds,
          is_in_playhead_area,
          sweep_mode: *sweep_mode,
          sweep_row_mode: *sweep_row_mode,
          v,
          h,
        });

        // crosshair follows active_pos only, even when area spans multiple channels
        if sweep.is_crosshair(x) && !is_active_pos && *sweep_mode && sweep_row_mode.is_row_active(y)
        {
          final_ch = '|';
          final_style = if is_regex_match && !is_in_playhead_area {
            style_highlight
          } else if crosshair_nearest_match.get(&x).is_some_and(|&sel_y| {
            let lo = active_absolute_pos.y.min(sel_y);
            let hi = active_absolute_pos.y.max(sel_y);
            y >= lo && y <= hi
          }) {
            style_light
          } else {
            style_xhair_dim
          };
        }

        // drone line
        if *drone_mode && x == playhead_area.left() + *drone_x && !is_active_pos {
          let (note_index, _octave) = playhead_ui.scale_mode_left.pos_to_scale_note(
            y,
            self.height,
            consts::BASE_OCTAVE,
            playhead_ui.scale_root_left.to_root_offset(),
          );
          let note_i = note_index.round() as usize % 12;
          let is_black_key = matches!(note_i, 1 | 3 | 6 | 8 | 10);
          let is_c = consts::NOTE_NAMES[note_i] == "C";
          final_ch = if is_regex_match {
            consts::TRIGGER_CHAR
          } else if is_c {
            '┣'
          } else {
            '┃'
          };
          let color = if is_black_key {
            ColorType::rgb(50, 50, 50)
          } else {
            ColorType::rgb(100, 100, 100)
          };
          final_style = if is_regex_match {
            Style::highlight()
          } else {
            Style::from_color_style(ColorStyle::front(color))
          }
        }

        // Focus dim (only for cells outside the playhead x-band).
        if *focus_mode
          && !is_in_playhead_area
          && (x < playhead_area.left() || x >= playhead_area.right())
        {
          final_ch = ch;
          final_style = style_dim;
        }

        // Playhead
        if is_in_playhead_area {
          if is_active_pos {
            final_ch = matched
              .map(|m| {
                if m.l <= 1 {
                  consts::PLAYHEAD_MATCH_CHAR
                } else {
                  consts::MATCH_GROUP_CHAR
                }
              })
              .unwrap_or(consts::PLAYHEAD_CHAR);
            final_style = style_none;
          } else {
            final_style = style_highlight;
            if let Some(m) = matched {
              new_regex_in_area.push(cell_index);
              final_ch = if m.l <= 1 {
                consts::TRIGGER_CHAR
              } else {
                consts::MATCH_GROUP_CHAR
              };
            }
          }
        }

        // aimed pos
        if let Some(aimed_rect) = aimed_area {
          if aimed_rect.contains((x, y).into()) {
            final_ch = ch;
            final_style = if is_regex_match {
              style_highlight
            } else {
              style_aimed_bg
            };
          }
        }

        // Single print per cell
        tmp.clear();
        tmp.push(final_ch);
        printer.with_style(final_style, |p| p.print(pos, &tmp));
      }
    }

    // Update regex_indexes once per frame .
    if !new_regex_in_area.is_empty() {
      let mut regex_indexes = playhead_ui.regex_indexes.lock().unwrap();
      for idx in new_regex_in_area {
        regex_indexes.insert(idx);
      }
      let playhead_end = *playhead_pos + playhead_area.size();
      regex_indexes.retain(|&index| {
        let p = self.index_to_xy(&index);
        p.fits(*playhead_pos) && p.fits_in(playhead_end)
      });
    }
  }
}
