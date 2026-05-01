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
  let kstyle = CellStyle {
    fg: key_col,
    bg: crate::terminal::cell::Color::Reset,
    reverse: false,
  };
  let vstyle = CellStyle {
    fg: val_col,
    bg: crate::terminal::cell::Color::Reset,
    reverse: false,
  };
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
  focused: bool,
) {
  use crate::terminal::cell::Color;
  use crate::view::printer::{apply_style, CellStyle};
  let bracket_style = if focused {
    CellStyle::fg_rgb(255, 255, 255)
  } else {
    CellStyle::fg_rgb(100, 100, 100)
  };
  let on_style = CellStyle::fg_rgb(255, 255, 255);
  let off_style = CellStyle::fg_rgb(60, 60, 60);
  let label_style = CellStyle {
    fg: if focused {
      Color::Rgb(255, 255, 255)
    } else {
      Color::Reset
    },
    bg: Color::Reset,
    reverse: false,
  };

  let tick = if active { 'x' } else { ' ' };
  let flag_str = format!("[{}]", tick);
  let mut cx = x;
  for (i, ch) in flag_str.chars().enumerate() {
    let style = if i == 1 {
      if active {
        on_style
      } else {
        off_style
      }
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

/// Render a single mod matrix cell: "[amt]" or "[   ]" if no route.
fn draw_mod_cell(
  buf: &mut crate::terminal::buffer::ScreenBuffer,
  x: u16,
  y: u16,
  amount: Option<f32>,
  focused: bool,
) {
  use crate::view::printer::{apply_style, CellStyle};

  let active_style = CellStyle::fg_rgb(200, 200, 200);
  let dim_style = CellStyle::fg_rgb(60, 60, 60);
  let bracket_style = if focused {
    CellStyle::fg_rgb(255, 255, 255)
  } else {
    CellStyle::fg_rgb(80, 80, 80)
  };

  // 4-char inner width: always sign-prefixed so all values are same width
  let text = match amount {
    None => "    ".to_string(),
    Some(v) => format!("{:+.1}", v),
  };
  let cell_style = if amount.is_some() {
    active_style
  } else {
    dim_style
  };

  let chars: Vec<char> = format!("[{}]", text).chars().collect();
  for (i, &ch) in chars.iter().enumerate() {
    let style = if i == 0 || i == chars.len() - 1 {
      bracket_style
    } else {
      cell_style
    };
    if let Some(c) = buf.get_mut(x + i as u16, y) {
      apply_style(c, ch, style);
    }
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
  let focus_cs = matches!(state.focus, crate::app_state::Focus::FlagCaseSensitive);
  let focus_ml = matches!(state.focus, crate::app_state::Focus::FlagMultiline);
  draw_flag(
    buf,
    flag_x,
    y_off + 1,
    "i ",
    state.flags.case_sensitive,
    focus_cs,
  );
  draw_flag(
    buf,
    flag_x + 5,
    y_off + 1,
    "m",
    state.flags.multiline,
    focus_ml,
  );

  draw_kv(
    buf,
    col0,
    y_off + 2,
    "ERRR: ",
    &state.regex_error,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col0,
    y_off + 3,
    "TOTL: ",
    &state.regex_match_count,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col0,
    y_off + 4,
    "MIDI: ",
    &state.midi_status,
    key_col,
    val_col,
  );

  // column 1: BPM/DIV/LEN/POS
  let col1 = x_off + 33;
  draw_kv(
    buf,
    col1,
    y_off,
    "BPM: ",
    &state.bpm_display,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col1,
    y_off + 1,
    "DIV: ",
    &state.ratio_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col1,
    y_off + 2,
    "LEN: ",
    &state.len_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col1,
    y_off + 3,
    "POS: ",
    &state.pos_status,
    key_col,
    val_col,
  );

  // column 2: MDE/MVE/ACM/CHN/TLT
  let col2 = x_off + 55;
  draw_kv(
    buf,
    col2,
    y_off,
    "MDE: ",
    &state.mode_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col2,
    y_off + 1,
    "MVE: ",
    &state.movement_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col2,
    y_off + 2,
    "ACM: ",
    &state.input_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col2,
    y_off + 3,
    "CHN: ",
    &state.chn_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col2,
    y_off + 4,
    "TLT: ",
    &state.tilt_status,
    key_col,
    val_col,
  );

  // column 3: BUF/SYM/RPL
  let col3 = x_off + 80;
  draw_kv(
    buf,
    col3,
    y_off,
    "BUF: ",
    &state.buf_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col3,
    y_off + 1,
    "SYM: ",
    &state.sym_status,
    key_col,
    val_col,
  );
  draw_kv(
    buf,
    col3,
    y_off + 2,
    "RPL: ",
    &state.rpl_status,
    key_col,
    val_col,
  );

  // column 4: mod matrix routing display (3 sources x 3 destinations)
  // right-aligned: block width = 4 (label+gap) + n_dests * 6 (widest row is the header)
  use crate::core::engine::mod_matrix::{DiceDest, ModSource};
  let mdm_width = 4 + DiceDest::ALL.len() as u16 * 6;
  let col4 = x_off + w.saturating_sub(mdm_width) - 6;

  // let key_dim = CellStyle::fg_rgb(80, 80, 80);
  let dest_dim = CellStyle::fg_rgb(60, 60, 60);
  // for (i, ch) in "MDM: ".chars().enumerate() {
  //   if let Some(c) = buf.get_mut(col4 + i as u16, y_off) {
  //     apply_style(c, ch, key_dim);
  //   }
  // }
  let cells_x = col4 + 4;

  // rows: one per source, cells for each dest
  let cursor = if let crate::app_state::Focus::ModMatrix { row, col } = state.focus {
    Some((row as usize, col as usize))
  } else {
    None
  };

  let focused_col = cursor.map(|(_, c)| c);

  for (col, &dst) in DiceDest::ALL.iter().enumerate() {
    let col_style = if focused_col == Some(col) {
      CellStyle::fg_rgb(200, 200, 200)
    } else {
      dest_dim
    };
    for (i, ch) in dst.label().chars().enumerate() {
      if let Some(c) = buf.get_mut(cells_x + col as u16 * 6 + i as u16, y_off) {
        apply_style(c, ch, col_style);
      }
    }
  }

  for (row, &src) in ModSource::ALL.iter().enumerate() {
    let row_focused = cursor.map(|(r, _)| r == row).unwrap_or(false);
    let label_style = if row_focused {
      CellStyle::fg_rgb(200, 200, 200)
    } else {
      CellStyle::fg_rgb(80, 80, 80)
    };
    for (i, ch) in src.label().chars().enumerate() {
      if let Some(c) = buf.get_mut((col4 - 1) + i as u16, y_off + 1 + row as u16) {
        apply_style(c, ch, label_style);
      }
    }
    for (col, &dst) in DiceDest::ALL.iter().enumerate() {
      let cell_focused = cursor.map(|(r, c)| r == row && c == col).unwrap_or(false);
      let amount = state.mod_matrix.get_amount(src, dst);
      draw_mod_cell(
        buf,
        cells_x + col as u16 * 6,
        y_off + 1 + row as u16,
        amount,
        cell_focused,
      );
    }
  }

  // debug: show current MovementPhase value below the matrix
  let pui = &state.playhead_ui;
  let area = pui.playhead_area;
  let area_w = (area.bottom_right.x.saturating_sub(area.top_left.x) + 1).max(1);
  let area_h = (area.bottom_right.y.saturating_sub(area.top_left.y) + 1).max(1);
  let total = (area_w * area_h).max(1);
  let linear = pui.actived_pos.y * area_w + pui.actived_pos.x;
  let phase = (linear as f32 / (total - 1).max(1) as f32).clamp(0.0, 1.0);
  let grid_w = state.grid_width.max(1);
  let anchor_x = (pui.playhead_pos.x as f32 / (grid_w - 1).max(1) as f32).clamp(0.0, 1.0);
  use crate::core::engine::mod_matrix::BAR_COUNT_PERIOD;
  let bar_count = (pui.current_beat % BAR_COUNT_PERIOD) as f32 / BAR_COUNT_PERIOD as f32;
  let debug_str = format!("ph:{:.2} ax:{:.2} br:{:.2}", phase, anchor_x, bar_count);
  let debug_style = CellStyle::fg_rgb(80, 80, 80);
  for (i, ch) in debug_str.chars().enumerate() {
    if let Some(c) = buf.get_mut(col4 + i as u16 - 1, y_off + 4) {
      apply_style(c, ch, debug_style);
    }
  }
}
