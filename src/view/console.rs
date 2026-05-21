#![cfg_attr(rustfmt, rustfmt_skip)]
use crate::app_state::SYNTH_BUF_SIZE;
use crate::core::consts;
use crate::terminal::buffer::ScreenBuffer;
use crate::view::printer::{apply_style, CellStyle};
use crate::core::engine::mod_matrix::{DiceDest, ModSource};
use crate::app_state::Focus;
use super::consts::CONSOLE_HEIGHT;
use crate::core::engine::mod_matrix::BAR_COUNT_PERIOD;

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
fn draw_kv(buf: &mut ScreenBuffer, x: u16, y: u16, key: &str, value: &str, max_x: u16) -> u16 {
  let vx = draw_str_clip(buf, x, y, key,   CellStyle::dim(),     max_x);
  draw_str_clip(buf, vx, y, value, CellStyle::primary(), max_x)
}

/// Draw a flag toggle: "[x]" or "[ ]" followed by label at `(x, y)`.
fn draw_flag(buf: &mut ScreenBuffer, x: u16, y: u16, label: &str, active: bool, focused: bool) {
  let bracket_style = if focused { CellStyle::white() } else { CellStyle::secondary() };
  let tick_style   = if active  { CellStyle::white() } else { CellStyle::canvas() };
  let label_style  = if focused { CellStyle::white() } else { CellStyle::reset() };

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
  let bracket_style = if focused { CellStyle::white()   } else { CellStyle::dim() };
  let cell_style    = if amount.is_some() { CellStyle::primary() } else { CellStyle::canvas() };

  let text = match amount {
    None    => "    ".to_string(),
    Some(v) => format!("{:+.1}", v),
  };

  if let Some(c) = buf.get_mut(x, y) { apply_style(c, '[', bracket_style); }
  let close_x = draw_str(buf, x + 1, y, &text, cell_style);
  if let Some(c) = buf.get_mut(close_x, y) { apply_style(c, ']', bracket_style); }
}

/// Map a terminal click at `(mx, my)` to the console `Focus` it lands on, or `None`
/// if the click is outside the console content area (rows y_off .. y_off + CONSOLE_HEIGHT).
pub fn hit_test_console(
  mx: u16,
  my: u16,
  x_off: u16,
  y_off: u16,
  w: u16,
) -> Option<crate::app_state::Focus> {
  if my < y_off || my >= y_off + CONSOLE_HEIGHT {
    return None;
  }

  let row = my - y_off;
  let col0: u16 = x_off + 1;

  // mod matrix geometry - mirrors draw_console exactly
  let n_dests: u16 = crate::core::engine::mod_matrix::DiceDest::ALL.len() as u16;
  let mdm_width: u16 = 4 + n_dests * 6;
  let col4 = x_off + w.saturating_sub(mdm_width + 6);
  let cells_x = col4 + 4;

  // mod matrix rows occupy console rows 1-3 (y_off+1 to y_off+3)
  if (1..=3).contains(&row) {
    let mat_row = (row - 1) as u8;
    for c in 0..n_dests {
      let cell_x = cells_x + c * 6;
      if mx >= cell_x && mx < cell_x + 6 {
        return Some(Focus::ModMatrix { row: mat_row, col: c as u8 });
      }
    }
  }

  match row {
    0 => Some(Focus::RegexInput),
    1 => {
      // flags row: "[i] i " at flag_x, "[m]m" at flag_x+5
      let flag_x = col0 + 5; // after "FLG: "
      if mx >= flag_x && mx < flag_x + 5 {
        Some(Focus::FlagCaseSensitive)
      } else if mx >= flag_x + 5 && mx < flag_x + 9 {
        Some(Focus::FlagMultiline)
      } else {
        Some(Focus::RegexInput)
      }
    }
    // any other row inside the console bounds (info-only rows) focuses the RGXP input
    _ => Some(Focus::RegexInput),
  }
}

