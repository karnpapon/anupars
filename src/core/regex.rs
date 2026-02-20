use std::{
  collections::HashMap,
  sync::mpsc::{channel, Receiver, Sender},
};

use cursive::views::{Canvas, TextView};
use regex::Regex;

use serde::{Deserialize, Serialize};

use crate::view::common::{canvas_editor::CanvasEditor, marker};

use super::consts;

#[derive(Debug, Serialize, Deserialize)]
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
  i: usize,
  l: usize,
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

  // example case:
  // without this conversion,
  // a matches position will be incorrect for multi-byte char eg. "naïve"
  // UTF-8 bytes:  [n][a][ï=2bytes][v][e]
  // Byte indices:   0  1  2   3    4  5
  // Char indices:   0  1  2        3  4
  /// Convert byte index to character index for proper Unicode handling
  fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text
      .char_indices()
      .position(|(i, _)| i == byte_index)
      .unwrap_or(0)
  }

  /// Convert text character index to grid index, accounting for newlines
  /// When a newline is encountered, the rest of that row is filled with '\0',
  /// so we need to skip those positions in the grid index calculation
  fn text_index_to_grid_index(text: &str, text_index: usize, grid_width: usize) -> usize {
    let mut grid_index = 0;
    let mut current_row_position = 0;

    for (idx, ch) in text.chars().enumerate() {
      if idx >= text_index {
        break;
      }

      if ch == '\n' {
        // Skip to the end of current row (padding with '\0')
        let padding = grid_width - current_row_position;
        grid_index += padding;
        current_row_position = 0;
      } else {
        grid_index += 1;
        current_row_position += 1;

        // If we've reached the end of a row, move to next row
        if current_row_position >= grid_width {
          current_row_position = 0;
        }
      }
    }

    grid_index
  }

  fn process_event(data: &EventData) -> Result<HashMap<usize, Match>, RegexError> {
    // Build the regex pattern with flags
    // In Rust regex, flags are added as inline modifiers:
    // (?i) = case insensitive, (?m) = multiline, (?s) = dot matches newline, (?x) = ignore whitespace, (?U) = lazy
    let pattern_with_flags = if !data.flags.is_empty() {
      format!("(?{}){}", data.flags, data.pattern)
    } else {
      data.pattern.to_string()
    };

    match Regex::new(&pattern_with_flags) {
      Ok(regex) => {
        let text = &data.text;

        let mut matches = HashMap::new();

        for cap in regex.captures_iter(text) {
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
          // Convert byte index to character index for multi-byte Unicode support
          let text_char_index = Self::byte_to_char_index(text, byte_start);

          // Convert text character index to grid index (accounting for newlines)
          let grid_index = Self::text_index_to_grid_index(text, text_char_index, data.grid_width);

          // Calculate the grid length (excluding newlines from the match)
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
      Err(e) => Err(RegexError {
        id: "regex_error".to_string(),
        warning: true,
        name: "SyntaxError".to_string(),
        message: e.to_string(),
      }),
    }
  }

  pub fn run(self) {
    for control_message in &self.rx {
      match control_message {
        Message::Clear => {
          let _ = self.cb_sink.send(Box::new(move |s| {
            let _ = s
              .call_on_name(
                consts::canvas_editor_section_view,
                |c: &mut Canvas<CanvasEditor>| {
                  c.state_mut()
                    .marker_tx
                    .send(marker::Message::SetMatcher(None))
                },
              )
              .unwrap();
          }));
        }
        Message::Solve(data) => {
          self
            .cb_sink
            .send(Box::new(move |s| {
              let res = match Self::process_event(&data) {
                Ok(matches) => {
                  let mm = if matches.is_empty() {
                    None
                  } else {
                    Some(matches)
                  };

                  let _ = s
                    .call_on_name(
                      consts::canvas_editor_section_view,
                      |c: &mut Canvas<CanvasEditor>| {
                        c.state_mut()
                          .marker_tx
                          .send(marker::Message::SetMatcher(mm))
                      },
                    )
                    .unwrap();

                  "".to_string()
                }
                Err(err) => err.message,
              };

              if !res.is_empty() {
                s.call_on_name(consts::display_view, |c: &mut TextView| c.set_content(res))
                  .unwrap();
              }
            }))
            .unwrap();
        }
      }
    }
  }
}
