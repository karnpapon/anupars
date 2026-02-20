use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::view::common::marker;

#[cfg(feature = "desktop")]
use crate::view::desktop::anu::Anu;

#[cfg(feature = "microcontroller")]
use crate::view::microcontroller::anu::Anu;

use super::application::UserData;
use super::clock::metronome::Message;
use super::command::{Adjustment, Command, MoveDirection};
use super::{consts, utils};

use cursive::event::{Event, Key};
use cursive::views::{LinearLayout, TextView};
use cursive::Cursive;
use log::error;
use std::cell::RefCell;

pub struct CommandManager {
  aliases: HashMap<String, String>,
  bindings: RefCell<HashMap<String, Vec<Command>>>,
  anu: Arc<Anu>,
  metronome_sender: Sender<Message>,
  cb_sink: cursive::CbSink,
  temp_tempo: Arc<Mutex<i64>>,
  temp_ratio: Arc<Mutex<(i64, usize)>>,
  pub last_key_time: Arc<Mutex<Option<Instant>>>,
  marker_tx_cloned: Sender<marker::Message>,
}

impl CommandManager {
  pub fn new(
    anu: Anu,
    m_tx: Sender<Message>,
    cb_sink: cursive::CbSink,
    temp_tempo: Arc<Mutex<i64>>,
    last_key_time: Arc<Mutex<Option<Instant>>>,
    marker_tx_cloned: Sender<marker::Message>,
  ) -> Self {
    let bindings = RefCell::new(Self::get_bindings());
    Self {
      aliases: HashMap::new(),
      bindings,
      anu: Arc::new(anu),
      metronome_sender: m_tx,
      cb_sink,
      temp_tempo,
      temp_ratio: Arc::new(Mutex::new((1, 16))),
      last_key_time,
      marker_tx_cloned,
    }
  }

  pub fn get_bindings() -> HashMap<String, Vec<Command>> {
    Self::default_keybindings()
  }

  pub fn register_aliases<S: Into<String>>(&mut self, name: S, aliases: Vec<S>) {
    let name = name.into();
    for a in aliases {
      self.aliases.insert(a.into(), name.clone());
    }
  }

  pub fn register_all(&mut self) {
    self.register_aliases("quit", vec!["q", "x"]);
    self.register_aliases("playpause", vec!["pause", "toggleplay", "toggleplayback"]);
    self.register_aliases("showmenubar", vec!["showmenubar"]);
  }

