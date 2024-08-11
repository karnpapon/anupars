use cursive::{
  reexports::time::{Duration, Instant},
  views::Canvas,
  Cursive, Vec2,
};
use std::ops::{Index, IndexMut};

#[derive(Clone)]
pub struct Grid {
  pub grid_row_spacing: usize,
  pub grid_col_spacing: usize,
  pub size: Vec2,
  pub grid: Vec<Vec<char>>,

  pub last_started: Instant,
  pub last_elapsed: Duration,
  pub running: bool,
}

impl Grid {
  pub fn new(rows: i32, cols: i32) -> Self {
    Grid {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      grid: (0..rows)
        .map(|_| (0..cols).map(|_| '\0').collect())
        .collect(),
      last_started: Instant::now(),
      last_elapsed: Duration::default(),
      running: true,
    }
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = (0..size.x)
      .map(|_| (0..size.y).map(|_| '\0').collect())
      .collect();
    self.size = size
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

  pub fn toggle_play(&mut self) {
    if self.running {
      self.pause()
    } else {
      self.start()
    }
  }

  pub fn start(&mut self) {
    if self.running {
      return;
    }
    self.running = true;
    self.last_started = Instant::now();
  }

  pub fn elapsed(&self) -> Duration {
    self.last_elapsed
      + if self.running {
        Instant::now() - self.last_started
      } else {
        Duration::default()
      }
  }

  pub fn pause(&mut self) {
    self.last_elapsed = self.elapsed();
    self.running = false;
  }
}

// impl Index<Vec2> for Grid {
//   type Output = Cell;

//   fn index(&self, pos: Vec2) -> &Self::Output {
//     &self.field[pos]
//   }
// }
