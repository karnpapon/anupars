/// Console panel rendering - ScreenBuffer path.
/// The old Cursive-based Console struct and widgets have been removed.

/// Draw a labeled key-value row at `(x, y)`.
fn draw_kv(
  buf: &mut crate::terminal::buffer::ScreenBuffer,
  x: u16,
  y: u16,
  key: &str,
  value: &str,
  key_col: crate::terminal::cell::Color,
  val_col: crate::terminal::cell::Color,
) {
  use crate::view::printer::{apply_style, CellStyle};
  let kstyle = CellStyle { fg: key_col, bg: crate::terminal::cell::Color::Reset, reverse: false };
  let vstyle = CellStyle { fg: val_col, bg: crate::terminal::cell::Color::Reset, reverse: false };
  let mut cx = x;
  for ch in key.chars() {
    if let Some(c) = buf.get_mut(cx, y) {
      apply_style(c, ch, kstyle);
    }
    cx += 1;
  }
  for ch in value.chars() {
    if let Some(c) = buf.get_mut(cx, y) {
      apply_style(c, ch, vstyle);
    }
    cx += 1;
  }
}

/// Draw a flag toggle: "[x]" or "[ ]" followed by label at `(x, y)`.
fn draw_flag(
  buf: &mut crate::terminal::buffer::ScreenBuffer,
  x: u16,
  y: u16,
  label: &str,
  active: bool,
) {
  use crate::terminal::cell::Color;
  use crate::view::printer::{apply_style, CellStyle};
  let bracket_style = CellStyle::fg_rgb(100, 100, 100);
  let on_style = CellStyle::fg_rgb(255, 255, 255);
  let off_style = CellStyle::fg_rgb(60, 60, 60);
  let label_style = CellStyle { fg: Color::Reset, bg: Color::Reset, reverse: false };

  let tick = if active { 'x' } else { ' ' };
  let flag_str = format!("[{}]", tick);
  let mut cx = x;
  for (i, ch) in flag_str.chars().enumerate() {
    let style = if i == 1 {
      if active { on_style } else { off_style }
    } else {
      bracket_style
    };
    if let Some(c) = buf.get_mut(cx, y) {
      apply_style(c, ch, style);
    }
    cx += 1;
  }
  for ch in label.chars() {
    if let Some(c) = buf.get_mut(cx, y) {
      apply_style(c, ch, label_style);
    }
    cx += 1;
  }
}

/// Render the entire console panel into `buf` at `(x_off, y_off)`.
pub fn draw_console(
  state: &crate::app_state::AppState,
  buf: &mut crate::terminal::buffer::ScreenBuffer,
  x_off: u16,
  y_off: u16,
  w: u16,
  _h: u16,
) {
  use crate::terminal::cell::Color;
  use crate::view::printer::{apply_style, CellStyle};
  let _ = w;

  let key_col = Color::Rgb(80, 80, 80);
  let val_col = Color::Rgb(200, 200, 200);

  // column 0: RGXP editor + flags + ERRR/TOTL/MIDI
  let col0 = x_off + 1;

  let rgxp_label = "RGXP: ";
  for (i, ch) in rgxp_label.chars().enumerate() {
    if let Some(c) = buf.get_mut(col0 + i as u16, y_off) {
      apply_style(c, ch, CellStyle::fg_rgb(80, 80, 80));
    }
  }
  let editor_x = col0 + rgxp_label.len() as u16;
  let focused = matches!(state.focus, crate::app_state::Focus::RegexInput);
  state.line_editor.draw(buf, editor_x, y_off, 20, focused);

  let flg_label = "FLG: ";
  for (i, ch) in flg_label.chars().enumerate() {
    if let Some(c) = buf.get_mut(col0 + i as u16, y_off + 1) {
      apply_style(c, ch, CellStyle::fg_rgb(80, 80, 80));
    }
  }
  let flag_x = col0 + flg_label.len() as u16;
  draw_flag(buf, flag_x, y_off + 1, "i ", state.flags.case_sensitive);
  draw_flag(buf, flag_x + 5, y_off + 1, "m", state.flags.multiline);

  draw_kv(buf, col0, y_off + 2, "ERRR: ", &state.regex_error, key_col, val_col);
  draw_kv(buf, col0, y_off + 3, "TOTL: ", &state.regex_match_count, key_col, val_col);
  draw_kv(buf, col0, y_off + 4, "MIDI: ", &state.midi_status, key_col, val_col);

  // column 1: BPM/DIV/LEN/POS
  let col1 = x_off + 33;
  draw_kv(buf, col1, y_off, "BPM: ", &state.bpm_display, key_col, val_col);
  draw_kv(buf, col1, y_off + 1, "DIV: ", &state.ratio_status, key_col, val_col);
  draw_kv(buf, col1, y_off + 2, "LEN: ", &state.len_status, key_col, val_col);
  draw_kv(buf, col1, y_off + 3, "POS: ", &state.pos_status, key_col, val_col);

  // column 2: MDE/MVE/ACM/CHN/TLT
  let col2 = x_off + 55;
  draw_kv(buf, col2, y_off, "MDE: ", &state.mode_status, key_col, val_col);
  draw_kv(buf, col2, y_off + 1, "MVE: ", &state.movement_status, key_col, val_col);
  draw_kv(buf, col2, y_off + 2, "ACM: ", &state.input_status, key_col, val_col);
  draw_kv(buf, col2, y_off + 3, "CHN: ", &state.chn_status, key_col, val_col);
  draw_kv(buf, col2, y_off + 4, "TLT: ", &state.tilt_status, key_col, val_col);

  // column 3: BUF/SYM/RPL
  let col3 = x_off + 80;
  draw_kv(buf, col3, y_off, "BUF: ", &state.buf_status, key_col, val_col);
  draw_kv(buf, col3, y_off + 1, "SYM: ", &state.sym_status, key_col, val_col);
  draw_kv(buf, col3, y_off + 2, "RPL: ", &state.rpl_status, key_col, val_col);
}
