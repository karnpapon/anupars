mod core;
mod view;

use core::anu::Anu;
use core::application::UserDataInner;
use core::clock::metronome::{self, Metronome};
use core::commands::CommandManager;
use core::regex::RegExpHandler;

use cursive::theme::{BorderStyle, Palette};
use cursive::{Cursive, CursiveExt, With};
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
  let menu_app = Menubar::build_menu_app();
  let menu_help = Menubar::build_menu_help();
  let regex = RegExpHandler::new(siv.cb_sink().clone());
  let mut anu: Anu = Anu::new();
  let metronome = Metronome::new(siv.cb_sink().clone());
  let m_tx = metronome.tx.clone();

  let mut cmd_manager = CommandManager::new(anu.clone(), m_tx);

  cmd_manager.register_all();
  cmd_manager.register_keybindings(&mut siv);

  siv.set_autohide_menu(true);
  siv.set_autorefresh(true);
  siv.set_user_data(Rc::new(UserDataInner { cmd: cmd_manager }));

  init_default_style(&mut siv);

  let main_views = anu.build(regex.tx.clone());

  siv
    .menubar()
    .add_subtree("Anu", menu_app)
    .add_subtree("Help", menu_help)
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.add_layer(main_views);

  thread::spawn(move || {
    regex.run();
  });
  thread::spawn(move || {
    metronome.run();
  });

  siv.run();
}
