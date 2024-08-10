use super::config;
use cursive::align::HAlign;
use cursive::view::Margins;
use cursive::views::SelectView;
use cursive::{views::Dialog, Cursive};

use cursive::{event::Key, menu, traits::*};
use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File, ReadDir};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

pub fn setup_app_menu(siv: &mut Cursive) {
  // let counter = AtomicUsize::new(1);

  siv
    .menubar()
    .add_subtree(
      "Anu",
      menu::Tree::new()
        .leaf("Insert File", move |s| {
          let default_path = get_default_database_path();
          let paths = fs::read_dir(default_path.unwrap())
            .unwrap()
            .map(|res| res.map(|e| e.file_name().into_string()))
            .collect::<Vec<_>>();

          show_listed_files(s, paths);

          // if let Ok(contents) = read_file(Path::new("hello.txt")) {
          //   s.add_layer(TextView::new(contents));
          // }
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
        }),
    )
    .add_subtree(
      "Help",
      menu::Tree::new().leaf("Docs", |s| {
        let mut docs = String::new();
        for (command, desc) in config::APP_DOCS.iter() {
          docs.push_str(format!("{}: {}\n", command, desc).as_str());
        }
        s.add_layer(Dialog::info(format!("{}\n\n{}", "DOCUMENTATION", docs)))
      }),
    )
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.add_global_callback(Key::Esc, |s| s.select_menubar());
}

pub fn read_file(path: &Path) -> Result<String, Box<dyn Error>> {
  let mut file = match File::open(&path) {
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

pub fn show_listed_files(s: &mut Cursive, dir: Vec<Result<Result<String, OsString>, io::Error>>) {
  if dir.is_empty() {
    s.add_layer(Dialog::info(config::app_empty_dir()).fixed_size((50, 10)));
    return;
  }

  let mut select = SelectView::new().h_align(HAlign::Center).autojump();

  for list in dir {
    let list_cloned = list.unwrap().clone();
    let title_str = list_cloned.as_ref().unwrap().clone();
    select.add_item(title_str, list_cloned.unwrap());
  }
  // select.set_on_submit(render_downloading_song);
  s.add_layer(Dialog::around(select.scrollable().fixed_size((50, 10))).dismiss_button("close"));
}
