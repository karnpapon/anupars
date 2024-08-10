use super::config;
use super::grid::Grid;

use std::error::Error;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc;

use cursive::align::HAlign;
use cursive::event::Event;
use cursive::style::{BorderStyle, Palette};
use cursive::traits::*;
use cursive::view::Margins;
use cursive::views::{Dialog, NamedView};
use cursive::views::{OnEventView, SelectView};
use cursive::{event::Key, menu};
use cursive::{Cursive, CursiveExt, Printer, Vec2};

#[derive(Debug)]
pub struct Account {
  pub name: String,
  pub user: String,
  pub password: String,
  pub url: String,
  pub notes: String,
}

#[derive(Debug)]
pub enum UiMessage {
  UpdateStatus,
  Refresh,
}

#[derive(Clone)]
pub struct Canvas {
  pub grid: Grid,
}

impl Canvas {
  pub fn new() -> Canvas {
    let g = Grid::new(0, 0);
    Canvas { grid: g }
  }

  pub fn update_grid_src(&mut self, src: &str) {
    self.grid.update_grid_src(src)
  }
}

impl cursive::view::View for Canvas {
  fn layout(&mut self, size: Vec2) {
    self.grid.resize(size)
  }

  fn draw(&self, printer: &Printer) {
    println!(">");
    for (x, row) in self.grid.grid.iter().enumerate() {
      for (y, &value) in row.iter().enumerate() {
        let display_value = if value != '\0' {
          value
        } else if x % self.grid.grid_row_spacing == 0 && y % self.grid.grid_col_spacing == 0 {
          '+'
        } else {
          '.'
        };

        printer.print((x, y), &display_value.to_string())
      }
    }
  }
}

pub struct Ui {
  cursive: Cursive,
  // canvas: ResizedView<ResizedView<Canvas>>,
  ui_rx: mpsc::Receiver<UiMessage>,
  ui_tx: mpsc::Sender<UiMessage>,
  controller_tx: mpsc::Sender<Message>,
}

#[derive(Debug)]
pub enum Message {
  Sync,
  Quit,
}

impl Ui {
  pub fn new(s: Cursive, controller_tx: mpsc::Sender<Message>) -> Ui {
    let (ui_tx, ui_rx) = mpsc::channel::<UiMessage>();
    // let canvas = Canvas::new().full_width().full_height();

    Ui {
      cursive: s,
      // canvas,
      ui_tx,
      ui_rx,
      controller_tx,
    }
  }

  pub fn init(&mut self) {
    self.init_keybinding();
    self.init_with_default_style();
    self.init_menu();
    self.init_canvas();
  }

  pub fn run(&mut self) -> bool {
    if !self.cursive.is_running() {
      return false;
    }

    self.cursive.run();

    while let Some(message) = self.next_ui_message() {
      match message {
        UiMessage::UpdateStatus => {}
        UiMessage::Refresh => {}
      }
    }
    true
  }

  pub fn init_with_default_style(&mut self) {
    self.cursive.set_theme(cursive::theme::Theme {
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

  fn init_keybinding(&mut self) {
    self.cursive.add_global_callback(Event::Char('z'), |s| {
      // s.call_on_name("canvas", |sc: &mut Canvas| {
      //   println!("ccccccccccccccc = {:?}", sc.grid.size);
      // })
      // .unwrap()
    });
  }

  pub fn init_canvas(&mut self) {
    let canvas = Canvas::new().with_name("canvas").full_width().full_height();
    self.cursive.add_layer(canvas);
  }

  pub fn init_menu(&mut self) {
    self
      .cursive
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

    self
      .cursive
      .add_global_callback(Key::Esc, |s| s.select_menubar());
  }

  pub fn next_ui_message(&self) -> Option<UiMessage> {
    self.ui_rx.try_iter().next()
  }

  pub fn quit(&mut self) {
    self.cursive.quit();
  }
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
    // s.add_layer(TextView::new(&contents));
    s.call_on_name("canvas", |cv: &mut Canvas| cv.update_grid_src(&contents));
  }

  s.set_autorefresh(true);
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
