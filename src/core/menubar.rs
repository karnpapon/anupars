use std::{
  error::Error,
  ffi::OsString,
  fs::{self, File},
  io::{self, Read},
  path::{Path, PathBuf},
};

use cursive::{
  align::HAlign,
  event::{Callback, Event, EventResult, Key},
  menu,
  theme::{BaseColor, Color, ColorStyle},
  view::{Margins, Resizable, Scrollable},
  views::{Dialog, OnEventView, SelectView},
  Cursive, Printer, Vec2, View, With,
};

use super::config;

pub struct Menubar {}

impl Default for Menubar {
  fn default() -> Self {
    Self::new()
  }
}

impl Menubar {
  pub fn new() -> Self {
    Self {}
  }

  pub fn init(&mut self, siv: &mut Cursive) {
    siv
      .menubar()
      .add_subtree(
        "Anu",
        menu::Tree::new()
          .leaf("Insert File", |s| {
            let default_path = get_default_database_path();
            let paths = fs::read_dir(default_path.unwrap())
              .unwrap()
              .map(|res| res.map(|e| e.file_name().into_string()))
              .collect::<Vec<_>>();

            show_listed_files(s, paths);
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
          s.add_layer(
            OnEventView::new(Dialog::info(format!("{}\n\n{}", "DOCUMENTATION", docs))).on_event(
              Event::Key(Key::Esc),
              |s| {
                s.pop_layer();
              },
            ),
          )
        }),
      )
      .add_delimiter()
      .add_leaf("Quit", |s| s.quit());

    siv.add_global_callback(Key::Esc, |s| s.select_menubar());
  }
}

impl View for Menubar {
  fn draw(&self, printer: &Printer) {
    // for y in 0..printer.size.y {
    //   for x in 0..printer.size.x {
    //     printer.with_color(
    //       ColorStyle::new(Color::Dark(BaseColor::Blue), Color::Dark(BaseColor::Blue)),
    //       |printer| {
    //         printer.print((x, y), " ");
    //       },
    //     );
    //   }
    // }
    // printer.with_color(
    //   ColorStyle::new(Color::Dark(BaseColor::White), Color::Dark(BaseColor::Blue)),
    //   |printer| {
    //     printer.print((10, 2), "paused, press m to resume");
    //   },
    // );
  }

  // fn required_size(&mut self, _constraint: cursive::Vec2) -> cursive::Vec2 {
  //   Vec2::new(45, 5)
  // }

  // fn on_event(&mut self, event: Event) -> EventResult {
  //   if event != Event::Char('m') && event != Event::Char('M') {
  //     EventResult::Ignored
  //   } else {
  //     EventResult::Consumed(Some(Callback::from_fn(move |s| {
  //       s.pop_layer();
  //       s.call_on_name("tetris", |t: &mut Tetris| t.on_event(Event::Char('m')));
  //     })))
  //   }
  // }
}

fn show_listed_files(s: &mut Cursive, dir: Vec<Result<Result<String, OsString>, io::Error>>) {
  if dir.is_empty() {
    s.add_layer(Dialog::info(config::app_empty_dir()).fixed_size((50, 10)));
    return;
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

  select.set_on_submit(render_texts);
  s.add_layer(
    OnEventView::new(
      Dialog::around(select.scrollable().fixed_size((50, 10))).dismiss_button("close"),
    )
    .on_event(Event::Key(Key::Esc), |ss| {
      ss.pop_layer();
    }),
  );
}

fn render_texts(s: &mut Cursive, file: &PathBuf) {
  if let Ok(contents) = read_file(Path::new(file)) {
    // let g = canvas::run(|siv| grid::Grid::update_grid_src(siv, &contents));
    // g(s);
  }
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
