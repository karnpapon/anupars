use std::borrow::BorrowMut;

use cursive::{
  event::{Event, EventResult},
  view::{Nameable, Resizable},
  views::{Canvas, Panel},
  Cursive, Printer, Vec2, View,
};

pub struct CanvasView {
  pub grid_row_spacing: usize,
  pub grid_col_spacing: usize,
  pub size: Vec2,
  pub grid: Vec<Vec<char>>,
}

impl CanvasView {
  pub fn new(w: usize, h: usize) -> Self {
    CanvasView {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      grid: (0..w).map(|_| (0..h).map(|_| '\0').collect()).collect(),
    }
  }

  pub fn init(&mut self, siv: &mut Cursive) {
    let canvas = CanvasView {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      grid: (0..self.size.x)
        .map(|_| (0..self.size.y).map(|_| '\0').collect())
        .collect(),
    };

    siv.add_layer(canvas.with_name("canvas_view").full_width().full_height());
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = (0..size.x)
      .map(|_| (0..size.y).map(|_| '\0').collect())
      .collect();
    self.size = size
  }

  fn draw_canvas(&self, printer: &Printer) {
    for (x, row) in self.grid.iter().enumerate() {
      for (y, &value) in row.iter().enumerate() {
        let display_value = if value != '\0' {
          value
        } else if x % self.grid_row_spacing == 0 && y % self.grid_col_spacing == 0 {
          '+'
        } else {
          '.'
        };

        printer.print((x, y), &display_value.to_string())
      }
    }
  }

  pub fn update_grid_src(&mut self, src: &str) {
    let rows = self.grid.len();
    let cols = self.grid[0].len();

    for row in 0..rows {
      for col in 0..cols {
        if let Some(char) = src.chars().nth(row + col) {
          self.grid[row][col] = char;
        }
      }
    }
  }
}

impl View for CanvasView {
  fn layout(&mut self, size: Vec2) {
    self.resize(size)
  }

  fn draw(&self, printer: &Printer) {
    if self.size > Vec2::new(0, 0) {
      self.draw_canvas(printer);
    }
  }

  fn on_event(&mut self, event: Event) -> EventResult {
    // if event == Event::Refresh {}

    // match event {
    //   Event::Char(' ') => EventResult::Consumed(Some(Callback::from_fn(run(Grid::toggle_play)))),
    //   _ => EventResult::Ignored,
    // }
    EventResult::Ignored
  }
}

// pub fn run<F>(f: F) -> impl Fn(&mut Cursive)
// where
//   F: Fn(&mut CanvasView),
// {
//   move |s| {
//     s.call_on_name("canvas_view", |c: &mut CanvasView| {
//       f(c.state_mut());
//     });
//   }
// }