  fn handle_default_commands(
    &self,
    s: &mut Cursive,
    cmd: &Command,
  ) -> Result<Option<String>, String> {
    match cmd {
      Command::Quit => Ok(None),
      Command::TogglePlay => {
        let _ = self.metronome_sender.send(Message::StartStop);
        Ok(None)
      }
      Command::ShowMenubar => {
        s.select_menubar();
        Ok(None)
      }
      Command::ToggleInputRegexAndCanvas => {
        self.anu.set_toggle_regex_input();

        if !self.anu.toggle_regex_input() {
          let mut display_view = s.find_name::<TextView>(consts::display_view).unwrap();
          display_view.set_content(utils::build_doc_string(&consts::APP_WELCOME_MSG));

          let mut interactive_display_section_view = s
            .find_name::<LinearLayout>(consts::main_section_view)
            .unwrap();
          let _ = interactive_display_section_view.set_focus_index(2); // microcontroller=2, desktop=3
        } else {
          let mut interactive_display_section_view = s
            .find_name::<LinearLayout>(consts::main_section_view)
            .unwrap();
          let _ = interactive_display_section_view.set_focus_index(0);
        }

        Ok(None)
      }
      Command::AdjustMarker(direction) => {
        let dir = match direction {
          MoveDirection::Down => (0, -1),
          MoveDirection::Left => (-1, 0),
          MoveDirection::Right => (1, 0),
          MoveDirection::Up => (0, 1),
        };

        self
          .marker_tx_cloned
          .send(marker::Message::Scale(dir))
          .unwrap();

        Ok(None)
      }
      Command::AdjustBPM(direction) => {
        let nudge = match direction {
          Adjustment::Increase => 1,
          Adjustment::Decrease => -1,
        };

        let mut last_press = self.last_key_time.lock().unwrap();
        *last_press = Some(Instant::now());

        let mut tempo = self.temp_tempo.lock().unwrap();
        *tempo += nudge;
        let temp = *tempo as usize;

        self
          .marker_tx_cloned
          .send(marker::Message::SetTempo(temp))
          .unwrap();

        self
          .cb_sink
          .send(Box::new(move |s| {
            s.call_on_name(consts::bpm_status_unit_view, |view: &mut TextView| {
              view.set_content(utils::build_bpm_status_str(temp));
            })
            .unwrap();
          }))
          .unwrap();

        Ok(None)
      }
      Command::AdjustRatio(direction) => {
        let ratios = [1, 2, 4, 8, 16, 32, 64];

        let current_ratio = *self.temp_ratio.lock().unwrap();
        let current_denom = current_ratio.1;

        let current_idx = ratios.iter().position(|&d| d == current_denom).unwrap_or(4); // Default to 16 if not found

        let new_idx = match direction {
          Adjustment::Increase => {
            if current_idx < ratios.len() - 1 {
              current_idx + 1
            } else {
              current_idx
            }
          }
          Adjustment::Decrease => {
            if current_idx > 0 {
              current_idx - 1
            } else {
              current_idx
            }
          }
        };

        let new_ratio = (1, ratios[new_idx]);

        let mut ratio = self.temp_ratio.lock().unwrap();
        *ratio = new_ratio;
        drop(ratio);

        self
          .marker_tx_cloned
          .send(marker::Message::SetRatio(new_ratio))
          .unwrap();

        Ok(None)
      }
      Command::ToggleReverse => {
        self
          .marker_tx_cloned
          .send(marker::Message::ToggleReverseMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleArpeggiator => {
        self
          .marker_tx_cloned
          .send(marker::Message::ToggleArpeggiatorMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleAccumulation => {
        self
          .marker_tx_cloned
          .send(marker::Message::ToggleAccumulationMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleRandom => {
        self
          .marker_tx_cloned
          .send(marker::Message::ToggleRandomMode())
          .unwrap();
        Ok(None)
      }
    }
  }

  fn handle_callbacks(&self, s: &mut Cursive, cmd: &Command) -> Result<Option<String>, String> {
    self.handle_default_commands(s, cmd)
  }

  pub fn handle(&self, siv: &mut Cursive, cmd: Command) {
    let _result = self.handle_callbacks(siv, &cmd);
    siv.on_event(Event::Refresh);
  }

  pub fn register_keybinding<E: Into<cursive::event::Event>>(
    &self,
    cursive: &mut Cursive,
    event: E,
    commands: Vec<Command>,
  ) {
    cursive.add_global_callback(event, move |s| {
      let cb_sink = s.cb_sink().clone();
      let cmd_cloned = commands.clone();
      cb_sink
        .send(Box::new(move |inner_s: &mut Cursive| {
          if let Some(data) = inner_s.user_data::<UserData>().cloned() {
            for command in cmd_cloned.into_iter() {
              data.cmd.handle(inner_s, command);
            }
          };
        }))
        .unwrap();
    });
  }

  pub fn unregister_keybindings(&self, cursive: &mut Cursive) {
    let kb = self.bindings.borrow();

    for (k, _v) in kb.iter() {
      if let Some(binding) = Self::parse_keybinding(k) {
        cursive.clear_global_callbacks(binding);
      }
    }
  }

  pub fn register_keybindings(&self, cursive: &mut Cursive) {
    let kb = self.bindings.borrow();

    for (k, v) in kb.iter() {
      if let Some(binding) = Self::parse_keybinding(k) {
        self.register_keybinding(cursive, binding, v.clone());
      } else {
        error!("Could not parse keybinding: \"{}\"", k);
      }
    }
  }

  fn default_keybindings() -> HashMap<String, Vec<Command>> {
    let mut kb = HashMap::new();

    kb.insert("q".into(), vec![Command::Quit]);
    kb.insert("Space".into(), vec![Command::TogglePlay]);
    kb.insert("Ctrl+h".into(), vec![Command::ShowMenubar]);
    kb.insert("Esc".into(), vec![Command::ToggleInputRegexAndCanvas]);
    kb.insert(
      "Shift+D".into(),
      vec![Command::AdjustMarker(MoveDirection::Right)],
    );
    kb.insert(
      "Shift+A".into(),
      vec![Command::AdjustMarker(MoveDirection::Left)],
    );
    kb.insert(
      "Shift+W".into(),
      vec![Command::AdjustMarker(MoveDirection::Up)],
    );
    kb.insert(
      "Shift+S".into(),
      vec![Command::AdjustMarker(MoveDirection::Down)],
    );
    kb.insert(">".into(), vec![Command::AdjustBPM(Adjustment::Increase)]);
    kb.insert("<".into(), vec![Command::AdjustBPM(Adjustment::Decrease)]);
    kb.insert("}".into(), vec![Command::AdjustRatio(Adjustment::Increase)]);
    kb.insert("{".into(), vec![Command::AdjustRatio(Adjustment::Decrease)]);
    kb.insert("Ctrl+r".into(), vec![Command::ToggleReverse]);
    kb.insert("Ctrl+a".into(), vec![Command::ToggleArpeggiator]);
    kb.insert("Ctrl+u".into(), vec![Command::ToggleAccumulation]);
    kb.insert("Ctrl+d".into(), vec![Command::ToggleRandom]);

    kb
  }

  fn parse_key(key: &str) -> Event {
    match key {
      "Enter" => Event::Key(Key::Enter),
      "Space" => Event::Char(" ".chars().next().unwrap()),
      "Tab" => Event::Key(Key::Tab),
      "Backspace" => Event::Key(Key::Backspace),
      "Esc" => Event::Key(Key::Esc),
      "Left" => Event::Key(Key::Left),
      "Right" => Event::Key(Key::Right),
      "Up" => Event::Key(Key::Up),
      "Down" => Event::Key(Key::Down),
      "Ins" => Event::Key(Key::Ins),
      "Del" => Event::Key(Key::Del),
      "Home" => Event::Key(Key::Home),
      "End" => Event::Key(Key::End),
      "PageUp" => Event::Key(Key::PageUp),
      "PageDown" => Event::Key(Key::PageDown),
      "PauseBreak" => Event::Key(Key::PauseBreak),
      "NumpadCenter" => Event::Key(Key::NumpadCenter),
      "F0" => Event::Key(Key::F0),
      "F1" => Event::Key(Key::F1),
      "F2" => Event::Key(Key::F2),
      "F3" => Event::Key(Key::F3),
      "F4" => Event::Key(Key::F4),
      "F5" => Event::Key(Key::F5),
      "F6" => Event::Key(Key::F6),
      "F7" => Event::Key(Key::F7),
      "F8" => Event::Key(Key::F8),
      "F9" => Event::Key(Key::F9),
      "F10" => Event::Key(Key::F10),
      "F11" => Event::Key(Key::F11),
      "F12" => Event::Key(Key::F12),
      s => Event::Char(s.chars().next().unwrap()),
    }
  }

  fn parse_keybinding(kb: &str) -> Option<cursive::event::Event> {
    let mut split = kb.split('+');
    if kb != "+" && split.clone().count() == 2 {
      let modifier = split.next().unwrap();
      let key = split.next().unwrap();
      let parsed = Self::parse_key(key);
      if let Event::Key(parsed) = parsed {
        match modifier {
          "Shift" => Some(Event::Shift(parsed)),
          "Alt" => Some(Event::Alt(parsed)),
          "Ctrl" => Some(Event::Ctrl(parsed)),
          _ => None,
        }
      } else if let Event::Char(parsed) = parsed {
        match modifier {
          "Shift" => Some(Event::Char(parsed.to_uppercase().next().unwrap())),
          "Alt" => Some(Event::AltChar(parsed)),
          "Ctrl" => Some(Event::CtrlChar(parsed)),
          _ => None,
        }
      } else {
        None
      }
    } else {
      Some(Self::parse_key(kb))
    }
  }
}
