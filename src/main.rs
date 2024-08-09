mod core;

use core::config::MENU_OSC;
use core::{canvas::Canvas, config::MENU_MIDI};
use cursive::{views::Dialog, Cursive};

use cursive::style::{BorderStyle, Palette};
use cursive::{event::Key, menu, traits::*};
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
  let mut siv = cursive::default();

  siv.set_theme(cursive::theme::Theme {
    shadow: false,
    borders: BorderStyle::None,
    palette: Palette::retro().with(|palette| {
      use cursive::style::Color::TerminalDefault;
      use cursive::style::PaletteColor::{
        Background, Highlight, Primary, Secondary, TitlePrimary, View,
      };

      palette[Background] = TerminalDefault;
      palette[View] = TerminalDefault;
      palette[Primary] = TerminalDefault;
      palette[TitlePrimary] = TerminalDefault;
      palette[Highlight] = TerminalDefault;
      palette[Secondary] = TerminalDefault;
    }),
  });

  let counter = AtomicUsize::new(1);

  siv
    .menubar()
    .add_subtree(
      "Anu",
      menu::Tree::new()
        .leaf("Insert File", move |s| {
          let i = counter.fetch_add(1, Ordering::Relaxed);
          let filename = format!("New {i}");
          s.menubar()
            .find_subtree("Insert File")
            .unwrap()
            .insert_leaf(0, filename, |_| ());
        })
        .delimiter()
        .subtree(
          "MIDI",
          menu::Tree::new().with(|tree| {
            for (i, (midi, _)) in MENU_MIDI.iter().enumerate() {
              tree.add_item(menu::Item::leaf(format!("{i}: {midi}"), |_| ()))
            }
          }),
        )
        .subtree(
          "OSC",
          menu::Tree::new().with(|tree| {
            for (osc, port) in MENU_OSC.iter() {
              tree.add_item(menu::Item::leaf(format!("{osc}: {port}"), |_| ()))
            }
          }),
        )
        .delimiter()
        .leaf("About", move |s| {}),
    )
    .add_subtree(
      "Help",
      menu::Tree::new()
        .leaf("Documentation", |s| {
          s.add_layer(Dialog::info("anu documentation"))
        })
        .leaf("About", |s| s.add_layer(Dialog::info("anu v0.1.0"))),
    )
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.set_autohide_menu(false);
  siv.add_global_callback(Key::Esc, |s| s.select_menubar());

  run_app(&mut siv);

  siv.run();
}

fn run_app(siv: &mut Cursive) {
  let dialog = Canvas::new().full_width().full_height();

  siv.add_layer(dialog);
}
