mod core;
mod view;

use core::anu::{Controller, ControllerData};
use core::{config, utils};
use view::canvas_editor::CanvasEditor;
use view::menubar::{self, Menubar};

use cursive::direction::{Absolute, Direction, Relative};
use cursive::event::{Event, Key};
use cursive::theme::{BorderStyle, ColorType, Palette};
use cursive::view::{Finder, Selector};
use cursive::views::{Canvas, Dialog, DialogFocus, EditView, LinearLayout, TextView};
use cursive::{Cursive, CursiveExt, Printer, View, With};

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
  let mut menubar = Menubar::new();
  let menu_app = Menubar::build_menu_app();
  let menu_help = Menubar::build_menu_help();
  // let mut doc_view = Menubar::build_doc_view();
  // let mut file_explorer_view = Menubar::build_file_explorer_view();
  let mut controller: Controller = Controller::new();

  // doc_view.get_mut().hide();
  // file_explorer_view.get_mut().hide();

  siv.set_autohide_menu(false);
  siv.set_autorefresh(true);
  siv.set_user_data(Menubar::default());
  siv.set_user_data(ControllerData::default());
  let mut current_data = siv
    .with_user_data(|controller_data: &mut ControllerData| controller_data.clone())
    .unwrap();

  init_default_style(&mut siv);

  let main_views = controller.build(&mut current_data);

  siv
    .menubar()
    .add_subtree("Anu", menu_app)
    .add_subtree("Help", menu_help)
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.add_layer(main_views);
  // siv.add_layer(doc_view);
  // siv.add_layer(file_explorer_view);

  // siv.add_global_callback(Event::Char('h'), move |s| {
  //   menubar::Menubar::show_doc_view(s, menubar.toggle_show_doc())
  // });
  // siv.add_global_callback(Event::CtrlChar('o'), move |s| {
  //   menubar::Menubar::show_file_explorer_view(s, menubar.toggle_show_file_explorer())
  // });
  // siv.add_global_callback(Key::F2, move |s| {
  //   if let Some(mut res) = s.find_name::<LinearLayout>("main_view") {
  //     match res.set_focus_index(2) {
  //       Ok(res) => println!("ok"),
  //       Err(e) => println!("error {:?}", e),
  //     }
  //     // res.process(s);
  //   };
  // });
  siv.add_global_callback(Key::Esc, move |s| {
    if !current_data.show_regex_display {
      let mut regex_display_unit_view = s
        .find_name::<TextView>(config::regex_display_unit_view)
        .unwrap();
      regex_display_unit_view
        .get_shared_content()
        .set_content(utils::build_doc_string(&config::APP_WELCOME_MSG));
    }

    // TODO: handle s.pop_layer() eg. when dismiss btn is clicked.
    // menubar::Menubar::show_doc_view(s, false);
    // menubar::Menubar::show_file_explorer_view(s, false);

    // menubar.reset_toggle();

    s.select_menubar();
  });

  siv.run();
}
