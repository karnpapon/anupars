use crossterm::event::KeyCode;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use crate::core::tonal::scale::{ScaleMode, ScaleRoot};
use crate::terminal::cell::Color;
use crate::view::printer::apply_style;
use crate::view::printer::black;

use crate::view::printer::dim;
use crate::view::printer::primary;

use crate::view::printer::white;
use crate::view::printer::CellStyle;

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
      vec![
        // Item::Action("Insert File".into()),
        // Item::Delimiter,
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
        Item::Submenu("Scale (Left) \u{25b8}".into()),
        Item::Submenu("Scale (Top) \u{25b8}".into()),
        Item::Submenu("Scale Root (Top) \u{25b8}".into()),
        Item::Delimiter,
        Item::Action("Release All".into()),
        Item::Action("Clear Queue".into()),
        Item::Delimiter,
        Item::Action("About".into()),
      ]
    }
    MenuId::View => {
      let focus_on = consts::FOCUS_MODE.load(Ordering::Relaxed);
      vec![Item::Toggle(
        format!("Focus Mode [{}]", if focus_on { "X" } else { " " }),
        focus_on,
      )]
    }
    MenuId::Help => vec![Item::Action("Docs".into())],
  }
}

/// Top-level menu titles in display order.
const MENU_TITLES: &[(MenuId, &str)] = &[
  (MenuId::App, "anupars"),
  (MenuId::View, "view"),
  (MenuId::Help, "help"),
];

/// Uniform slot width: widest title + 1 space padding on each side.
const TAB_SLOT_W: u16 = {
  let mut max = 0usize;
  let mut i = 0;
  while i < MENU_TITLES.len() {
    let len = MENU_TITLES[i].1.len();
    if len > max {
      max = len;
    }
    i += 1;
  }
  max as u16 + 2
};

