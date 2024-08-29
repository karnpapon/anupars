mod core;
mod view;

use core::anu::Anu;
use core::clock::metronome;
use core::{config, utils};
use std::thread;
use view::canvas_editor::CanvasEditor;
use view::menubar::Menubar;

use cursive::event::{Event, Key};
use cursive::theme::{BorderStyle, Palette};
use cursive::views::{Canvas, TextView};
use cursive::{Cursive, CursiveExt, With};

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
  let mut anu: Anu = Anu::new();

  let cb_sink = siv.cb_sink().clone();

  siv.set_autohide_menu(false);
  siv.set_autorefresh(true);
  siv.set_user_data(Menubar::default());
  siv.set_user_data(Anu::default());
  let current_data = siv
    .with_user_data(|controller_data: &mut Anu| controller_data.clone())
    .unwrap();

  init_default_style(&mut siv);

  let main_views = anu.build();

  siv
    .menubar()
    .add_subtree("Anu", menu_app)
    .add_subtree("Help", menu_help)
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.add_layer(main_views);

  siv.add_global_callback(Key::Esc, move |s| {
    if !current_data.show_regex_display {
      let mut regex_display_unit_view = s
        .find_name::<TextView>(config::regex_display_unit_view)
        .unwrap();
      regex_display_unit_view
        .get_shared_content()
        .set_content(utils::build_doc_string(&config::APP_WELCOME_MSG));
    }

    s.select_menubar();
  });

  siv.add_global_callback(Event::Char(' '), move |s| {
    s.call_on_name(
      config::canvas_editor_section_view,
      |c: &mut Canvas<CanvasEditor>| {
        let is_playing = c.state_mut().marker_mut().is_playing;
        c.state_mut().set_playing(!is_playing)
      },
    )
    .unwrap();
  });

  thread::spawn(move || {
    let metronome = metronome::Metronome::new();
    metronome.run(cb_sink);
  });

  siv.run();
}
