use crate::core::grid::Grid;
use cursive::{Printer, Vec2};

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
  }
}
