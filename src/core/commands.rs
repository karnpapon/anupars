use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;

use super::anu::Anu;
use super::application::UserData;
use super::clock::metronome::Message;
use super::command::Command;
use super::{config, utils};

use cursive::event::{Event, Key};
// use cursive::traits::View;
use cursive::views::{LinearLayout, TextView};
use cursive::Cursive;
use log::error;
use std::cell::RefCell;

// pub struct Config {
//   filename: String,
// }

// pub enum CommandResult {
//   Consumed(Option<String>),
//   // View(Box<dyn ViewExt>),
//   Modal(Box<dyn View>),
//   Ignored,
// }

pub struct CommandManager {
  aliases: HashMap<String, String>,
  bindings: RefCell<HashMap<String, Vec<Command>>>,
  anu: Arc<Anu>,
  metronome_sender: Sender<Message>,
}

impl CommandManager {
  pub fn new(anu: Anu, m_tx: Sender<Message>) -> Self {
    let bindings = RefCell::new(Self::get_bindings());
    Self {
      aliases: HashMap::new(),
      bindings,
      anu: Arc::new(anu),
      metronome_sender: m_tx,
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
          let mut regex_display_unit_view = s
            .find_name::<TextView>(config::regex_display_unit_view)
            .unwrap();
          regex_display_unit_view.set_content(utils::build_doc_string(&config::APP_WELCOME_MSG));

          let mut interactive_display_section_view = s
            .find_name::<LinearLayout>(config::main_section_view)
            .unwrap();
          let _ = interactive_display_section_view.set_focus_index(3);
        } else {
          let mut interactive_display_section_view = s
            .find_name::<LinearLayout>(config::main_section_view)
            .unwrap();
          let _ = interactive_display_section_view.set_focus_index(0);
        }

        Ok(None)
      }
    }
  }

  fn handle_callbacks(&self, s: &mut Cursive, cmd: &Command) -> Result<Option<String>, String> {
    self.handle_default_commands(s, cmd)
  }

  pub fn handle(&self, siv: &mut Cursive, cmd: Command) {
    let _result = self.handle_callbacks(siv, &cmd);

    // s.call_on_name("main", |v: &mut Layout| {
    //   v.set_result(result);
    // });

    siv.on_event(Event::Refresh);
  }

  pub fn register_keybinding<E: Into<cursive::event::Event>>(
    &self,
    cursive: &mut Cursive,
    event: E,
    commands: Vec<Command>,
  ) {
    cursive.add_global_callback(event, move |s| {
      if let Some(data) = s.user_data::<UserData>().cloned() {
        for command in commands.clone().into_iter() {
          data.cmd.handle(s, command);
        }
      }
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
