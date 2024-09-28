use std::collections::HashMap;

use super::application::UserData;
use super::command::{parse, Command};

use cursive::event::{Event, Key};
use cursive::traits::View;
// use cursive::views::Dialog;
use cursive::Cursive;
use log::{debug, error, info};
// use ncspot::CONFIGURATION_FILE_NAME;
use std::cell::RefCell;

pub struct Config {
  /// The configuration file path.
  filename: String,
  // / Configuration set by the user, read only.
  // values: RwLock<ConfigValues>,
  // / Runtime state which can't be edited by the user, read/write.
  // state: RwLock<UserState>,
}

pub enum CommandResult {
  Consumed(Option<String>),
  // View(Box<dyn ViewExt>),
  Modal(Box<dyn View>),
  Ignored,
}

pub struct CommandManager {
  aliases: HashMap<String, String>,
  bindings: RefCell<HashMap<String, Vec<Command>>>,
  // spotify: Spotify,
  // queue: Arc<Queue>,
  // library: Arc<Library>,
  // config: Arc<Config>,
  // events: EventManager,
}

impl CommandManager {
  pub fn new(// spotify: Spotify,
    // queue: Arc<Queue>,
    // library: Arc<Library>,
    // config: Arc<Config>,
    // events: EventManager,
  ) -> Self {
    let bindings = RefCell::new(Self::get_bindings());
    Self {
      aliases: HashMap::new(),
      bindings,
      // spotify,
      // queue,
      // library,
      // config,
      // events,
    }
  }

  pub fn get_bindings() -> HashMap<String, Vec<Command>> {
    // let config = config.values();
    // let mut kb = if config.default_keybindings.unwrap_or(true) {
    let mut kb = Self::default_keybindings();
    // } else {
    //   HashMap::new()
    // };
    // let custom_bindings: Option<HashMap<String, String>> = config.keybindings.clone();

    // for (key, commands) in custom_bindings.unwrap_or_default() {
    // match parse(&commands) {
    //   Ok(cmds) => {
    //     info!("Custom keybinding: {} -> {:?}", key, cmds);
    //     kb.insert(key, cmds);
    //   }
    //   Err(err) => {
    //     error!(
    //       "Invalid command(s) for key {}-\"{}\": {}",
    //       key, commands, err
    //     );
    //   }
    // }
    // }

    kb
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
    self.register_aliases("repeat", vec!["loop"]);
  }

  fn handle_default_commands(
    &self,
    s: &mut Cursive,
    cmd: &Command,
  ) -> Result<Option<String>, String> {
    match cmd {
      Command::Quit => Ok(None),
    }
  }

  // fn handle_callbacks(&self, s: &mut Cursive, cmd: &Command) -> Result<Option<String>, String> {
  //   let local = if let Some(mut contextmenu) = s.find_name::<ContextMenu>("contextmenu") {
  //     contextmenu.on_command(s, cmd)?
  //   } else if let Some(mut add_track_menu) = s.find_name::<AddToPlaylistMenu>("addtrackmenu") {
  //     add_track_menu.on_command(s, cmd)?
  //   } else if let Some(mut select_artist) = s.find_name::<SelectArtistMenu>("selectartist") {
  //     select_artist.on_command(s, cmd)?
  //   } else if let Some(mut select_artist_action) =
  //     s.find_name::<SelectArtistActionMenu>("selectartistaction")
  //   {
  //     select_artist_action.on_command(s, cmd)?
  //   } else {
  //     s.on_layout(|siv, mut l| l.on_command(siv, cmd))?
  //   };

  //   if let CommandResult::Consumed(output) = local {
  //     Ok(output)
  //   } else if let CommandResult::Modal(modal) = local {
  //     s.add_layer(modal);
  //     Ok(None)
  //   } else if let CommandResult::View(view) = local {
  //     s.call_on_name("main", move |v: &mut Layout| {
  //       v.push_view(view);
  //     });

  //     Ok(None)
  //   } else {
  //     self.handle_default_commands(s, cmd)
  //   }
  // }

  // pub fn handle(&self, s: &mut Cursive, cmd: Command) {
  //   let result = self.handle_callbacks(s, &cmd);

  //   s.call_on_name("main", |v: &mut Layout| {
  //     v.set_result(result);
  //   });

  //   s.on_event(Event::Refresh);
  // }

  pub fn register_keybinding<E: Into<cursive::event::Event>>(
    &self,
    cursive: &mut Cursive,
    event: E,
    commands: Vec<Command>,
  ) {
    cursive.add_global_callback(event, move |s| {
      if let Some(data) = s.user_data::<UserData>().cloned() {
        for command in commands.clone().into_iter() {
          // data.cmd.handle(s, command);
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
