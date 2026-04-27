use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use crate::app_state::{MenuId, MenuState};
use crate::core::{consts, utils};
use crate::view::grid::GridEditor;

/// Load a file's contents into the grid editor.
pub fn set_grid_contents(grid: &mut GridEditor, contents: String) {
  let contents = contents.replace("\r\n", "\n").replace("\r", "\n");
  let w = grid.grid.width;
  let h = grid.grid.height;
  let clipped = if w > 0 && h > 0 {
    let contents = utils::scale_to_width(&contents, w);
    let lines: Vec<&str> = contents.split('\n').collect();
    let total = lines.len();
    let start = if total > 1 {
      fastrand::usize(0..total)
    } else {
      0
    };
    lines
      .iter()
      .cycle()
      .skip(start)
      .take(h)
      .map(|line| line.chars().take(w).collect::<String>())
      .collect::<Vec<_>>()
      .join("\n")
  } else {
    contents
  };
  grid.clear_contents();
  grid.update_text_contents(&clipped);
  grid.update_grid_src();
}

/// Load a file at `path` into the grid.
pub fn load_file_into_grid(grid: &mut GridEditor, path: &Path) -> Result<(), Box<dyn Error>> {
  let contents = read_file(path)?;
  set_grid_contents(grid, contents);
  Ok(())
}

fn read_file(path: &Path) -> Result<String, Box<dyn Error>> {
  let mut file = File::open(path)?;
  let mut s = String::new();
  file.read_to_string(&mut s)?;
  Ok(s)
}

// --- ScreenBuffer rendering path ---

/// A single entry in a dropdown menu.
#[derive(Clone)]
enum Item {
  Action(String),
  Toggle(String, bool),
  Submenu(String),
  Delimiter,
}

impl Item {
  fn is_selectable(&self) -> bool {
    !matches!(self, Item::Delimiter)
  }

  fn label(&self) -> &str {
    match self {
      Item::Action(s) | Item::Toggle(s, _) | Item::Submenu(s) => s,
      Item::Delimiter => "",
    }
  }
}

/// Build the item list for each top-level menu.
fn menu_items(id: MenuId, state: &MenuState) -> Vec<Item> {
  match id {
    MenuId::App => {
      let clock_on = consts::CLOCK_ENABLED.load(Ordering::Relaxed);
      let focus_on = consts::FOCUS_MODE.load(Ordering::Relaxed);
      vec![
        Item::Action("Insert File".into()),
        Item::Delimiter,
        Item::Submenu(if state.midi_output_devices.is_empty() {
          "MIDI Output (none) \u{25b8}".into()
        } else {
          format!("MIDI Output ({}) \u{25b8}", state.midi_output_devices.len())
        }),
        Item::Submenu(if state.midi_input_devices.is_empty() {
          "MIDI Input  (none) \u{25b8}".into()
        } else {
          format!("MIDI Input  ({}) \u{25b8}", state.midi_input_devices.len())
        }),
        Item::Toggle(
          format!("Clock Out [{}]", if clock_on { "ON " } else { "OFF" }),
          clock_on,
        ),
        Item::Delimiter,
        Item::Submenu("Scale (Left)      \u{25b8}".into()),
        Item::Submenu("Scale (Top)       \u{25b8}".into()),
        Item::Submenu("Scale Root (Top)  \u{25b8}".into()),
        Item::Delimiter,
        Item::Toggle(
          format!("Focus Mode   [{}]", if focus_on { "X" } else { " " }),
          focus_on,
        ),
        Item::Action("Release All".into()),
        Item::Action("Clear Queue".into()),
        Item::Delimiter,
        Item::Action("About".into()),
      ]
    }
    MenuId::View => vec![Item::Action("Docs [h]".into())],
    MenuId::Help => vec![Item::Action("Docs [h]".into())],
  }
}

/// Top-level menu titles in display order.
const MENU_TITLES: &[(MenuId, &str)] = &[
  (MenuId::App, "anupars"),
  (MenuId::View, "view"),
  (MenuId::Help, "help"),
];

/// Pixel (column) offset of each menu title, computed from title widths + separators.
fn menu_title_x(index: usize) -> u16 {
  let mut x = 1u16;
  for (i, (_, title)) in MENU_TITLES.iter().enumerate() {
    if i == index {
      return x;
    }
    x += title.len() as u16 + 2;
  }
  x
}

