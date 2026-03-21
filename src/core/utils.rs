use cursive::XY;
use std::time::{Duration, Instant};

use super::consts;

/// build documentation to string splitted by newline.
pub fn build_doc_string(src: &consts::StaticStrStr) -> String {
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

pub fn build_ratio_status_str((_numerator, denominator): (usize, usize)) -> String {
  format!("{denominator}")
}

pub fn build_len_status_str((w, h): (usize, usize)) -> String {
  format!("w:{w},h:{h}")
}

pub fn build_pos_status_str(pos: XY<usize>) -> String {
  format!("x:{:?},y:{:?}", pos.x, pos.y)
}

pub struct Throttler {
  last_call: Instant,
  interval: Duration,
}

impl Throttler {
  pub fn new(interval: Duration) -> Self {
    Throttler {
      last_call: Instant::now() - interval, // Start at a time that allows immediate first call
      interval,
    }
  }

  pub fn call<F>(&mut self, func: F)
  where
    F: Fn(),
  {
    let now = Instant::now();
    if now.duration_since(self.last_call) >= self.interval {
      self.last_call = now;
      func();
    }
  }
}

// Bresenham line algorithm
pub fn bresenham(x0: isize, y0: isize, x1: isize, y1: isize) -> Vec<(usize, usize)> {
  let mut pts = Vec::new();
  let mut x0 = x0;
  let mut y0 = y0;
  let dx = (x1 - x0).abs();
  let sx = if x0 < x1 { 1 } else { -1 };
  let dy = -(y1 - y0).abs();
  let sy = if y0 < y1 { 1 } else { -1 };
  let mut err = dx + dy;
  loop {
    if x0 >= 0 && y0 >= 0 {
      pts.push((x0 as usize, y0 as usize));
    }
    if x0 == x1 && y0 == y1 {
      break;
    }
    let e2 = 2 * err;
    if e2 >= dy {
      err += dy;
      x0 += sx;
    }
    if e2 <= dx {
      err += dx;
      y0 += sy;
    }
  }
  pts
}

/// Rescales the whitespace in `text` so the layout fills `target_width` columns
/// proportionally. Each run of spaces in a line is scaled by the ratio
/// `target_width / original_max_line_width`, preserving at least one space
/// wherever a gap existed in the source. Non-space characters are kept as-is.
pub fn scale_to_width(text: &str, target_width: usize) -> String {
  if target_width == 0 {
    return text.to_string();
  }

  let original_max = text.lines().map(|l| l.len()).max().unwrap_or(0);

  if original_max == 0 || original_max == target_width {
    return text.to_string();
  }

  text
    .lines()
    .map(|line| scale_line(line, original_max, target_width))
    .collect::<Vec<_>>()
    .join("\n")
}

fn scale_line(line: &str, original_max: usize, target: usize) -> String {
  let mut result = String::new();
  let mut chars = line.chars().peekable();

  while chars.peek().is_some() {
    if chars.peek() == Some(&' ') {
      let mut count = 0usize;
      while chars.peek() == Some(&' ') {
        chars.next();
        count += 1;
      }
      // Scale proportionally, keeping at least 1 space so words don't merge.
      let scaled = ((count * target) as f64 / original_max as f64).round() as usize;
      let scaled = scaled.max(1);
      result.extend(std::iter::repeat_n(' ', scaled));
    } else {
      result.push(chars.next().unwrap());
    }
  }

  result
}
