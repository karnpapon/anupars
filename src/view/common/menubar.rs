use std::{
  error::Error,
  ffi::OsString,
  fs::{self, File},
  io::{self, Read},
  path::{Path, PathBuf},
  sync::mpsc::Sender,
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
  Cursive, With,
};

use super::canvas_editor::CanvasEditor;
use crate::core::{config, disspress, utils};

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

// impl View for Menubar {
//   fn draw(&self, _: &Printer) {}
// }

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

  pub fn build_menu_app(
    midi_devices: &[(String, usize)],
    midi_tx: Sender<crate::core::midi::Message>,
  ) -> Tree {
    let midi_tx_reset = midi_tx.clone();
    menu::Tree::new()
      .leaf("Generate Text", generate_contents)
      .leaf("Insert File", build_file_explorer_view)
      .delimiter()
      .subtree(
        "MIDI",
        build_midi_menu(midi_devices.to_vec(), midi_tx.clone()),
      )
      .subtree("OSC", build_osc_menu())
      .delimiter()
      .leaf("Reset", move |s| {
        s.reset_default_callbacks();
        // Clear MIDI config and stop all notes
        let _ = midi_tx_reset.send(crate::core::midi::Message::ClearMsgConfig());
        let _ = midi_tx_reset.send(crate::core::midi::Message::Panic());
      })
      .delimiter()
      .leaf("About", build_about_view)
  }

  pub fn build_menu_help() -> Tree {
    // menu::Tree::new().leaf("Docs [h]", |s| Self::show_doc_view(s, true))
    menu::Tree::new().leaf("Docs [h]", |s| s.add_layer(Self::build_doc_view()))
  }
}

// ------------------------------------------------------------

fn build_midi_menu(
  devices: Vec<(String, usize)>,
  midi_tx: Sender<crate::core::midi::Message>,
) -> cursive::menu::Tree {
  menu::Tree::new().with(|tree| {
    if devices.is_empty() {
      tree.add_item(menu::Item::leaf("No devices found", |_| ()));
    } else {
      for (name, idx) in devices {
        let midi_tx_clone = midi_tx.clone();
        let name_clone = name.clone();
        tree.add_item(menu::Item::leaf(format!("{}: {}", idx, name), move |s| {
          // Send message to switch MIDI device
          if let Err(e) = midi_tx_clone.send(crate::core::midi::Message::SwitchDevice(idx)) {
            s.add_layer(Dialog::info(format!("Failed to switch device: {}", e)));
          } else {
            // Update the MIDI status display
            s.call_on_name(config::midi_status_unit_view, |c: &mut TextView| {
              c.set_content(&name_clone);
            });
          }
        }));
      }
    }
  })
}
fn build_osc_menu() -> cursive::menu::Tree {
  menu::Tree::new().with(|tree| {
    for (osc, port) in config::MENU_OSC.iter() {
      tree.add_item(menu::Item::leaf(format!("{osc}: {port}"), |_| ()))
    }
  })
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

pub fn build_file_explorer_view(siv: &mut Cursive) {
  siv.add_layer(
    HideableView::new(dialog_file_explorer()).with_name(config::file_explorer_unit_view),
  );
}

fn build_about_view(siv: &mut Cursive) {
  siv.add_layer(
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
}

// ----------------------------------------------------------------

// generate random text based-on Dissociate Press algorithm:
// https://en.wikipedia.org/wiki/Dissociated_press
pub fn generate_contents(siv: &mut Cursive) {
  let contents = disspress::run();
  set_contents(siv, contents);
}

fn set_contents(siv: &mut Cursive, contents: String) {
  siv
    .call_on_name(
      config::canvas_editor_section_view,
      move |c: &mut Canvas<CanvasEditor>| {
        c.state_mut().clear_contents();
        c.state_mut().update_text_contents(&contents);
        c.state_mut().update_grid_src();
      },
    )
    .unwrap();
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
  siv.pop_layer();
  if let Ok(contents) = read_file(Path::new(file)) {
    set_contents(siv, contents);
  }
}

// ----------------------------------------------------------------

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
  path.push(config::DEFAULT_APP_FILENAME);
  if !path.is_dir() {
    fs::create_dir_all(&path)?;
  }
  Ok(path)
}
