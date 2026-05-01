#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::terminal::buffer::ScreenBuffer;
use crate::terminal::cell::Color;
use crate::view::printer::{apply_style, dim as col_dim, primary as col_primary, CellStyle};

const GAP: u16 = 2;
const MAX_W: [u16; 4] = [32, 22, 25, 26];

/// Write `s` into `buf` at `(x, y)` with uniform `style`. Returns x after the last char.
fn draw_str(buf: &mut ScreenBuffer, x: u16, y: u16, s: &str, style: CellStyle) -> u16 {
  let mut cx = x;
  for ch in s.chars() {
    if let Some(c) = buf.get_mut(cx, y) {
      apply_style(c, ch, style);
    }
    cx += 1;
  }
  cx
}

/// Like `draw_str` but stops before `max_x`. Returns x after the last drawn char.
fn draw_str_clip(buf: &mut ScreenBuffer, x: u16, y: u16, s: &str, style: CellStyle, max_x: u16) -> u16 {
  let mut cx = x;
  for ch in s.chars() {
    if cx >= max_x { break; }
    if let Some(c) = buf.get_mut(cx, y) {
      apply_style(c, ch, style);
    }
    cx += 1;
  }
  cx
}

/// Draw a labeled key-value row at `(x, y)`, clipped to `max_x`.
fn draw_kv(buf: &mut ScreenBuffer, x: u16, y: u16, key: &str, value: &str, max_x: u16) {
  let vx = draw_str_clip(buf, x, y, key,   CellStyle { fg: col_dim(),     bg: Color::Reset, reverse: false }, max_x);
  draw_str_clip(buf, vx, y, value, CellStyle { fg: col_primary(), bg: Color::Reset, reverse: false }, max_x);
}

/// Draw a flag toggle: "[x]" or "[ ]" followed by label at `(x, y)`.
fn draw_flag(buf: &mut ScreenBuffer, x: u16, y: u16, label: &str, active: bool, focused: bool) {
  let bracket_style = if focused { CellStyle::fg_rgb(255, 255, 255) } else { CellStyle::fg_rgb(100, 100, 100) };
  let tick_style   = if active  { CellStyle::fg_rgb(255, 255, 255) } else { CellStyle::fg_rgb(60, 60, 60) };
  let label_style  = if focused { CellStyle::fg_rgb(255, 255, 255) } else { CellStyle { fg: Color::Reset, bg: Color::Reset, reverse: false } };

  let flag_str = format!("[{}]", if active { 'x' } else { ' ' });
  let mut cx = x;
  for (i, ch) in flag_str.chars().enumerate() {
    if let Some(c) = buf.get_mut(cx, y) {
      apply_style(c, ch, if i == 1 { tick_style } else { bracket_style });
    }
    cx += 1;
  }
  draw_str(buf, cx, y, label, label_style);
}

/// Render a single mod matrix cell: "[+amt]" or "[    ]" if no route.
fn draw_mod_cell(buf: &mut ScreenBuffer, x: u16, y: u16, amount: Option<f32>, focused: bool) {
  let bracket_style = if focused { CellStyle::fg_rgb(255, 255, 255) } else { CellStyle::fg_rgb(80, 80, 80) };
  let cell_style    = if amount.is_some() { CellStyle::fg_rgb(200, 200, 200) } else { CellStyle::fg_rgb(60, 60, 60) };

  let text = match amount {
    None    => "    ".to_string(),
    Some(v) => format!("{:+.1}", v),
  };

  if let Some(c) = buf.get_mut(x, y) { apply_style(c, '[', bracket_style); }
  let close_x = draw_str(buf, x + 1, y, &text, cell_style);
  if let Some(c) = buf.get_mut(close_x, y) { apply_style(c, ']', bracket_style); }
}

