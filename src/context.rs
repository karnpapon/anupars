use std::collections::{HashMap, HashSet};

pub struct Context {
  pub grid: Vec<Vec<char>>,
  pub width: usize,
  pub height: usize,
  pub locks: HashSet<(i32, i32)>,
  pub variables: HashMap<char, char>,
  pub ticks: usize,
  pub tempo: u64,
  pub divisions: u64,
  pub tick_time: u64,
}

impl Context {
  pub fn new(grid: Vec<Vec<char>>, tempo: u64, divisions: u64) -> Context {
    let width = grid[0].len();
    let height = grid.len();
    Context {
      grid,
      width,
      height,
      locks: HashSet::new(),
      variables: HashMap::new(),
      ticks: 0,
      tempo,
      divisions,
      tick_time: 60000 / (tempo * divisions),
    }
  }
  #[allow(dead_code)]
  pub fn display(&self) {
    let rows = self.grid.len();
    let cols = self.grid[0].len();
    for row in 0..rows {
      for col in 0..cols {
        print!("{}", self.grid[row][col]);
      }
      println!();
    }
    // println!("{:?}", self.notes);
  }
}