/// Action produced by `handle_menu_key`.
pub enum MenuAction {
  /// Focus, navigation, or dropdown toggle - state already updated.
  None,
  /// User confirmed "Insert File".
  InsertFile,
  /// User confirmed "About".
  About,
  /// User confirmed "Release All" (panic/clear midi).
  ReleaseAll,
  /// User confirmed "Clear Queue".
  ClearQueue,
  /// User toggled Clock Out.
  ToggleClock,
  /// User toggled Focus mode.
  ToggleFocus,
  /// Close menu and focus grid.
  Close,
  /// Quit the application.
  Quit,
}

/// Handle a key event when `Focus::Menu` is active.
/// Mutates `menu` and returns what action (if any) was confirmed.
#[cfg(not(target_arch = "wasm32"))]
pub fn handle_menu_key(menu: &mut MenuState, key: crossterm::event::KeyEvent) -> MenuAction {
  use crossterm::event::KeyCode;

  // If no dropdown is open yet, Left/Right opens one.
  if menu.active_menu.is_none() {
    match key.code {
      KeyCode::Left => {
        let last = MENU_TITLES.len() - 1;
        menu.active_menu = Some(MENU_TITLES[last].0);
        menu.active_item = 0;
        return MenuAction::None;
      }
      KeyCode::Right | KeyCode::Enter => {
        menu.active_menu = Some(MENU_TITLES[0].0);
        menu.active_item = 0;
        return MenuAction::None;
      }
      KeyCode::Esc => return MenuAction::Close,
      _ => return MenuAction::None,
    }
  }

  let current_id = menu.active_menu.unwrap();
  let items = menu_items(current_id, menu);

  match (key.modifiers, key.code) {
    (_, KeyCode::Esc) => {
      menu.active_menu = None;
      MenuAction::Close
    }
    (_, KeyCode::Left) => {
      let pos = MENU_TITLES
        .iter()
        .position(|(id, _)| *id == current_id)
        .unwrap_or(0);
      let next = if pos == 0 {
        MENU_TITLES.len() - 1
      } else {
        pos - 1
      };
      menu.active_menu = Some(MENU_TITLES[next].0);
      menu.active_item = 0;
      MenuAction::None
    }
    (_, KeyCode::Right) => {
      let pos = MENU_TITLES
        .iter()
        .position(|(id, _)| *id == current_id)
        .unwrap_or(0);
      let next = (pos + 1) % MENU_TITLES.len();
      menu.active_menu = Some(MENU_TITLES[next].0);
      menu.active_item = 0;
      MenuAction::None
    }
    (_, KeyCode::Up) => {
      let mut idx = menu.active_item;
      loop {
        idx = if idx == 0 { items.len() - 1 } else { idx - 1 };
        if items[idx].is_selectable() {
          break;
        }
      }
      menu.active_item = idx;
      MenuAction::None
    }
    (_, KeyCode::Down) => {
      let mut idx = menu.active_item;
      loop {
        idx = (idx + 1) % items.len();
        if items[idx].is_selectable() {
          break;
        }
      }
      menu.active_item = idx;
      MenuAction::None
    }
    (_, KeyCode::Enter) => {
      let action = confirm_item(current_id, menu.active_item, menu);
      if !matches!(action, MenuAction::None) {
        menu.active_menu = None;
      }
      action
    }
    _ => MenuAction::None,
  }
}

/// Map a confirmed item selection to a `MenuAction`.
fn confirm_item(id: MenuId, item_idx: usize, menu: &MenuState) -> MenuAction {
  let items = menu_items(id, menu);
  match items.get(item_idx) {
    None => MenuAction::None,
    Some(Item::Delimiter) => MenuAction::None,
    Some(Item::Submenu(_)) => MenuAction::None,
    Some(item) => {
      let label = item.label();
      if label.starts_with("Insert File") {
        MenuAction::InsertFile
      } else if label.starts_with("About") {
        MenuAction::About
      } else if label.starts_with("Release All") {
        MenuAction::ReleaseAll
      } else if label.starts_with("Clear Queue") {
        MenuAction::ClearQueue
      } else if label.starts_with("Clock Out") {
        MenuAction::ToggleClock
      } else if label.starts_with("Focus Mode") {
        MenuAction::ToggleFocus
      } else {
        MenuAction::None
      }
    }
  }
}

