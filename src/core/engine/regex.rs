use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;
use std::time::Instant;

use cursive::views::Canvas;
use cursive::views::TextView;
use regex_lite::Regex;

use crate::core::consts;
use crate::core::playhead;
use crate::view::grid::GridEditor;

/// Maximum text length (in bytes) we will attempt to match against.
const MAX_INPUT_LEN: usize = 10_000;

/// Abort matching if a single `captures_iter` pass takes longer than this.
const MATCH_TIMEOUT: Duration = Duration::from_millis(50);

pub(crate) struct RegexCache {
  last_pattern: String,
  compiled: Option<Regex>,
}

impl RegexCache {
  pub(crate) fn new() -> Self {
    Self {
      last_pattern: String::new(),
      compiled: None,
    }
  }

  /// Returns `Some(&Regex)` only when the pattern is syntactically valid.
  /// Re-compiles only when `pattern` has actually changed.
  fn get(&mut self, pattern: &str) -> Option<&Regex> {
    if pattern != self.last_pattern {
      self.last_pattern = pattern.to_string();
      self.compiled = if pattern.len() >= 2 {
        Regex::new(pattern).ok()
      } else {
        None
      };
    }
    self.compiled.as_ref()
  }
}

#[derive(Debug)]
struct RegexError {
  id: String,
  warning: bool,
  name: String,
  message: String,
}

#[derive(Debug, Clone)]
struct MatchGroup {
  s: String,
}

#[derive(Debug, Clone)]
pub struct Match {
  pub i: usize,
  pub l: usize,
  groups: Vec<MatchGroup>,
}

#[derive(Debug, Clone)]
pub enum Message {
  Solve(EventData),
  Clear,
}

#[derive(Debug, Default, Clone)]
pub struct EventData {
  pub text: String,
  pub pattern: String,
  pub flags: String,
  pub grid_width: usize,
}

pub struct RegExpHandler {
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  cb_sink: cursive::CbSink,
}

impl RegExpHandler {
  pub fn new(cb_sink: cursive::CbSink) -> Self {
    let (tx, rx) = channel();
    Self { tx, rx, cb_sink }
  }

  fn process_event(
    data: &EventData,
    cache: &mut RegexCache,
  ) -> Result<HashMap<usize, Match>, RegexError> {
    // count only non-whitespace bytes
    let meaningful_len = data
      .text
      .bytes()
      .filter(|&b| b != b' ' && b != b'\0' && b != b'\n')
      .count();
    if meaningful_len > MAX_INPUT_LEN {
      return Err(RegexError {
        id: "input_too_large".to_string(),
        warning: true,
        name: "InputError".to_string(),
        message: "input too large".to_string(),
      });
    }

    // Build the regex pattern with flags
    // In Rust regex, flags are added as inline modifiers:
    // (?i) = case insensitive, (?m) = multiline, (?s) = dot matches newline, (?x) = ignore whitespace, (?U) = lazy
    let pattern_with_flags = if !data.flags.is_empty() {
      format!("(?{}){}", data.flags, data.pattern)
    } else {
      data.pattern.to_string()
    };

    // Guard 2: use cache, skips Regex::new if pattern hasn't changed,
    // and returns None (instead of panicking) for invalid patterns like `foo|[`
    let regex = cache.get(&pattern_with_flags).ok_or_else(|| RegexError {
      id: "regex_error".to_string(),
      warning: true,
      name: "SyntaxError".to_string(),
      message: "invalid or too short".to_string(),
    })?;

    let text = &data.text;
    let (byte_to_char, char_to_grid) = build_index_tables(text, data.grid_width);

    let mut matches = HashMap::new();

    #[cfg(not(target_arch = "wasm32"))]
    let deadline = Instant::now();
    #[cfg(target_arch = "wasm32")]
    let deadline_ms = web_sys::window()
      .and_then(|w| w.performance())
      .map(|p| p.now() + MATCH_TIMEOUT.as_millis() as f64);

    for cap in regex.captures_iter(text) {
      #[cfg(not(target_arch = "wasm32"))]
      if deadline.elapsed() > MATCH_TIMEOUT {
        return Err(RegexError {
          id: "timeout".to_string(),
          warning: true,
          name: "TimeoutError".to_string(),
          message: "matching timed out".to_string(),
        });
      }
      #[cfg(target_arch = "wasm32")]
      if let Some(dl) = deadline_ms {
        if web_sys::window()
          .and_then(|w| w.performance())
          .map_or(false, |p| p.now() > dl)
        {
          return Err(RegexError {
            id: "timeout".to_string(),
            warning: true,
            name: "TimeoutError".to_string(),
            message: "matching timed out".to_string(),
          });
        }
      }

      let groups: Vec<MatchGroup> = cap
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
          if i == 0 {
            None
          } else {
            Some(MatchGroup {
              s: s?.as_str().to_string(),
            })
          }
        })
        .collect();

