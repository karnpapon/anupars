use std::usize;

use cursive::{
  direction::Direction,
  event::EventResult,
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  view::CannotFocus,
  views::Canvas,
  Printer, Vec2,
};

#[derive(Clone)]
pub struct CanvasBase {
  pub grid_row_spacing: usize,
  pub grid_col_spacing: usize,
  pub size: Vec2,
  pub grid: Vec<Vec<char>>,
}

impl CanvasBase {
  pub fn new() -> Canvas<CanvasBase> {
    Canvas::new(CanvasBase {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      grid: vec![],
    })
    .with_draw(draw)
    .with_layout(layout)
    // .with_on_event(on_event)
    .with_take_focus(take_focus)
  }

  pub fn set_char_at(&mut self, x: usize, y: usize, glyph: char) {
    self.grid[x][y] = glyph;
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = vec![vec!['\0'; size.x]; size.y];
    self.size = size;
    self.set_empty_char();
  }

  pub fn set_empty_char(&mut self) {
    let temp_grid = self.grid.clone();
    for (x, row) in temp_grid.iter().enumerate() {
      for (y, &value) in row.iter().enumerate() {
        let display_value = match value {
          '\0' => {
            if x % self.grid_row_spacing == 0 && y % self.grid_col_spacing == 0 {
              '+'
            } else {
              '.'
            }
          }
          _ => value,
        };

        self.set_char_at(x, y, display_value);
      }
    }
  }
}

fn layout(canvas: &mut CanvasBase, size: Vec2) {
  canvas.resize(size)
}

pub fn draw(canvas: &CanvasBase, printer: &Printer) {
  if canvas.size > Vec2::new(0, 0) {
    for (x, row) in canvas.grid.iter().enumerate() {
      for (y, &value) in row.iter().enumerate() {
        printer.print_styled(
          (y, x),
          &SpannedString::styled(
            &value.to_string(),
            Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
          ),
        );
      }
    }
  }
}

fn take_focus(_: &mut CanvasBase, _: Direction) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}