/// Draw the menu bar row and any open dropdown into `buf`.
/// `y_off` is the row of the menu bar (row 0 in a full-screen layout).
pub fn draw_menubar(
  state: &crate::app_state::AppState,
  buf: &mut crate::terminal::buffer::ScreenBuffer,
  y_off: u16,
) {
  use crate::terminal::cell::Color;
  use crate::view::printer::{apply_style, CellStyle};

  let w = buf.width;
  let bar_bg = Color::Rgb(30, 30, 30);
  let title_style = CellStyle {
    fg: Color::Rgb(200, 200, 200),
    bg: bar_bg,
    reverse: false,
  };
  let active_style = CellStyle {
    fg: Color::Rgb(0, 0, 0),
    bg: Color::Rgb(200, 200, 200),
    reverse: false,
  };
  let quit_style = CellStyle {
    fg: Color::Rgb(160, 60, 60),
    bg: bar_bg,
    reverse: false,
  };

  // Fill bar background.
  for x in 0..w {
    if let Some(c) = buf.get_mut(x, y_off) {
      apply_style(
        c,
        ' ',
        CellStyle {
          fg: Color::Reset,
          bg: bar_bg,
          reverse: false,
        },
      );
    }
  }

  // Draw each menu title.
  for (i, (id, title)) in MENU_TITLES.iter().enumerate() {
    let x = menu_title_x(i);
    let is_active = state.menu.active_menu == Some(*id)
      || (state.focus == crate::app_state::Focus::Menu
        && state.menu.active_menu.is_none()
        && i == 0);
    let style = if is_active { active_style } else { title_style };
    for (j, ch) in title.chars().enumerate() {
      if let Some(c) = buf.get_mut(x + j as u16, y_off) {
        apply_style(c, ch, style);
      }
    }
  }

  // "quit" at the far right.
  let quit_label = "quit";
  if w > quit_label.len() as u16 {
    let qx = w - quit_label.len() as u16 - 1;
    for (j, ch) in quit_label.chars().enumerate() {
      if let Some(c) = buf.get_mut(qx + j as u16, y_off) {
        apply_style(c, ch, quit_style);
      }
    }
  }

  // Draw open dropdown if any.
  let Some(open_id) = state.menu.active_menu else {
    return;
  };
  let title_idx = MENU_TITLES
    .iter()
    .position(|(id, _)| *id == open_id)
    .unwrap_or(0);
  let drop_x = menu_title_x(title_idx);
  let items = menu_items(open_id, &state.menu);
  let max_label_w = items.iter().map(|it| it.label().len()).max().unwrap_or(10);
  let drop_w = (max_label_w + 4) as u16;

  let item_bg = Color::Rgb(40, 40, 40);
  let item_fg = Color::Rgb(200, 200, 200);
  let dim_fg = Color::Rgb(80, 80, 80);
  let active_item_bg = Color::Rgb(100, 100, 100);

  for (row, item) in items.iter().enumerate() {
    let dy = y_off + 1 + row as u16;
    let is_active_item = row == state.menu.active_item;
    let (fg, bg) = if item.is_selectable() && is_active_item {
      (Color::Rgb(255, 255, 255), active_item_bg)
    } else if item.is_selectable() {
      (item_fg, item_bg)
    } else {
      (dim_fg, item_bg)
    };
    let row_style = CellStyle {
      fg,
      bg,
      reverse: false,
    };

    // Background fill for the dropdown row.
    for x in 0..drop_w {
      if let Some(c) = buf.get_mut(drop_x + x, dy) {
        apply_style(
          c,
          ' ',
          CellStyle {
            fg: Color::Reset,
            bg,
            reverse: false,
          },
        );
      }
    }

    match item {
      Item::Delimiter => {
        let sep_style = CellStyle {
          fg: dim_fg,
          bg: item_bg,
          reverse: false,
        };
        for x in 0..drop_w {
          if let Some(c) = buf.get_mut(drop_x + x, dy) {
            apply_style(c, '─', sep_style);
          }
        }
      }
      _ => {
        let label = item.label();
        for (j, ch) in label.chars().enumerate() {
          if let Some(c) = buf.get_mut(drop_x + 1 + j as u16, dy) {
            apply_style(c, ch, row_style);
          }
        }
      }
    }
  }
}

/// Return the path to the default location (~/.anupars/contents)
pub fn get_default_database_path() -> Result<PathBuf, Box<dyn Error>> {
  #[cfg(target_arch = "wasm32")]
  return Err("filesystem not available in WASM".into());

  #[cfg(not(target_arch = "wasm32"))]
  {
    let mut path = match dirs::home_dir().map(|p| p.join(consts::DEFAULT_APP_DIRECTORY)) {
      Some(d) => d,
      None => return Err("home directory not found".into()),
    };
    path.push(consts::DEFAULT_APP_FILENAME);
    if !path.is_dir() {
      fs::create_dir_all(&path)?;
    }
    Ok(path)
  }
}
