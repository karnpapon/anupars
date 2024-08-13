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
  menu,
  view::{Margins, Nameable, Resizable},
  views::{Dialog, DummyView, LinearLayout, OnEventView, SelectView, TextView},
  Cursive, Printer, View, With,
};

use super::{config, utils};

pub struct Menubar {
  show_doc_view: bool,
}

impl Default for Menubar {
  fn default() -> Self {
    Self::new()
  }
}

impl Menubar {
  pub fn new() -> Self {
    Self {
      show_doc_view: false,
    }
  }

  pub fn add_doc_view(siv: &mut Cursive) {
    siv.add_layer(
      OnEventView::new(Dialog::info(format!(
        "{}\n\n{}",
        "DOCUMENTATION",
        utils::build_doc_string(&config::APP_DOCS)
      )))
      .on_event(Event::Key(Key::Esc), |s| {
        s.pop_layer();
      }),
    )
  }

  pub fn init(&mut self, siv: &mut Cursive) {
    let menu_app = menu::Tree::new()
      .leaf("Insert File", |s| {
        let default_path = get_default_database_path();
        let paths = fs::read_dir(default_path.unwrap())
          .unwrap()
          .map(|res| res.map(|e| e.file_name().into_string()))
          .collect::<Vec<_>>();

        let file_explorer = show_listed_files(paths);
        let dialog_file_exp = OnEventView::new(
          Dialog::around(file_explorer)
            .dismiss_button("close")
            .max_width(200),
        )
        .on_event(Event::Key(Key::Esc), |ss| {
          ss.pop_layer();
        });

        s.add_layer(dialog_file_exp)
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
      });

    let menu_help = menu::Tree::new().leaf("Docs", Menubar::add_doc_view);

    siv
      .menubar()
      .add_subtree("Anu", menu_app)
      .add_subtree("Help", menu_help)
      .add_delimiter()
      .add_leaf("Quit", |s| s.quit());

    siv.add_global_callback(Key::Esc, |s| s.select_menubar());
  }
}

impl View for Menubar {
  fn draw(&self, printer: &Printer) {}
}

fn load_contents(siv: &mut Cursive, file: &PathBuf) {
  let mut text_view = siv.find_name::<TextView>("file_contents").unwrap();
  if let Ok(contents) = read_file(Path::new(file)) {
    text_view.set_content(contents);
  }
}

fn show_listed_files(dir: Vec<Result<Result<String, OsString>, io::Error>>) -> LinearLayout {
  let mut panes = LinearLayout::horizontal();

  if dir.is_empty() {
    let empty_dialog = Dialog::info(config::app_empty_dir()).fixed_size((50, 10));
    panes.add_child(empty_dialog);
    return panes;
  }

  let mut select = SelectView::new().h_align(HAlign::Left);

  for list in dir {
    let list_cloned = list.unwrap().clone();
    let title_str = list_cloned.as_ref().unwrap().clone();
    let select_value = dirs::home_dir()
      .map(|p| {
        p.join(config::DEFAULT_APP_DIRECTORY)
          .join(config::DEFAULT_APP_FILENAME)
          .join(list_cloned.unwrap())
      })
      .unwrap();
    select.add_item(title_str, select_value);
  }
  let file_details = TextView::new("")
    .with_name("file_contents")
    .fixed_size((50, 15));

  let padding = DummyView::new().fixed_width(2);

  panes.add_child(select.on_select(load_contents));
  panes.add_child(padding);
  panes.add_child(file_details);
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
  // Create the directory if it doesn't already exist.
  if !path.is_dir() {
    fs::create_dir_all(&path)?;
  }
  path.push(config::DEFAULT_APP_FILENAME);
  Ok(path)
}
