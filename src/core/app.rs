use super::canvas::Canvas;
use super::menu::setup_app_menu;
use cursive::Cursive;

use cursive::style::{BorderStyle, Palette};
use cursive::traits::*;

pub fn init_app_with_default_style(siv: &mut Cursive) {
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

pub fn add_canvas(siv: &mut Cursive) {
  let canvas = Canvas::new().full_width().full_height();

  siv.add_layer(canvas);
}

pub fn start(siv: &mut Cursive) {
  init_app_with_default_style(siv);
  setup_app_menu(siv);
  add_canvas(siv);
}