      let byte_start = cap.get(0).unwrap().start();
      let char_idx = byte_to_char[byte_start];
      let grid_index = char_to_grid[char_idx];

      let match_str = cap.get(0).unwrap().as_str();
      let grid_length = match_str.chars().filter(|&c| c != '\n').count();

      matches.insert(
        grid_index,
        Match {
          i: grid_index,
          l: grid_length,
          groups,
        },
      );
    }

    Ok(matches)
  }

  /// WASM: process one batch of pending messages without blocking.
  #[cfg(target_arch = "wasm32")]
  pub fn wasm_tick(&self, cache: &mut RegexCache) {
    while let Ok(control_message) = self.rx.try_recv() {
      match control_message {
        Message::Clear => {
          let _ = self.cb_sink.send(Box::new(move |s| {
            let _ = s
              .call_on_name(
                crate::core::consts::canvas_editor_section_view,
                |c: &mut cursive::views::Canvas<crate::view::grid::GridEditor>| {
                  c.state_mut()
                    .playhead_tx
                    .send(crate::core::playhead::Message::SetMatcher(None))
                },
              )
              .unwrap();
            s.call_on_name(
              crate::core::consts::regex_matches_amount_unit_view,
              |c: &mut cursive::views::TextView| c.set_content("-"),
            );
          }));
        }
        Message::Solve(data) => {
          let result = Self::process_event(&data, cache);
          self
            .cb_sink
            .send(Box::new(move |s| {
              let res = match result {
                Ok(matches) => {
                  let total_matches = matches.len();
                  let mm = if matches.is_empty() {
                    None
                  } else {
                    Some(matches)
                  };
                  let _ = s
                    .call_on_name(
                      crate::core::consts::canvas_editor_section_view,
                      |c: &mut cursive::views::Canvas<crate::view::grid::GridEditor>| {
                        c.state_mut()
                          .playhead_tx
                          .send(crate::core::playhead::Message::SetMatcher(mm))
                      },
                    )
                    .unwrap();
                  s.call_on_name(
                    crate::core::consts::regex_matches_amount_unit_view,
                    |c: &mut cursive::views::TextView| c.set_content(total_matches.to_string()),
                  );
                  "".to_string()
                }
                Err(err) => err.message,
              };
              if !res.is_empty() {
                s.call_on_name(
                  crate::core::consts::regex_err_display_unit_view,
                  |c: &mut cursive::views::TextView| c.set_content(res),
                )
                .unwrap()
              } else {
                s.call_on_name(
                  crate::core::consts::regex_err_display_unit_view,
                  |c: &mut cursive::views::TextView| c.set_content("-"),
                )
                .unwrap();
              }
            }))
            .unwrap();
        }
      }
    }
  }

  pub fn run(self) {
    // Cache lives for the lifetime of this thread, one allocation, reused every solve.
    let mut cache = RegexCache::new();

    for control_message in &self.rx {
      match control_message {
        Message::Clear => {
          let _ = self.cb_sink.send(Box::new(move |s| {
            let _ = s
              .call_on_name(
                consts::canvas_editor_section_view,
                |c: &mut Canvas<GridEditor>| {
                  c.state_mut()
                    .playhead_tx
                    .send(playhead::Message::SetMatcher(None))
                },
              )
              .unwrap();

            s.call_on_name(
              consts::regex_matches_amount_unit_view,
              |c: &mut TextView| c.set_content("-"),
            );
          }));
        }
        Message::Solve(data) => {
          let result = Self::process_event(&data, &mut cache);

          self
            .cb_sink
            .send(Box::new(move |s| {
              let res = match result {
                Ok(matches) => {
                  let total_matches = matches.len();
                  let mm = if matches.is_empty() {
                    None
                  } else {
                    Some(matches)
                  };

                  let _ = s
                    .call_on_name(
                      consts::canvas_editor_section_view,
                      |c: &mut Canvas<GridEditor>| {
                        c.state_mut()
                          .playhead_tx
                          .send(playhead::Message::SetMatcher(mm))
                      },
                    )
                    .unwrap();

                  s.call_on_name(
                    consts::regex_matches_amount_unit_view,
                    |c: &mut TextView| c.set_content(total_matches.to_string()),
                  );

                  "".to_string()
                }
                Err(err) => err.message,
              };

              if !res.is_empty() {
                // s.call_on_name(consts::display_view, |c: &mut TextView| {
                //   c.set_content(res.clone())
                // })
                // .unwrap();
                s.call_on_name(consts::regex_err_display_unit_view, |c: &mut TextView| {
                  c.set_content(res)
                })
                .unwrap()
              } else {
                s.call_on_name(consts::regex_err_display_unit_view, |c: &mut TextView| {
                  c.set_content("-")
                })
                .unwrap();
              }
            }))
            .unwrap();
        }
      }
    }
  }
}

