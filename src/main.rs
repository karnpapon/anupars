// mod context;

// use context::Context;
// use ncurses::{
//   cbreak, curs_set, getmouse, initscr, keypad, mmask_t, mousemask, nodelay, noecho, refresh,
//   resize_term, waddch, wgetch, wmove, wresize, ALL_MOUSE_EVENTS, CURSOR_VISIBILITY, KEY_BACKSPACE,
//   KEY_DC, KEY_DOWN, KEY_LEFT, KEY_MOUSE, KEY_RIGHT, KEY_UP, MEVENT, REPORT_MOUSE_POSITION,
// };
// use std::sync::{Arc, Mutex};
// use std::thread::sleep;
// use std::time::Duration;

// fn main() {
//   let rows = 30;
//   let cols = 100;
//   let grid_row_spacing = 9;
//   let grid_col_spacing = 9;
//   let grid: Vec<Vec<char>> = (0..rows)
//     .map(|_| (0..cols).map(|_| '\0').collect())
//     .collect();
//   let context = Context::new(grid, 120, 4);

//   let context_arc = Arc::new(Mutex::new(context));

//   let (mut cursor_row, mut cursor_col): (usize, usize) = (0, 0);

//   let window = initscr();
//   resize_term(rows, cols);
//   cbreak();
//   noecho();
//   curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
//   mousemask(
//     ALL_MOUSE_EVENTS as mmask_t | REPORT_MOUSE_POSITION as mmask_t,
//     None,
//   );
//   wresize(window, rows, cols);
//   keypad(window, true);
//   nodelay(window, true);
//   refresh();

//   loop {
//     let grid = {
//       let _context = context_arc.lock().unwrap();
//       _context.grid.clone()
//     };
//     wmove(window, 0, 0);
//     for (r, row) in grid.iter().enumerate() {
//       for (c, &value) in row.iter().enumerate() {
//         let display_value = if value != '\0' {
//           value
//         } else if r % grid_row_spacing == 0 && c % grid_col_spacing == 0 {
//           '+'
//         } else {
//           '.'
//         };
//         waddch(window, display_value as u32);
//       }
//     }
//     wmove(window, cursor_row as i32, cursor_col as i32);

//     match wgetch(window) {
//       KEY_UP => {
//         cursor_row -= 1;
//       }
//       KEY_DOWN => {
//         cursor_row += 1;
//       }
//       KEY_LEFT => {
//         cursor_col -= 1;
//       }
//       KEY_RIGHT => {
//         cursor_col += 1;
//       }
//       KEY_BACKSPACE => {
//         let mut _context = context_arc.lock().unwrap();
//         _context.grid[cursor_row][cursor_col] = '\0';
//       }
//       KEY_DC => {
//         let mut _context = context_arc.lock().unwrap();
//         _context.grid[cursor_row][cursor_col] = '\0';
//       }
//       KEY_MOUSE => {
//         let mut mevent = MEVENT {
//           id: 0,
//           x: 0,
//           y: 0,
//           z: 0,
//           bstate: 0,
//         };
//         let error = getmouse(&mut mevent);
//         if error == 0 {
//           cursor_row = mevent.y as usize;
//           cursor_col = mevent.x as usize;
//         }
//       }
//       key => {
//         waddch(window, key.try_into().unwrap());
//         let mut _context = context_arc.lock().unwrap();
//         // _context.grid[cursor_row][cursor_col] = key;
//         println!("unexpected input: {:?}", key);
//       }
//     }

//     sleep(Duration::from_millis(10));
//   }
// }

mod context;

use crate::context::Context;
use pancurses::{
  cbreak, curs_set, getmouse, initscr, mousemask, noecho, resize_term, Input, ALL_MOUSE_EVENTS,
  REPORT_MOUSE_POSITION,
};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

fn main() {
  let rows = 30;
  let cols = 100;
  let grid_row_spacing = 9;
  let grid_col_spacing = 9;
  let grid: Vec<Vec<char>> = (0..rows)
    .map(|_| (0..cols).map(|_| '\0').collect())
    .collect();
  let context = Context::new(grid, 120, 4);

  let context_arc = Arc::new(Mutex::new(context));

  let (mut cursor_row, mut cursor_col): (usize, usize) = (0, 0);

  let mut window = initscr();
  resize_term(rows, cols);
  cbreak();
  noecho();
  curs_set(2);
  mousemask(ALL_MOUSE_EVENTS | REPORT_MOUSE_POSITION, None);
  window.resize(rows, cols);
  window.keypad(true);
  window.nodelay(true);
  window.refresh();

  loop {
    let grid = {
      let _context = context_arc.lock().unwrap();
      _context.grid.clone()
    };
    window.mv(0, 0);
    for (r, row) in grid.iter().enumerate() {
      for (c, &value) in row.iter().enumerate() {
        let display_value = if value != '\0' {
          value
        } else if r % grid_row_spacing == 0 && c % grid_col_spacing == 0 {
          '+'
        } else {
          '.'
        };
        window.addch(display_value);
      }
    }
    window.mv(cursor_row as i32, cursor_col as i32);

    match window.getch() {
      Some(Input::KeyUp) => {
        cursor_row -= 1;
      }
      Some(Input::KeyDown) => {
        cursor_row += 1;
      }
      Some(Input::KeyLeft) => {
        cursor_col -= 1;
      }
      Some(Input::KeyRight) => {
        cursor_col += 1;
      }
      Some(Input::KeyBackspace) => {
        let mut _context = context_arc.lock().unwrap();
        _context.grid[cursor_row][cursor_col] = '\0';
      }
      Some(Input::KeyDC) => {
        let mut _context = context_arc.lock().unwrap();
        _context.grid[cursor_row][cursor_col] = '\0';
      }
      Some(Input::KeyMouse) => {
        if let Ok(mouse_event) = getmouse() {
          cursor_row = mouse_event.y as usize;
          cursor_col = mouse_event.x as usize;
        }
      }
      Some(Input::Character(mut c)) => {
        if c == '\x08' {
          c = '\0';
        }
        window.addch(c);
        let mut _context = context_arc.lock().unwrap();
        _context.grid[cursor_row][cursor_col] = c;
      }
      None => (),
      other_inputs => {
        println!("unexpected input: {:?}", other_inputs);
      }
    }

    sleep(Duration::from_millis(10));
  }
}
