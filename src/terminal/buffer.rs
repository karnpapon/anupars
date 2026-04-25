use super::cell::Cell;

pub struct ScreenBuffer {
  pub cells: Vec<Cell>,
  pub width: u16,
  pub height: u16,
}

impl ScreenBuffer {
  pub fn new(width: u16, height: u16) -> Self {
    Self {
      cells: vec![Cell::default(); width as usize * height as usize],
      width,
      height,
    }
  }

  pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
    if x < self.width && y < self.height {
      Some(&mut self.cells[y as usize * self.width as usize + x as usize])
    } else {
      None
    }
  }

  pub fn resize(&mut self, width: u16, height: u16) {
    self.width = width;
    self.height = height;
    self.cells = vec![Cell::default(); width as usize * height as usize];
  }

  pub fn clear(&mut self) {
    for cell in self.cells.iter_mut() {
      *cell = Cell::default();
    }
  }
}
