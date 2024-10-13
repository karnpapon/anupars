mod core;
mod view;

use core::anu::Anu;
use core::application::UserDataInner;
use core::clock::metronome::{Message, Metronome};
use core::commands::CommandManager;
use core::midi::Midi;
use core::regex::RegExpHandler;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cursive::theme::{BorderStyle, Palette};
use cursive::{Cursive, CursiveExt, With};
use num::rational::Ratio;
use num::FromPrimitive;
use std::rc::Rc;
use std::thread;
use view::menubar::Menubar;

pub fn init_default_style(siv: &mut Cursive) {
  siv.set_theme(cursive::theme::Theme {
    shadow: false,
    borders: BorderStyle::Simple,
    palette: Palette::retro().with(|palette| {
      use cursive::style::Color::TerminalDefault;
      use cursive::style::PaletteColor::{
        Background, Highlight, HighlightInactive, HighlightText, Primary, Secondary, Shadow,
        Tertiary, TitlePrimary, TitleSecondary, View,
      };

      palette[Background] = TerminalDefault;
      palette[View] = TerminalDefault;
      palette[Primary] = TerminalDefault;
      palette[TitlePrimary] = TerminalDefault;
      palette[Highlight] = TerminalDefault;
      palette[Secondary] = TerminalDefault;
      palette[HighlightInactive] = TerminalDefault;
      palette[HighlightText] = TerminalDefault;
      palette[Shadow] = TerminalDefault;
      palette[TitleSecondary] = TerminalDefault;
      palette[Tertiary] = TerminalDefault;
    }),
  });
}

fn main() {
  let mut siv: Cursive = Cursive::new();
  init_default_style(&mut siv);

  let menu_app = Menubar::build_menu_app();
  let menu_help = Menubar::build_menu_help();
  let mut midi = Midi::default();
  midi.init().unwrap();
  let midi_tx = midi.tx.clone();
  let regex = RegExpHandler::new(siv.cb_sink().clone());
  let mut anu: Anu = Anu::new();
  let metronome = Metronome::new(siv.cb_sink().clone());
  let m_tx = metronome.tx.clone();
  let m_tx_2 = metronome.tx.clone();
  let last_key_time = Arc::new(Mutex::new(None));
  let last_key_time_clone = Arc::clone(&last_key_time);
  let temp_tempo = Arc::new(Mutex::new(120));
  let temp_tempo_cloned = Arc::clone(&temp_tempo);
  let mut cmd_manager = CommandManager::new(
    anu.clone(),
    m_tx,
    siv.cb_sink().clone(),
    temp_tempo_cloned,
    last_key_time.clone(),
  );

  cmd_manager.register_all();
  cmd_manager.register_keybindings(&mut siv);

  siv.set_autohide_menu(true);
  siv.set_autorefresh(false); // "false" to prevent unintended events (eg. midi keep pushing into stack, etc.)
  siv.set_user_data(Rc::new(UserDataInner { cmd: cmd_manager }));

  let main_views = anu.build(regex.tx.clone(), midi_tx);

  siv
    .menubar()
    .add_subtree("Anu", menu_app)
    .add_subtree("Help", menu_help)
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.add_layer(main_views);

  thread::spawn(move || loop {
    thread::sleep(Duration::from_millis(100));
    let mut last_press = last_key_time_clone.lock().unwrap();
    if let Some(last_time) = *last_press {
      if last_time.elapsed() > Duration::from_millis(500) {
        *last_press = None;
        let tempo = *temp_tempo.lock().unwrap();
        let _ = m_tx_2.send(Message::Tempo(Ratio::from_i64(tempo).unwrap()));
      }
    }
  });
  thread::spawn(move || {
    regex.run();
  });
  thread::spawn(move || {
    metronome.run();
  });
  thread::spawn(move || {
    midi.run();
  });

  siv.run();
}
