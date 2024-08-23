use std::{
  error::Error,
  ffi::OsString,
  fs::{self, File},
  io::{self, Read},
  path::{Path, PathBuf},
};

use cursive::{
  align::HAlign,
  event::{Event, Key},
  menu::{self, Tree},
  view::{Margins, Nameable, Resizable},
  views::{
    Canvas, Dialog, DummyView, HideableView, LinearLayout, NamedView, OnEventView, ResizedView,
    SelectView, TextView,
  },
  Cursive, Printer, View, With,
};

use super::canvas_editor::{self, CanvasEditor};
use crate::core::{config, utils};

#[derive(Clone, Copy)]
pub struct Menubar {
  pub show_file_explorer: bool,
  pub show_doc: bool,
}

impl Default for Menubar {
  fn default() -> Self {
    Self::new()
  }
}

impl View for Menubar {
  fn draw(&self, _: &Printer) {}
}

impl Menubar {
  pub fn new() -> Self {
    Self {
      show_file_explorer: false,
      show_doc: false,
    }
  }

  // pub fn toggle_show_doc(&mut self) -> bool {
  //   self.show_doc = !self.show_doc;
  //   self.show_doc
  // }

  // pub fn toggle_show_file_explorer(&mut self) -> bool {
  //   self.show_file_explorer = !self.show_file_explorer;
  //   self.show_file_explorer
  // }

  // pub fn reset_toggle(&mut self) {
  //   self.show_file_explorer = false;
  //   self.show_doc = false;
  // }

  pub fn build_doc_view() -> NamedView<HideableView<OnEventView<Dialog>>> {
    HideableView::new(
      OnEventView::new(Dialog::text(format!(
        "{}\n\n{}",
        "DOCUMENTATION",
        utils::build_doc_string(&config::APP_DOCS)
      )))
      .on_event(Event::Key(Key::Esc), |s| {
        // Menubar::show_doc_view(s, false);
        s.pop_layer();
      }),
    )
    .with_name(config::doc_unit_view)
  }

  pub fn build_file_explorer_view() -> NamedView<HideableView<OnEventView<ResizedView<Dialog>>>> {
    HideableView::new(Self::dialog_file_explorer()).with_name(config::file_explorer_unit_view)
  }

  fn dialog_file_explorer() -> OnEventView<ResizedView<Dialog>> {
    let default_path = get_default_database_path();
    let paths = fs::read_dir(default_path.unwrap())
      .unwrap()
      .map(|res| res.map(|e| e.file_name().into_string()))
      .collect::<Vec<_>>();

    OnEventView::new(Dialog::around(listed_files_view(paths)).max_width(200)).on_event(
      Event::Key(Key::Esc),
      |s| {
        // Menubar::show_file_explorer_view(s, false)
        s.pop_layer();
      },
    )
  }

  // pub fn show_doc_view(siv: &mut Cursive, show: bool) {
  //   siv
  //     .find_name::<HideableView<OnEventView<Dialog>>>("doc_view")
  //     .unwrap()
  //     .set_visible(show);
  // }

  // pub fn show_file_explorer_view(siv: &mut Cursive, show: bool) {
  //   siv
  //     .find_name::<HideableView<OnEventView<ResizedView<Dialog>>>>("file_explorer_view")
  //     .unwrap()
  //     .set_visible(show);
  // }

  pub fn build_menu_app() -> Tree {
    menu::Tree::new()
      .leaf("Insert File [Ctr+o]", |s| {
        s.add_layer(Self::build_file_explorer_view())
      })
      .delimiter()
      .subtree(
        "MIDI",
        menu::Tree::new().with(|tree| {
          for (i, (midi, _)) in config::MENU_MIDI.iter().enumerate() {
            tree.add_item(menu::Item::leaf(format!("{i}: {midi}"), |_| ()))
          }
        }),
      )
      .subtree(
        "OSC",
        menu::Tree::new().with(|tree| {
          for (osc, port) in config::MENU_OSC.iter() {
            tree.add_item(menu::Item::leaf(format!("{osc}: {port}"), |_| ()))
          }
        }),
      )
      .delimiter()
      .leaf("Reset", move |s| s.reset_default_callbacks())
      .delimiter()
      .leaf("About", move |s| {
        s.add_layer(
          Dialog::info(format!(
            "{}\n{}\n\nauthor: {}\nversion: {}",
            config::APP_NAME,
            env!("CARGO_PKG_DESCRIPTION"),
            env!("CARGO_PKG_AUTHORS"),
            env!("CARGO_PKG_VERSION"),
          ))
          .padding(Margins::lrtb(2, 2, 0, 0))
          .max_width(50),
        );
      })
  }

