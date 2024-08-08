use crate::pkg::grid::{CellContent, CellState, Grid, Options};
use cursive::event::{Event, EventResult, MouseButton, MouseEvent};
use cursive::theme::{BaseColor, Color, ColorStyle};
use cursive::{Cursive, Printer, Vec2, XY};

pub struct Canvas {
  pub grid: Grid,
}

impl Canvas {
  pub fn new() -> Canvas {
    let g = Grid::new(0, 0);
    Canvas { grid: g }
  }
}

impl cursive::view::View for Canvas {
  fn layout(&mut self, size: Vec2) {
    self.grid.resize_grid(size)
  }

  fn draw(&self, printer: &Printer) {
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

    // for i in self.grid.size.iter() {
    //   // let x = (i % self.grid.size.x) * 2;
    //   // let y = i / self.grid.size.x;
    //   let x = i % 20;
    //   let y = i % 20;

    //   println!("draw i:{:?}", i);
    // println!("draw x:{:?}, y:{:?}", x, y);

    // let text = match (cell.state, cell.content) {
    //   (Closed, _) => " .",
    //   (Marked, _) => " +",
    //   (Opened, Free(n)) => ["  ", " 1", " 2", " 3", " 4", " 5", " 6", " 7", " 8"][n],
    //   (Opened, Bomb) => "\u{01F4A3}",
    // };

    // let color = match (cell.state, cell.content) {
    //   (Closed, _) => Color::RgbLowRes(3, 3, 3),
    //   (Marked, _) => Color::RgbLowRes(4, 4, 2),
    //   (Opened, Free(n)) => match n {
    //     1 => Color::RgbLowRes(3, 5, 3),
    //     2 => Color::RgbLowRes(5, 5, 3),
    //     3 => Color::RgbLowRes(5, 4, 3),
    //     4 => Color::RgbLowRes(5, 3, 3),
    //     5 => Color::RgbLowRes(5, 2, 2),
    //     6 => Color::RgbLowRes(5, 0, 1),
    //     7 => Color::RgbLowRes(5, 0, 2),
    //     8 => Color::RgbLowRes(5, 0, 3),
    //     _ => Color::Dark(BaseColor::White),
    //   },
    //   _ => Color::Dark(BaseColor::White),
    // };
    // printer.print((x, y), ".")
    // printer.with_color(
    //   ColorStyle::new(Color::Dark(BaseColor::Black), color),
    //   |printer| printer.print((x, y), text),
    // );
    // }
  }
}