/// Build two lookup tables in one O(n) pass over the text:
///
/// - `byte_to_char[byte_offset]  → char_index`
///   Needed because `regex_lite` returns byte offsets, but the grid is
///   indexed by character position (multi-byte chars like 'ï' span >1 byte).
///
/// - `char_to_grid[char_index]   → grid_cell_index`
///   Needed because newlines are not stored in the grid; instead the rest
///   of the row is padded with '\0', so every line after the first shifts
///   all subsequent char positions forward by the padding amount.
fn build_index_tables(text: &str, grid_width: usize) -> (Vec<usize>, Vec<usize>) {
  let mut byte_to_char = vec![0usize; text.len() + 1];
  let mut char_to_grid: Vec<usize> = Vec::with_capacity(text.len());
  let mut grid_idx: usize = 0;
  let mut row_pos: usize = 0;
  for (char_idx, (byte_idx, ch)) in text.char_indices().enumerate() {
    byte_to_char[byte_idx] = char_idx;
    char_to_grid.push(grid_idx);
    if ch == '\n' {
      let padding = if grid_width > 0 {
        grid_width.saturating_sub(row_pos)
      } else {
        0
      };
      grid_idx += padding;
      row_pos = 0;
    } else {
      grid_idx += 1;
      row_pos += 1;
      if grid_width > 0 && row_pos >= grid_width {
        row_pos = 0;
      }
    }
  }
  (byte_to_char, char_to_grid)
}

#[cfg(test)]
impl Match {
  /// Test-only constructor
  pub fn make_test(i: usize, l: usize) -> Self {
    Self {
      i,
      l,
      groups: vec![],
    }
  }
}

#[cfg(test)]
mod tests {
  use super::build_index_tables;

  #[test]
  fn byte_to_char_ascii() {
    // text: "abcd"  (4 chars, 4 bytes)
    let (b2c, _) = build_index_tables("abcd", 10);
    assert_eq!(b2c[0], 0); // 'a' → char 0
    assert_eq!(b2c[1], 1); // 'b' → char 1
    assert_eq!(b2c[2], 2); // 'c' → char 2
    assert_eq!(b2c[3], 3); // 'd' → char 3
  }

  /// Multi-byte: 'ï' (U+00EF) is 2 bytes (0xC3 0xAF).
  /// Without this table a regex match on 'v' would use byte offset 4
  /// as char index and land one cell too far in the grid.
  ///
  ///   text:        n  a  ï     v  e
  ///   byte offset: 0  1  2  3  4  5
  ///   char index:  0  1  2     3  4
  #[test]
  fn byte_to_char_multibyte() {
    let (b2c, _) = build_index_tables("naïve", 10);
    assert_eq!(b2c[0], 0); // 'n' starts at byte 0 → char 0
    assert_eq!(b2c[1], 1); // 'a' starts at byte 1 → char 1
    assert_eq!(b2c[2], 2); // 'ï' starts at byte 2 → char 2
                           // byte 3 is the second byte of 'ï'
    assert_eq!(b2c[4], 3); // 'v' starts at byte 4 → char 3
    assert_eq!(b2c[5], 4); // 'e' starts at byte 5 → char 4
  }