  pub fn build_menu_help() -> Tree {
    // menu::Tree::new().leaf("Docs [h]", |s| Self::show_doc_view(s, true))
    menu::Tree::new().leaf("Docs [h]", |s| s.add_layer(Self::build_doc_view()))
  }
}

pub fn set_preview_contents(siv: &mut Cursive, file: &PathBuf) {
  let mut text_view = siv
    .find_name::<TextView>(config::file_contents_unit_view)
    .unwrap();
  if let Ok(contents) = read_file(Path::new(file)) {
    text_view.set_content(contents);
  }
}

fn set_selected_contents(siv: &mut Cursive, file: &PathBuf) {
  if let Ok(contents) = read_file(Path::new(file)) {
    if let Some(mut view) =
      siv.find_name::<Canvas<CanvasEditor>>(config::canvas_editor_section_view)
    {
      view.state_mut().update_grid_src(&contents);
      view.set_draw(canvas_editor::draw);
    }
  }
}

pub fn listed_files_view(dir: Vec<Result<Result<String, OsString>, io::Error>>) -> LinearLayout {
  let mut panes = LinearLayout::horizontal();

  if dir.is_empty() {
    let empty_dialog = Dialog::info(config::app_empty_dir()).fixed_size((50, 10));
    panes.add_child(empty_dialog);
    return panes;
  }

  let mut select = SelectView::new().h_align(HAlign::Left);
  let mut first_file_path = PathBuf::new();

  for (i, list) in dir.iter().enumerate() {
    let list_cloned = list.as_ref().unwrap().clone();
    let title_str = list_cloned.as_ref().unwrap().clone();
    let select_value = dirs::home_dir()
      .map(|p| {
        p.join(config::DEFAULT_APP_DIRECTORY)
          .join(config::DEFAULT_APP_FILENAME)
          .join(list_cloned.unwrap())
      })
      .unwrap();
    if i == 0 {
      first_file_path = select_value.clone();
    }
    select.add_item(title_str, select_value);
  }

  let init_file_details =
    read_file(first_file_path.as_path()).unwrap_or("empty content".to_string());

  let file_contents_unit_view = TextView::new(init_file_details)
    .with_name(config::file_contents_unit_view)
    .fixed_size((50, 15));

  let padding_view = DummyView::new().fixed_width(2);

  panes.add_child(
    select
      .on_select(set_preview_contents)
      .on_submit(set_selected_contents),
  );
  panes.add_child(padding_view);
  panes.add_child(file_contents_unit_view);
  panes
}

fn read_file(path: &Path) -> Result<String, Box<dyn Error>> {
  let mut file = match File::open(path) {
    Err(why) => panic!("couldn't open: {}", why),
    Ok(file) => file,
  };

  let mut s = String::new();
  let file_contents = match file.read_to_string(&mut s) {
    Err(why) => panic!("couldn't read :{}", why),
    Ok(_) => s,
  };

  Ok(file_contents)
}

/// Return the path to the default location (~/.anu_rs/contents)
fn get_default_database_path() -> Result<PathBuf, Box<dyn Error>> {
  let mut path = match dirs::home_dir().map(|p| p.join(config::DEFAULT_APP_DIRECTORY)) {
    Some(d) => d,
    None => return Err("invalid filename".into()),
  };
  if !path.is_dir() {
    fs::create_dir_all(&path)?;
  }
  path.push(config::DEFAULT_APP_FILENAME);
  Ok(path)
}
