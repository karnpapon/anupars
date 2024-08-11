mod core;

use core::anu::Anu;
// use std::sync::Mutex;

use cursive::theme::{BorderStyle, Palette};
use cursive::view::{Nameable, Selector};
use cursive::CursiveExt;
use cursive::{Cursive, With};

fn init_with_default_style(siv: &mut Cursive) {
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

  let mut anu = Anu::new().with_name("anu");

  anu.get_mut().init(&mut siv);
  init_with_default_style(&mut siv);
  siv.add_layer(anu);
  siv.focus(&Selector::Name("anu")).unwrap();
  // siv.set_autorefresh(true);
  // siv.set_fps(20);
  // let siv: Mutex<Cursive> = Mutex::new(siv);
  siv.run();
}