  #[test]
  fn char_to_grid_no_newlines() {
    // text: "abcd", grid_width: 10 - one row, no wrapping
    let (_, c2g) = build_index_tables("abcd", 10);
    assert_eq!(c2g, vec![0, 1, 2, 3]);
  }

  /// Newline with padding.
  ///
  ///   text: "ab\ncd",  grid_width: 4
  ///
  ///   grid layout (4 cols):
  ///     col:   0    1    2    3
  ///     row 0: 'a'  'b'  '\0' '\0'   ← '\n' pads 2 cells
  ///     row 1: 'c'  'd'  '\0' '\0'
  ///
  ///   char_to_grid should be: a→0, b→1, \n→2, c→4, d→5
  ///
  /// The '\n' char maps to grid index 2 (where the newline sits before padding).
  /// 'c' maps to 4 because the 2 padding cells (indices 2 and 3) are skipped.
  #[test]
  fn char_to_grid_newline_padding() {
    let (_, c2g) = build_index_tables("ab\ncd", 4);
    assert_eq!(c2g[0], 0); // 'a' → grid 0
    assert_eq!(c2g[1], 1); // 'b' → grid 1
    assert_eq!(c2g[2], 2); // '\n' → grid 2  (padding: 4-2=2 cells skipped)
    assert_eq!(c2g[3], 4); // 'c' → grid 4  (skipped over cells 2 and 3)
    assert_eq!(c2g[4], 5); // 'd' → grid 5
  }

  /// Row that fills exactly to grid_width, then a newline follows.
  ///
  ///   text: "abcd\nef",  grid_width: 4
  ///
  ///   When 'd' (char 3) fills col 3, row_pos wraps to 0 immediately.
  ///   The '\n' that follows sees row_pos=0 and computes padding = 4-0 = 4,
  ///   so it occupies grid cell 4 and the next row of text ('e') starts at 8.
  ///
  ///   grid layout:
  ///     row 0 (cells 0-3):  a  b  c  d
  ///     row 1 (cells 4-7):  \n \0 \0 \0
  ///     row 2 (cells 8-11): e  f  …
  #[test]
  fn char_to_grid_full_row_then_newline() {
    let (_, c2g) = build_index_tables("abcd\nef", 4);
    assert_eq!(c2g[0], 0); // 'a' → grid 0
    assert_eq!(c2g[3], 3); // 'd' → grid 3
    assert_eq!(c2g[4], 4); // '\n' → grid 4  (row_pos was 0, padding = 4)
    assert_eq!(c2g[5], 8); // 'e' → grid 8  (skipped cells 5-7)
    assert_eq!(c2g[6], 9); // 'f' → grid 9
  }

  /// grid_width == 0: no padding is ever added (safe, no integer underflow).
  ///
  ///   text: "ab\ncd",  grid_width: 0
  ///
  ///   '\n' adds 0 padding, so grid_idx does not advance after it.
  ///   'c' therefore maps to the same grid_idx value that '\n' occupied,
  ///   then advances by 1 as normal.
  ///
  ///   char_to_grid: a→0, b→1, \n→2, c→2, d→3
  #[test]
  fn char_to_grid_zero_width_no_panic() {
    let (_, c2g) = build_index_tables("ab\ncd", 0);
    assert_eq!(c2g[0], 0); // 'a'
    assert_eq!(c2g[1], 1); // 'b'
    assert_eq!(c2g[2], 2); // '\n' → grid 2, padding = 0
    assert_eq!(c2g[3], 2); // 'c' → grid 2 (grid_idx not advanced by '\n')
    assert_eq!(c2g[4], 3); // 'd' → grid 3
  }

  #[test]
  fn empty_text() {
    let (b2c, c2g) = build_index_tables("", 10);
    assert_eq!(b2c.len(), 1); // vec![0usize; 0 + 1]
    assert!(c2g.is_empty());
  }
}
