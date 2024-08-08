use crate::pkg::model::CellState::*;
use cursive::Vec2;
use std::ops::{Index, IndexMut};
use std::slice::Iter;

#[derive(Clone, Copy)]
pub struct Options {
  pub size: Vec2,
  pub mines: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellContent {
  Free(usize),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum CellState {
  Closed,
  Opened,
}

#[derive(Copy, Clone)]
pub struct Cell {
  pub state: CellState,

  pub content: CellContent,
}

impl Cell {
  pub fn new(content: CellContent) -> Self {
    Self {
      state: CellState::Closed,
      content,
    }
  }
}

pub struct Field {
  size: Vec2,
  cells: Vec<Cell>,
}

impl Field {
  fn new(size: Vec2) -> Self {
    Self {
      size,
      // init stub for cells, see method `Field::place_bombs()` details
      cells: vec![Cell::new(CellContent::Free(0)); size.x * size.y],
    }
  }

  fn pos_to_cell_idx(&self, pos: Vec2) -> usize {
    pos.x + pos.y * self.size.x
  }
}

impl Index<Vec2> for Field {
  type Output = Cell;

  fn index(&self, pos: Vec2) -> &Self::Output {
    &self.cells[self.pos_to_cell_idx(pos)]
  }
}

impl IndexMut<Vec2> for Field {
  fn index_mut(&mut self, pos: Vec2) -> &mut Self::Output {
    let idx = self.pos_to_cell_idx(pos);
    &mut self.cells[idx]
  }
}

pub struct Grid {
  pub field: Field,
  pub grid_row_spacing: usize,
  pub grid_col_spacing: usize,
  pub grid: Vec<Vec<char>>,
}

impl Grid {
  pub fn new(rows: i32, cols: i32) -> Self {
    Grid {
      field: Field::new(Vec2::new(1, 1)),
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      grid: (0..rows)
        .map(|_| (0..cols).map(|_| '\0').collect())
        .collect(),
    }
  }

  pub fn resize_grid(&mut self, size: Vec2) {
    self.grid = (0..size.x)
      .map(|_| (0..size.y).map(|_| '\0').collect())
      .collect()
  }
}

impl Index<Vec2> for Grid {
  type Output = Cell;

  fn index(&self, pos: Vec2) -> &Self::Output {
    &self.field[pos]
  }
}
