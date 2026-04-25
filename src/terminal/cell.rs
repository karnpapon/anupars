#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Color {
  #[default]
  Reset,
  Rgb(u8, u8, u8),
  Indexed(u8),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Cell {
  pub ch: char,
  pub fg: Color,
  pub bg: Color,
  pub bold: bool,
  pub underline: bool,
  pub reverse: bool,
}

impl Default for Cell {
  fn default() -> Self {
    Cell {
      ch: ' ',
      fg: Color::Indexed(7), // white
      bg: Color::Indexed(0), // black
      bold: false,
      underline: false,
      reverse: false,
    }
  }
}
