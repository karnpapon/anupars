use std::sync::mpsc::{channel, Receiver, Sender};

use regex::Regex;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct RegexError {
  id: String,
  warning: bool,
  name: String,
  message: String,
}

#[derive(Debug)]
struct MatchGroup {
  s: String,
}

#[derive(Debug)]
struct Match {
  i: usize,
  l: usize,
  groups: Vec<MatchGroup>,
}

struct RegExpErrorResp {
  id: String,
  warning: bool,
  name: String,
  message: String,
}

struct RegExpResp {
  idx: usize,
  len: usize,
  group: Vec<usize>,
}

enum Message {
  Resolve((Option<RegExpErrorResp>, Vec<RegExpResp>)),
}

struct RegExpHandler {
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
}

impl RegExpHandler {
  fn new() -> Self {
    let (tx, rx) = channel();
    Self { tx, rx }
  }
}

fn process_event(data: &EventData) -> Result<Vec<Match>, Option<RegexError>> {
  let text = &data.text;
  let regex = Regex::new(&data.pattern.to_string()).unwrap();
  let mut matches = Vec::new();
  // let mut index = 0;
  // let mut error: Option<RegexError> = None;

  // let mut last_index = 0;
  for cap in regex.captures_iter(text) {
    // if last_index == regex.capture_locations().end(0).unwrap_or(0) {
    //   error = Some(RegexError {
    //     id: "infinite".to_string(),
    //     warning: true,
    //     name: "SyntaxError".to_string(),
    //     message: "infinite loop occurrence".to_string(),
    //   });
    //   break;
    // }

    // last_index = cap.get(0).map_or(0, |m| m.end());

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

    matches.push(Match {
      i: cap.get(0).unwrap().start(),
      l: cap.get(0).unwrap().as_str().len(),
      groups,
    });

    // if !data.flags.contains('g') {
    //   break;
    // } // Exit if not global regex
  }

  // if let Some(err) = error {
  //   return Err(Some(err));
  // }

  Ok(matches)
}

#[derive(Debug)]
struct EventData {
  text: String,
  pattern: String,
  flags: String,
}

pub fn solve() {
  let data = EventData {
    text: "some text to match".to_string(),
    pattern: "[e-t]".to_string(),
    flags: "g".to_string(),
  };

  match process_event(&data) {
    Ok(matches) => println!("Matches: {:?}", matches),
    Err(Some(err)) => println!("Error: {:?}", err),
    Err(None) => println!("Unknown error."),
  }
}