/// Column offset of each menu title (uniform slot width + 1 gap).
fn menu_title_x(index: usize) -> u16 {
  let mut x = 1u16;
  for i in 0..MENU_TITLES.len() {
    if i == index {
      return x;
    }
    x += TAB_SLOT_W + 1;
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
  /// User selected a MIDI output device by port index.
  MidiOutputSelected(usize),
  /// User selected a MIDI input device by port index.
  MidiInputSelected(usize),
  /// User selected a scale mode for the left keyboard by index into ScaleMode::all().
  ScaleLeftSelected(usize),
  /// User selected a scale mode for the top keyboard by index into ScaleMode::all().
  ScaleTopSelected(usize),
  /// User selected a scale root for the top keyboard by index into ScaleRoot::all().
  ScaleRootSelected(usize),
  /// User opened Docs (coming soon).
  ShowDocs,
}

/// Return the list of string labels for a submenu identified by its parent item label.
fn submenu_items_for(label: &str, state: &MenuState) -> Vec<String> {
  if label.starts_with("MIDI Output") {
    if state.midi_output_devices.is_empty() {
      vec!["(no devices)".into()]
    } else {
      state.midi_output_devices.clone()
    }
  } else if label.starts_with("MIDI Input") {
    if state.midi_input_devices.is_empty() {
      vec!["(no devices)".into()]
    } else {
      state.midi_input_devices.clone()
    }
  } else if label.starts_with("Scale (Left)") {
    ScaleMode::all()
      .iter()
      .map(|m| m.name().to_string())
      .collect()
  } else if label.starts_with("Scale (Top)") {
    ScaleMode::all()
      .iter()
      .map(|m| m.name().to_string())
      .collect()
  } else if label.starts_with("Scale Root") {
    ScaleRoot::all()
      .iter()
      .map(|r| r.name().to_string())
      .collect()
  } else {
    vec![]
  }
}

/// Handle a key event when `Focus::Menu` is active.
/// Mutates `menu` and returns what action (if any) was confirmed.
/// Platform-independent directional key for menu navigation.
pub enum MenuNavKey {
  Left,
  Right,
  Up,
  Down,
  Enter,
  Esc,
  Other,
}

/// Core menu navigation logic, shared by native and WASM.
pub fn handle_menu_nav(menu: &mut MenuState, key: MenuNavKey) -> MenuAction {
  // No dropdown open - Left/Right moves tab focus, Enter/Down expands.
  if menu.active_menu.is_none() {
    match key {
      MenuNavKey::Left => {
        menu.focused_tab = if menu.focused_tab == 0 {
          MENU_TITLES.len() - 1
        } else {
          menu.focused_tab - 1
        };
        return MenuAction::None;
      }
      MenuNavKey::Right => {
        menu.focused_tab = (menu.focused_tab + 1) % MENU_TITLES.len();
        return MenuAction::None;
      }
      MenuNavKey::Enter | MenuNavKey::Down => {
        menu.active_menu = Some(MENU_TITLES[menu.focused_tab].0);
        menu.active_item = 0;
        return MenuAction::None;
      }
      MenuNavKey::Esc => return MenuAction::Close,
      _ => return MenuAction::None,
    }
  }

  let current_id = menu.active_menu.unwrap();
  let items = menu_items(current_id, menu);

  // If a submenu is open, route keys there first.
  if let Some(sub_idx) = menu.open_submenu {
    let sub_label = items.get(sub_idx).map(|i| i.label()).unwrap_or("");
    let sub_items = submenu_items_for(sub_label, menu);
    match key {
      MenuNavKey::Up => {
        if menu.submenu_item > 0 {
          menu.submenu_item -= 1;
        } else {
          menu.submenu_item = sub_items.len().saturating_sub(1);
        }
      }
      MenuNavKey::Down => {
        if menu.submenu_item + 1 < sub_items.len() {
          menu.submenu_item += 1;
        } else {
          menu.submenu_item = 0;
        }
      }
      MenuNavKey::Left | MenuNavKey::Esc => {
        menu.open_submenu = None;
      }
      MenuNavKey::Enter => {
        let chosen = menu.submenu_item;
        menu.open_submenu = None;
        menu.active_menu = None;
        if sub_label.starts_with("MIDI Output") {
          return MenuAction::MidiOutputSelected(chosen);
        } else if sub_label.starts_with("MIDI Input") {
          return MenuAction::MidiInputSelected(chosen);
        } else if sub_label.starts_with("Scale (Left)") {
          return MenuAction::ScaleLeftSelected(chosen);
        } else if sub_label.starts_with("Scale (Top)") {
          return MenuAction::ScaleTopSelected(chosen);
        } else if sub_label.starts_with("Scale Root") {
          return MenuAction::ScaleRootSelected(chosen);
        }
      }
      _ => {}
    }
    return MenuAction::None;
  }

  match key {
    MenuNavKey::Esc => {
      // Collapse dropdown back to tab-row focus, don't close the whole menubar.
      let pos = MENU_TITLES
        .iter()
        .position(|(id, _)| *id == current_id)
        .unwrap_or(0);
      menu.focused_tab = pos;
      menu.active_menu = None;
      MenuAction::None
    }
    MenuNavKey::Left => {
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
      menu.focused_tab = next;
      menu.active_item = 0;
      MenuAction::None
    }
    MenuNavKey::Right => {
      // Open submenu if the focused item is a Submenu, otherwise switch tab.
      if let Some(Item::Submenu(_)) = items.get(menu.active_item) {
        let sub_label = items[menu.active_item].label();
        let sub_items = submenu_items_for(sub_label, menu);
        if !sub_items.is_empty() {
          menu.open_submenu = Some(menu.active_item);
          menu.submenu_item = 0;
        }
        return MenuAction::None;
      }
      let pos = MENU_TITLES
        .iter()
        .position(|(id, _)| *id == current_id)
        .unwrap_or(0);
      let next = (pos + 1) % MENU_TITLES.len();
      menu.active_menu = Some(MENU_TITLES[next].0);
      menu.focused_tab = next;
      menu.active_item = 0;
      MenuAction::None
    }
    MenuNavKey::Up => {
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
    MenuNavKey::Down => {
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
    MenuNavKey::Enter => {
      let action = confirm_item(current_id, menu.active_item, menu);
      let is_toggle = matches!(action, MenuAction::ToggleClock | MenuAction::ToggleFocus);
      if !matches!(action, MenuAction::None) && !is_toggle {
        menu.active_menu = None;
      }
      action
    }
    MenuNavKey::Other => MenuAction::None,
  }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_menu_key(menu: &mut MenuState, key: crossterm::event::KeyEvent) -> MenuAction {
  let nav = match key.code {
    KeyCode::Left => MenuNavKey::Left,
    KeyCode::Right => MenuNavKey::Right,
    KeyCode::Up => MenuNavKey::Up,
    KeyCode::Down => MenuNavKey::Down,
    KeyCode::Enter => MenuNavKey::Enter,
    KeyCode::Esc => MenuNavKey::Esc,
    _ => MenuNavKey::Other,
  };
  handle_menu_nav(menu, nav)
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
      } else if label.starts_with("Docs") {
        MenuAction::ShowDocs
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
  let w = buf.width;
  let title_style = CellStyle::subtle();
  let active_style = CellStyle::highlight();
  let _quit_style = CellStyle::danger();

  // Fill bar background.
  for x in 0..w {
    if let Some(c) = buf.get_mut(x, y_off) {
      apply_style(c, ' ', CellStyle::reset());
    }
  }

  // Draw each menu title.
  for (i, (id, title)) in MENU_TITLES.iter().enumerate() {
    let x = menu_title_x(i);
    let is_active = state.menu.active_menu == Some(*id)
      || (state.focus == crate::app_state::Focus::Menu
        && state.menu.active_menu.is_none()
        && i == state.menu.focused_tab);
    let style = if is_active { active_style } else { title_style };
    // Fill entire slot with style background then draw title at offset 1.
    for col in 0..TAB_SLOT_W {
      if let Some(c) = buf.get_mut(x + col, y_off) {
        apply_style(c, ' ', style);
      }
    }
    for (j, ch) in title.chars().enumerate() {
      if let Some(c) = buf.get_mut(x + 1 + j as u16, y_off) {
        apply_style(c, ch, style);
      }
    }
  }

  // "quit" at the far right.
  // let quit_label = "quit";
  // if w > quit_label.len() as u16 {
  //   let qx = w - quit_label.len() as u16 - 1;
  //   for (j, ch) in quit_label.chars().enumerate() {
  //     if let Some(c) = buf.get_mut(qx + j as u16, y_off) {
  //       apply_style(c, ch, quit_style);
  //     }
  //   }
  // }

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

  let item_bg = Color::Reset;
  let item_fg = primary();
  let dim_fg = dim();
  let border_style = CellStyle::secondary();

  // Top border row.
  let top_y = y_off + 1;
  if let Some(c) = buf.get_mut(drop_x, top_y) {
    apply_style(c, '┌', border_style);
  }
  for x in 1..=drop_w {
    if let Some(c) = buf.get_mut(drop_x + x, top_y) {
      apply_style(c, '─', border_style);
    }
  }
  if let Some(c) = buf.get_mut(drop_x + drop_w + 1, top_y) {
    apply_style(c, '┐', border_style);
  }

  for (row, item) in items.iter().enumerate() {
    let dy = y_off + 2 + row as u16;
    let is_active_item = row == state.menu.active_item;
    let (fg, bg) = if item.is_selectable() && is_active_item {
      (black(), white())
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

    // Left border.
    if let Some(c) = buf.get_mut(drop_x, dy) {
      apply_style(c, '│', border_style);
    }
    // Background fill for the inner row.
    for x in 1..=drop_w {
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
    // Right border.
    if let Some(c) = buf.get_mut(drop_x + drop_w + 1, dy) {
      apply_style(c, '│', border_style);
    }

    match item {
      Item::Delimiter => {
        let sep_style = CellStyle::dim();
        for x in 1..=drop_w {
          if let Some(c) = buf.get_mut(drop_x + x, dy) {
            apply_style(c, '─', sep_style);
          }
        }
      }
      Item::Submenu(_) => {
        // Strip trailing spaces and the ▸ glyph from the stored label,
        // then draw ▸ at the fixed right edge of the panel.
        let raw = item.label();
        let clean = raw.trim_end_matches('\u{25b8}').trim_end();
        for (j, ch) in clean.chars().enumerate() {
          if let Some(c) = buf.get_mut(drop_x + 2 + j as u16, dy) {
            apply_style(c, ch, row_style);
          }
        }
        // ▸ with 1 col of right padding before the border.
        if let Some(c) = buf.get_mut(drop_x + drop_w - 1, dy) {
          apply_style(c, '\u{25b8}', row_style);
        }
      }
      _ => {
        let label = item.label();
        for (j, ch) in label.chars().enumerate() {
          if let Some(c) = buf.get_mut(drop_x + 2 + j as u16, dy) {
            apply_style(c, ch, row_style);
          }
        }
      }
    }
  }

  // Bottom border row.
  let bot_y = y_off + 2 + items.len() as u16;
  if let Some(c) = buf.get_mut(drop_x, bot_y) {
    apply_style(c, '└', border_style);
  }
  for x in 1..=drop_w {
    if let Some(c) = buf.get_mut(drop_x + x, bot_y) {
      apply_style(c, '─', border_style);
    }
  }
  if let Some(c) = buf.get_mut(drop_x + drop_w + 1, bot_y) {
    apply_style(c, '┘', border_style);
  }

  // Draw submenu panel if one is open.
  let Some(sub_idx) = state.menu.open_submenu else {
    return;
  };
  let sub_label = items.get(sub_idx).map(|i| i.label()).unwrap_or("");
  let sub_items = submenu_items_for(sub_label, &state.menu);
  if sub_items.is_empty() {
    return;
  }

  let sub_max_w = sub_items.iter().map(|s| s.len()).max().unwrap_or(10);
  let sub_w = (sub_max_w + 2) as u16;
  // Position submenu to the right of the main dropdown, aligned to the parent row.
  let sub_x = drop_x + drop_w + 2;
  let sub_top_y = y_off + 1 + sub_idx as u16;

  // Top border.
  if let Some(c) = buf.get_mut(sub_x, sub_top_y) {
    apply_style(c, '┌', border_style);
  }
  for x in 1..=sub_w {
    if let Some(c) = buf.get_mut(sub_x + x, sub_top_y) {
      apply_style(c, '─', border_style);
    }
  }
  if let Some(c) = buf.get_mut(sub_x + sub_w + 1, sub_top_y) {
    apply_style(c, '┐', border_style);
  }

  for (row, label) in sub_items.iter().enumerate() {
    let dy = sub_top_y + 1 + row as u16;
    let is_active = row == state.menu.submenu_item;
    let (fg, bg) = if is_active {
      (black(), white())
    } else {
      (item_fg, item_bg)
    };
    let row_style = CellStyle {
      fg,
      bg,
      reverse: false,
    };
    if let Some(c) = buf.get_mut(sub_x, dy) {
      apply_style(c, '│', border_style);
    }
    for x in 1..=sub_w {
      if let Some(c) = buf.get_mut(sub_x + x, dy) {
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
    if let Some(c) = buf.get_mut(sub_x + sub_w + 1, dy) {
      apply_style(c, '│', border_style);
    }
    for (j, ch) in label.chars().enumerate() {
      if let Some(c) = buf.get_mut(sub_x + 2 + j as u16, dy) {
        apply_style(c, ch, row_style);
      }
    }
  }

  // Bottom border.
  let sub_bot_y = sub_top_y + 1 + sub_items.len() as u16;
  if let Some(c) = buf.get_mut(sub_x, sub_bot_y) {
    apply_style(c, '└', border_style);
  }
  for x in 1..=sub_w {
    if let Some(c) = buf.get_mut(sub_x + x, sub_bot_y) {
      apply_style(c, '─', border_style);
    }
  }
  if let Some(c) = buf.get_mut(sub_x + sub_w + 1, sub_bot_y) {
    apply_style(c, '┘', border_style);
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
