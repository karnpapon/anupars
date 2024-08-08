mod pkg;

use crate::pkg::model::{self};
use cursive::traits::Nameable;
use cursive::{
  views::{Button, Dialog, LinearLayout, Panel, SelectView},
  Cursive, Vec2,
};
use pkg::canvas::Canvas;
use pkg::view::BoardView;

use cursive::style::{BorderStyle, Palette};
use cursive::{event::Key, menu, traits::*};
use std::sync::atomic::{AtomicUsize, Ordering};

fn main() {
  let mut siv = cursive::default();

  // Start with a nicer theme than default
  siv.set_theme(cursive::theme::Theme {
    shadow: false,
    borders: BorderStyle::None,
    palette: Palette::retro().with(|palette| {
      use cursive::style::BaseColor::*;

      {
        // First, override some colors from the base palette.
        use cursive::style::Color::TerminalDefault;
        use cursive::style::PaletteColor::*;

        palette[Background] = TerminalDefault;
        palette[View] = TerminalDefault;
        palette[Primary] = White.dark();
        palette[TitlePrimary] = Blue.dark();
        palette[Secondary] = Blue.light();
        palette[Highlight] = Blue.dark();
      }

      {
        // Then override some styles.
        use cursive::style::Effect::*;
        use cursive::style::PaletteStyle::*;
        use cursive::style::Style;
        // palette[Highlight] = Style::from(Green.light()).combine(Bold);
        // palette[EditableTextCursor] = Style::secondary().combine(Reverse).combine(Underline)
      }
    }),
  });

  // We'll use a counter to name new files.
  let counter = AtomicUsize::new(1);

  // The menubar is a list of (label, menu tree) pairs.
  siv
    .menubar()
    // We add a new "File" tree
    .add_subtree(
      "File",
      menu::Tree::new()
        // Trees are made of leaves, with are directly actionable...
        .leaf("New", move |s| {
          // Here we use the counter to add an entry
          // in the list of "Recent" items.
          let i = counter.fetch_add(1, Ordering::Relaxed);
          let filename = format!("New {i}");
          s.menubar()
            .find_subtree("File")
            .unwrap()
            .find_subtree("Recent")
            .unwrap()
            .insert_leaf(0, filename, |_| ());

          // s.add_layer(Dialog::info("New file!"));
        })
        // ... and of sub-trees, which open up when selected.
        .subtree(
          "Recent",
          // The `.with()` method can help when running loops
          // within builder patterns.
          menu::Tree::new().with(|tree| {
            for i in 1..100 {
              // We don't actually do anything here,
              // but you could!
              tree.add_item(menu::Item::leaf(format!("Item {i}"), |_| ()).with(|s| {
                if i % 5 == 0 {
                  s.disable();
                }
              }))
            }
          }),
        )
        // Delimiter are simple lines between items,
        // and cannot be selected.
        .delimiter()
        .with(|tree| {
          for i in 1..10 {
            tree.add_leaf(format!("Option {i}"), |_| ());
          }
        }),
    )
    .add_subtree(
      "Help",
      menu::Tree::new()
        .subtree(
          "Help",
          menu::Tree::new()
            .leaf("General", |s| s.add_layer(Dialog::info("Help message!")))
            .leaf("Online", |s| {
              let text = "Google it yourself!\n\
                                        Kids, these days...";
              s.add_layer(Dialog::info(text))
            }),
        )
        .leaf("Start", |s| {
          s.pop_layer();
          new_game(s);
        })
        .leaf("About", |s| s.add_layer(Dialog::info("Cursive v0.0.0"))),
    )
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  // When `autohide` is on (default), the menu only appears when active.
  // Turning it off will leave the menu always visible.
  // Try uncommenting this line!

  // siv.set_autohide_menu(false);

  siv.add_global_callback(Key::Esc, |s| s.select_menubar());

  // siv.add_layer(Dialog::text("Hit <Esc> to show the menu!"));

  new_game(&mut siv);

  siv.run();
}

// fn main() {
// let mut siv = cursive::default();

// siv.add_layer(
//   Dialog::new()
//     .title("Minesweeper")
//     .padding_lrtb(2, 2, 1, 1)
//     .content(
//       LinearLayout::vertical()
//         .child(Button::new_raw("   New game  ", show_options))
//         .child(Button::new_raw("   Controls  ", show_controls))
//         .child(Button::new_raw("    Scores   ", show_scores))
//         .child(Button::new_raw("     Exit    ", |s| s.quit())),
//     ),
// );

// siv.run();
// }

// fn show_options(siv: &mut Cursive) {
//   siv.add_layer(
//     Dialog::new()
//       .title("Select difficulty")
//       .content(
//         SelectView::new()
//           .item(
//             "Easy:      8x8,   10 mines",
//             model::Options {
//               size: Vec2::new(8, 8),
//               mines: 10,
//             },
//           )
//           .item(
//             "Medium:    16x16, 40 mines",
//             model::Options {
//               size: Vec2::new(16, 16),
//               mines: 40,
//             },
//           )
//           .item(
//             "Difficult: 24x24, 99 mines",
//             model::Options {
//               size: Vec2::new(24, 24),
//               mines: 99,
//             },
//           )
//           .on_submit(|s, option| {
//             s.pop_layer();
//             new_game(s, *option);
//           }),
//       )
//       .dismiss_button("Back"),
//   );
// }

// fn show_controls(s: &mut Cursive) {
//   s.add_layer(
//     Dialog::info(
//       "Controls:
// Reveal cell:                  left click
// Mark as mine:                 right-click
// Reveal nearby unmarked cells: middle-click",
//     )
//     .title("Controls"),
//   )
// }

// fn show_scores(s: &mut Cursive) {
//   s.add_layer(Dialog::info("Not yet!").title("Scores"))
// }

fn new_game(siv: &mut Cursive) {
  let dialog = Canvas::new().full_width().full_height();

  siv.add_layer(dialog);
}
