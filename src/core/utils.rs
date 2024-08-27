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

pub fn build_bpm_status_str(bpm: usize) -> String {
  format!("{bpm}")
}

pub fn build_ratio_status_str((numerator, denominator): (usize, usize)) -> String {
  format!("{numerator}/{denominator}")
}

pub fn build_len_status_str((w, h): (usize, usize)) -> String {
  format!("w:{w},h:{h}")
}

pub fn build_pos_status_str(pos: XY<usize>) -> String {
  format!("x:{:?},y:{:?}", pos.x, pos.y)
}
