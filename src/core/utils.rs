use cursive::{Vec2, XY};

use super::config;

/// build documentation to string splitted by newline.
pub fn build_doc_string(src: &config::StaticStrStr) -> String {
  let mut doc_str = String::new();
  for (command, desc) in src.iter() {
    if command.is_empty() && desc.is_empty() {
      doc_str.push('\n');
      continue;
    }
    doc_str.push_str(format!("{}: {}\n", command, desc).as_str());
  }

  doc_str
}

pub fn replace_nth_char_ascii(s: &mut str, idx: usize, newchar: char) {
  let s_bytes: &mut [u8] = unsafe { s.as_bytes_mut() };
  s_bytes[idx] = newchar as u8;
}

pub fn build_bpm_status_str(bpm: usize) -> String {
  format!("{bpm}")
}

pub fn build_ratio_status_str((numerator, denominator): (i64, usize), tick_str: &str) -> String {
  format!("{numerator}/{denominator}, {tick_str}")
}

pub fn build_len_status_str((w, h): (usize, usize)) -> String {
  format!("w:{w},h:{h}")
}

pub fn build_pos_status_str(pos: XY<usize>) -> String {
  format!("x:{:?},y:{:?}", pos.x, pos.y)
}
