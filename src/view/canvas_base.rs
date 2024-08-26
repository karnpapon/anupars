use cursive::{
  direction::Direction,
  event::{Event, EventResult},
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView},
  Printer, Vec2,
};
use std::usize;

use crate::core::{config, traits::Matrix};

#[derive(Clone)]
pub struct CanvasBase {
  size: Vec2,
  grid: Matrix<char>,
  text_contents: Option<String>,
}

impl cursive::view::View for CanvasBase {
  fn draw(&self, printer: &Printer) {
    if self.size > Vec2::new(0, 0) {
      self.grid.print(printer);
    }
  }

  fn on_event(&mut self, _: Event) -> EventResult {
    EventResult::Consumed(None)
  }

  fn layout(&mut self, size: Vec2) {
    self.resize(size)
  }
}

impl CanvasBase {
  pub fn new() -> CanvasBase {
    CanvasBase {
      size: Vec2::new(181, 25),
      grid: Matrix::new(181, 25, '\0'),
      text_contents: None,
    }
  }

  pub fn build() -> ResizedView<ResizedView<NamedView<Canvas<CanvasBase>>>> {
    Canvas::new(CanvasBase::new())
      .with_draw(draw)
      // .with_needs_relayout(need_relayout)
      // .with_layout(layout)
      // .with_on_event(on_event)
      // .with_take_focus(take_focus)
      .with_name(config::canvas_base_section_view)
      .full_height()
      .full_width()
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Matrix::new(size.x, size.y, '\0');
    self.size = size;
    // self.grid().set_rect(size.x, size.y, '\0');
  }

  pub fn update_text_contents(&mut self, contents: &str) {
    self.text_contents = Some(String::from(contents));
  }

  pub fn update_grid_src(&mut self) {
    if self.text_contents.as_ref().is_none() {
      return;
    };

    let rows: usize = self.grid.width;
    let cols: usize = self.grid.height;

    for row in 0..rows {
      for col in 0..cols {
        if let Some(char) = self
          .text_contents
          .as_ref()
          .unwrap()
          .chars()
          .nth(col + (row * cols))
        {
          self.grid.set(col, row, char);
        }
      }
    }
  }

  // pub fn grid(&self) -> Matrix<char> {
  //   self.grid
  // }
}

fn draw(canvas: &CanvasBase, printer: &Printer) {
  if canvas.size > Vec2::new(0, 0) {
    canvas.grid.print(printer);
  }
}

fn need_relayout(_: &CanvasBase) -> bool {
  false
}

fn layout(canvas: &mut CanvasBase, size: Vec2) {
  canvas.resize(size)
}

fn take_focus(_: &mut CanvasBase, _: Direction) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

// fn on_event(_: &mut CanvasBase, _: Event) -> EventResult {
//   EventResult::Consumed(None)
// }
