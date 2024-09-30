use std::{
  collections::HashMap,
  sync::mpsc::{channel, Receiver, Sender},
};

use cursive::views::{Canvas, TextView};
use regex::Regex;

use serde::{Deserialize, Serialize};

use crate::view::{canvas_base::CanvasBase, canvas_editor::CanvasEditor};

use super::config;

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
  fn process_event(data: &EventData) -> Result<HashMap<usize, Match>, RegexError> {
    match Regex::new(&data.pattern.to_string()) {
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

          matches.insert(
            cap.get(0).unwrap().start(),
            Match {
              i: cap.get(0).unwrap().start(),
              l: cap.get(0).unwrap().as_str().len(),
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
            s.call_on_name(
              config::canvas_base_section_view,
              |c: &mut Canvas<CanvasBase>| c.state_mut().set_text_matcher(None),
            )
            .unwrap();

            s.call_on_name(
              config::canvas_editor_section_view,
              |c: &mut Canvas<CanvasEditor>| c.state_mut().set_text_matcher(None),
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

                  s.call_on_name(
                    config::canvas_base_section_view,
                    |c: &mut Canvas<CanvasBase>| c.state_mut().set_text_matcher(mm.clone()),
                  )
                  .unwrap();

                  s.call_on_name(
                    config::canvas_editor_section_view,
                    |c: &mut Canvas<CanvasEditor>| c.state_mut().set_text_matcher(mm),
                  )
                  .unwrap();

                  "".to_string()
                }
                Err(err) => err.message,
              };

              if !res.is_empty() {
                s.call_on_name(config::regex_display_unit_view, |c: &mut TextView| {
                  c.set_content(res)
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