// Unicode Braille U+2800: each cell is 2 columns x 4 rows of dots.
// Bit layout (dot numbers per BRF standard):
//   left col : dot1=0x01, dot2=0x02, dot3=0x04, dot7=0x40
//   right col: dot4=0x08, dot5=0x10, dot6=0x20, dot8=0x80
//
// This gives 2x horizontal sample density and 4x vertical resolution compared
// to one character = one sample.  Total vertical levels = OSC_ROWS * 4 = 16.

fn braille_l(inner: usize) -> u8 {
  [0x01, 0x02, 0x04, 0x40][inner]
}

fn braille_r(inner: usize) -> u8 {
  [0x08, 0x10, 0x20, 0x80][inner]
}

// Map a pitch-bend i16 value to a dot row index [0 = top/max, total-1 = bottom/min].
fn pb_dot(v: i16, rows: usize) -> usize {
  let total = (rows * 4) as f32 - 1.0;
  let norm = (v as f32 + 8192.0) / 16383.0;
  ((1.0 - norm) * total).round() as usize
}

/// Render a Braille waveform from `samples[0..w*2]` into `rows` character rows at (x, y).
/// Each screen column encodes two consecutive samples as left and right Braille columns.
/// Consecutive samples are connected with vertical line segments so discontinuities
/// (eg. sawtooth resets) render as vertical bars. A faint zero-line guide appears in
/// rows where no signal passes.
/// `total_samp` samples are scaled to fill exactly `w` character columns (2 slots each).
#[allow(clippy::too_many_arguments)]
fn draw_osc_braille(
  buf: &mut ScreenBuffer,
  x: u16,
  y: u16,
  samples: &[i16],
  w: usize,
  rows: usize,
  total_samp: usize,
  active: bool,
  scrub: Option<(usize, usize)>,
  cursor: Option<usize>,
) {
  let total_dots = rows * 4;
  let zero_dot = {
    let norm = 8192.0_f32 / 16383.0; // pitch bend 0 normalized
    ((1.0 - norm) * (total_dots - 1) as f32).round() as usize
  };

  // map a visual slot index [0, w*2) to a sample index in [0, total_samp)
  let total_slots = (w * 2).max(1);
  let sample_at = |slot: usize| -> i16 {
    let idx = (slot * total_samp / total_slots).min(total_samp.saturating_sub(1));
    samples.get(idx).copied().unwrap_or(0)
  };

  // tracks the right-column dot of the previous char, used to connect into the next left column
  let mut prev_r_dot: Option<usize> = None;

  for cx in 0..w {
    let l_val = sample_at(cx * 2);
    let r_val = sample_at(cx * 2 + 1);
    let l_dot = pb_dot(l_val, rows);
    let r_dot = pb_dot(r_val, rows);

    // left column spans from previous right dot to this left dot (cross-char connection)
    let left_from = prev_r_dot.unwrap_or(l_dot);
    let left_lo = left_from.min(l_dot);
    let left_hi = left_from.max(l_dot);

    // right column spans from this left dot to this right dot (intra-char connection)
    let right_lo = l_dot.min(r_dot);
    let right_hi = l_dot.max(r_dot);

    let in_scrub = scrub.is_none_or(|(lo, hi)| cx >= lo && cx < hi);
    let is_cursor = cursor == Some(cx);
    let signal_row = l_dot / 4;

    for row in 0..rows {
      let top = row * 4;
      let mut bits: u8 = 0;

      for inner in 0..4usize {
        let dot = top + inner;
        if dot >= left_lo && dot <= left_hi { bits |= braille_l(inner); }
        if dot >= right_lo && dot <= right_hi { bits |= braille_r(inner); }
      }

      // zero-line guide only in rows where the signal line does not pass
      let left_in_row  = left_hi  >= top && left_lo  <= top + 3;
      let right_in_row = right_hi >= top && right_lo <= top + 3;
      if (top..top + 4).contains(&zero_dot) && !left_in_row && !right_in_row {
        let inner = zero_dot - top;
        bits |= braille_l(inner) | braille_r(inner);
      }

      let has_signal = left_in_row || right_in_row;

      if is_cursor && row == signal_row {
        if let Some(c) = buf.get_mut(x + cx as u16, y + row as u16) {
          apply_style(c, consts::STREAM_CC_CHAR, CellStyle::white());
        }
      } else {
        let ch = if bits == 0 { ' ' } else { char::from_u32(0x2800 | bits as u32).unwrap_or(' ') };
        let style = if !in_scrub {
          CellStyle::secondary()
        } else if has_signal && active {
          CellStyle::primary()
        } else {
          CellStyle::dim()
        };
        if let Some(c) = buf.get_mut(x + cx as u16, y + row as u16) {
          apply_style(c, ch, style);
        }
      }
    }

    prev_r_dot = Some(r_dot);
  }
}