/// Render the entire console panel into `buf` at `(x_off, y_off)`.
pub fn draw_console(
  state: &crate::app_state::AppState,
  buf: &mut ScreenBuffer,
  x_off: u16,
  y_off: u16,
  w: u16,
  _h: u16,
) {
  let dim = CellStyle::fg_rgb(80, 80, 80);

  // column 4: mod matrix, right-anchored
  use crate::core::engine::mod_matrix::{DiceDest, ModSource};
  let mdm_width = 4 + DiceDest::ALL.len() as u16 * 6;
  let col4      = x_off + w.saturating_sub(mdm_width + 6);
  let cells_x   = col4 + 4;

  // distribute col0-col3 proportionally across the space left of the mod matrix
  // original design proportions: 0, 32, 54, 79 out of 105 units
  let avail = col4.saturating_sub(x_off + 1);
  let col0 = x_off + 1;
  let col1 = col0 + avail * 32 / 105;
  let col2 = col0 + avail * 54 / 105;
  let col3 = col0 + avail * 79 / 105;

  let lim0 = (col0 + MAX_W[0]).min(col1.saturating_sub(GAP));
  let lim1 = (col1 + MAX_W[1]).min(col2.saturating_sub(GAP));
  let lim2 = (col2 + MAX_W[2]).min(col3.saturating_sub(GAP));
  let lim3 = (col3 + MAX_W[3]).min(col4.saturating_sub(GAP));

  // column 0: RGXP editor + flags + ERRR/TOTL/MIDI
  let editor_x = draw_str_clip(buf, col0, y_off, "RGXP: ", dim, lim0);
  let editor_w = lim0.saturating_sub(editor_x);
  let focused = matches!(state.focus, crate::app_state::Focus::RegexInput);
  state.line_editor.draw(buf, editor_x, y_off, editor_w, focused);

  let flag_x = draw_str_clip(buf, col0, y_off + 1, "FLG: ", dim, lim0);
  let focus_cs = matches!(state.focus, crate::app_state::Focus::FlagCaseSensitive);
  let focus_ml = matches!(state.focus, crate::app_state::Focus::FlagMultiline);
  draw_flag(buf, flag_x,     y_off + 1, "i ", state.flags.case_sensitive, focus_cs);
  draw_flag(buf, flag_x + 5, y_off + 1, "m",  state.flags.multiline,     focus_ml);

  draw_kv(buf, col0, y_off + 2, "ERRR: ", &state.regex_error,       lim0);
  draw_kv(buf, col0, y_off + 3, "TOTL: ", &state.regex_match_count, lim0);
  draw_kv(buf, col0, y_off + 4, "MIDI: ", &state.midi_status,       lim0);

  // column 1: BPM/DIV/LEN/POS
  draw_kv(buf, col1, y_off,     "BPM: ", &state.bpm_display,  lim1);
  draw_kv(buf, col1, y_off + 1, "DIV: ", &state.ratio_status, lim1);
  draw_kv(buf, col1, y_off + 2, "LEN: ", &state.len_status,   lim1);
  draw_kv(buf, col1, y_off + 3, "POS: ", &state.pos_status,   lim1);

  // column 2: MDE/MVE/ACM/CHN/TLT
  draw_kv(buf, col2, y_off,     "MDE: ", &state.mode_status,     lim2);
  draw_kv(buf, col2, y_off + 1, "MVE: ", &state.movement_status, lim2);
  draw_kv(buf, col2, y_off + 2, "ACM: ", &state.input_status,    lim2);
  draw_kv(buf, col2, y_off + 3, "CHN: ", &state.chn_status,      lim2);
  draw_kv(buf, col2, y_off + 4, "TLT: ", &state.tilt_status,     lim2);

  // column 3: BUF/SYM/RPL
  draw_kv(buf, col3, y_off,     "BUF: ", &state.buf_status, lim3);
  draw_kv(buf, col3, y_off + 1, "SYM: ", &state.sym_status, lim3);
  draw_kv(buf, col3, y_off + 2, "RPL: ", &state.rpl_status, lim3);

  // column 4: mod matrix routing display (3 sources x 3 destinations)
  let cursor = if let crate::app_state::Focus::ModMatrix { row, col } = state.focus {
    Some((row as usize, col as usize))
  } else {
    None
  };
  let focused_col = cursor.map(|(_, c)| c);

  for (col, &dst) in DiceDest::ALL.iter().enumerate() {
    let style = if focused_col == Some(col) { CellStyle::fg_rgb(200, 200, 200) } else { CellStyle::fg_rgb(60, 60, 60) };
    draw_str(buf, cells_x + col as u16 * 6, y_off, dst.label(), style);
  }

  for (row, &src) in ModSource::ALL.iter().enumerate() {
    let row_focused = cursor.map(|(r, _)| r == row).unwrap_or(false);
    let label_style = if row_focused { CellStyle::fg_rgb(200, 200, 200) } else { CellStyle::fg_rgb(80, 80, 80) };
    draw_str(buf, col4 - 1, y_off + 1 + row as u16, src.label(), label_style);

    for (col, &dst) in DiceDest::ALL.iter().enumerate() {
      let cell_focused = cursor.map(|(r, c)| r == row && c == col).unwrap_or(false);
      draw_mod_cell(buf, cells_x + col as u16 * 6, y_off + 1 + row as u16, state.mod_matrix.get_amount(src, dst), cell_focused);
    }
  }

  // debug: mod source values below the matrix
  let pui = &state.playhead_ui;
  let area   = pui.playhead_area;
  let area_w = (area.bottom_right.x.saturating_sub(area.top_left.x) + 1).max(1);
  let area_h = (area.bottom_right.y.saturating_sub(area.top_left.y) + 1).max(1);
  let total  = (area_w * area_h).max(1);
  let linear = pui.actived_pos.y * area_w + pui.actived_pos.x;
  let phase    = (linear as f32 / (total - 1).max(1) as f32).clamp(0.0, 1.0);
  let anchor_x = (pui.playhead_pos.x as f32 / (state.grid_width.max(1) - 1).max(1) as f32).clamp(0.0, 1.0);
  use crate::core::engine::mod_matrix::BAR_COUNT_PERIOD;
  let bar_count = (pui.current_beat % BAR_COUNT_PERIOD) as f32 / BAR_COUNT_PERIOD as f32;
  let debug_str = format!("ph:{:.2} ax:{:.2} br:{:.2}", phase, anchor_x, bar_count);
  draw_str(buf, col4 - 1, y_off + 4, &debug_str, CellStyle::fg_rgb(80, 80, 80));
}