/// Render the waveform-only console panel (toggled with '0'). Splits the console width
/// into two equal oscilloscope lanes side by side, one per MIDI channel, plus a bottom
/// status row.
pub fn draw_waveform_console(
  state: &crate::app_state::AppState,
  buf: &mut ScreenBuffer,
  x_off: u16,
  y_off: u16,
  w: u16,
  h: u16,
) {
  let x_start = x_off + 1;
  let right_lim = w.saturating_sub(x_off + 1);
  let inner_w = right_lim.saturating_sub(x_start) as usize;
  let waveform_rows = (h as usize).saturating_sub(1).max(1);

  // split width evenly with a 1-col separator in the middle
  let half_cols = inner_w.saturating_sub(1) / 2;
  let sep_x     = x_start + half_cols as u16 + 1;
  let ch2_x     = sep_x + 2;
  let ch2_cols  = inner_w.saturating_sub(half_cols + 3);

  // check channel boundary
  let v       = state.playhead_ui.grid_v_splits.max(1);
  let h       = state.playhead_ui.grid_h_splits.max(1);
  let col_w   = (state.grid_width.max(1) / v).max(1);
  let row_h   = (state.grid_height.max(1) / h).max(1);
  let area    = state.playhead_ui.playhead_area;
  let covers_ch = |ch: usize| -> bool {
    let col_idx   = ch % v;
    let row_idx   = ch / v;
    let col_start = col_idx * col_w;
    let col_end   = col_start + col_w - 1;
    let row_start = row_idx * row_h;
    let row_end   = row_start + row_h - 1;
    area.top_left.x <= col_end
      && area.bottom_right.x >= col_start
      && area.top_left.y <= row_end
      && area.bottom_right.y >= row_start
  };

  // scrub zone
  let stream_cc = consts::STREAM_CC_MODE.load(std::sync::atomic::Ordering::Relaxed);
  let scrub_for = |ch_idx: usize, lane_cols: usize| -> Option<(usize, usize)> {
    if !stream_cc || lane_cols == 0 { return None; }
    let col_idx  = ch_idx % v;
    let row_idx  = ch_idx / v;
    let col_start = col_idx * col_w;
    let row_start = row_idx * row_h;
    // same boundary test as covers_ch - return None if playhead is not in this channel
    if !(area.top_left.x < col_start + col_w
      && area.bottom_right.x >= col_start
      && area.top_left.y < row_start + row_h
      && area.bottom_right.y >= row_start) {
      return None;
    }
    let lo = state.playhead_ui.playhead_pos.x.saturating_sub(col_start) * lane_cols / col_w;
    let hi = ((state.playhead_ui.playhead_area.bottom_right.x + 1).saturating_sub(col_start) * lane_cols / col_w).min(lane_cols);
    Some((lo, hi))
  };
  
  let cursor_for = |ch_idx: usize, lane_cols: usize| -> Option<usize> {
    if !stream_cc || lane_cols == 0 { return None; }
    let col_idx  = ch_idx % v;
    let row_idx  = ch_idx / v;
    let col_start = col_idx * col_w;
    let row_start = row_idx * row_h;
    if !(area.top_left.x < col_start + col_w
      && area.bottom_right.x >= col_start
      && area.top_left.y < row_start + row_h
      && area.bottom_right.y >= row_start) {
      return None;
    }
    let abs_x = state.playhead_ui.playhead_pos.x + state.playhead_ui.actived_pos.x;
    if abs_x < col_start || abs_x >= col_start + col_w { return None; }
    Some((abs_x - col_start) * lane_cols / col_w)
  };

  if let Ok(bufs) = state.synth_pb_bufs.lock() {
    if let Some(samples) = bufs.first() {
      draw_osc_braille(buf, x_start, y_off, samples, half_cols, waveform_rows, SYNTH_BUF_SIZE, covers_ch(0), scrub_for(0, half_cols), cursor_for(0, half_cols));
    }
    // vertical separator
    for row in 0..waveform_rows as u16 {
      if let Some(c) = buf.get_mut(sep_x, y_off + row) {
        apply_style(c, '│', CellStyle::dim());
      }
    }
    if let Some(samples) = bufs.get(1) {
      draw_osc_braille(buf, ch2_x, y_off, samples, ch2_cols, waveform_rows, SYNTH_BUF_SIZE, covers_ch(1), scrub_for(1, ch2_cols), cursor_for(1, ch2_cols));
    }

    let cc_val = |ch: usize| -> u8 {
      state.synth_cc_vals.get(ch)
        .map(|a| a.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(0)
    };
    let cc_number = consts::STREAM_CC_NUMBER.load(std::sync::atomic::Ordering::Relaxed);
    let status_y  = y_off + waveform_rows as u16;
    let synth_enabled = consts::SYNTH_ENABLED.load(std::sync::atomic::Ordering::Relaxed);
    let live_dot = |ch: usize| if synth_enabled && covers_ch(ch) { "●" } else { "○" };

    let live1_x = draw_kv(buf, x_start, status_y, "LIVE:", live_dot(0), sep_x.saturating_sub(1));
    let ch1_val_x = draw_kv(buf, live1_x + 1, status_y, "ch1:", &format!("{}", cc_val(0)), sep_x.saturating_sub(1));
    draw_kv(buf, ch1_val_x + 1, status_y, "cc:", &format!("{}", cc_number), sep_x.saturating_sub(1));

    let live2_x = draw_kv(buf, ch2_x, status_y, "LIVE:", live_dot(1), right_lim);
    let ch2_val_x = draw_kv(buf, live2_x + 1, status_y, "ch2:", &format!("{}", cc_val(1)), right_lim);
    draw_kv(buf, ch2_val_x + 1, status_y, "cc:", &format!("{}", cc_number), right_lim);
  }
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
  let dim = CellStyle::dim();

  // column 4: mod matrix, right-anchored
  let mdm_width = 4 + DiceDest::ALL.len() as u16 * 6;
  let col4      = x_off + w.saturating_sub(mdm_width + 6);
  let cells_x   = col4 + 4;

  // distribute col0-col3 proportionally across the space left of the mod matrix
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
    let style = if focused_col == Some(col) { CellStyle::primary() } else { CellStyle::canvas() };
    draw_str(buf, cells_x + col as u16 * 6, y_off, dst.label(), style);
  }

  for (row, &src) in ModSource::ALL.iter().enumerate() {
    let row_focused = cursor.map(|(r, _)| r == row).unwrap_or(false);
    let label_style = if row_focused { CellStyle::primary() } else { CellStyle::dim() };
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
  let bar_count = (pui.current_beat % BAR_COUNT_PERIOD) as f32 / BAR_COUNT_PERIOD as f32;
  let debug_str = format!("ph:{:.2} ax:{:.2} br:{:.2}", phase, anchor_x, bar_count);
  draw_str(buf, col4 - 1, y_off + 4, &debug_str, CellStyle::dim());
}
