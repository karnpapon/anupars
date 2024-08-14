use cursive::{
  direction::Direction,
  event::{Event, EventResult, MouseEvent},
  view::CannotFocus,
  views::Canvas,
  Printer, Vec2, View,
};

#[derive(Clone, Default)]
pub struct CanvasView {
  pub grid_row_spacing: usize,
  pub grid_col_spacing: usize,
  pub size: Vec2,
  pub grid: Vec<Vec<char>>,
}

impl CanvasView {
  pub fn new(w: usize, h: usize) -> Canvas<CanvasView> {
    Canvas::new(CanvasView {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      grid: (0..w).map(|_| (0..h).map(|_| '\0').collect()).collect(),
    })
    .with_draw(draw)
    .with_layout(layout)
    .with_on_event(on_event)
    .with_take_focus(take_focus)
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = (0..size.x)
      .map(|_| (0..size.y).map(|_| '\0').collect())
      .collect();
    self.size = size
  }

  pub fn draw_canvas(&self, printer: &Printer) {
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

fn layout(canvas: &mut CanvasView, size: Vec2) {
  canvas.resize(size)
}

fn draw(canvas: &CanvasView, printer: &Printer) {
  if canvas.size > Vec2::new(0, 0) {
    canvas.draw_canvas(printer);
  }
}

fn take_focus(canvas: &mut CanvasView, _: Direction) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(canvas: &mut CanvasView, event: Event) -> EventResult {
  match event {
    Event::Refresh => {
      println!("refresh!!!");
      // canvas.
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Press(_btn),
    } => {
      // println!("nouse_pressed pos = {:?}", position)
      // Get cell for position
      // if let Some(pos) = self.get_cell(position, offset) {
      //   self.focused = Some(pos);
      //   return EventResult::Consumed(None);
      // }
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Release(btn),
    } => {
      // println!("nouse_released pos = {:?}", position)
      // Get cell for position
      // if let Some(pos) = self.get_cell(position, offset) {
      //   if self.focused == Some(pos) {
      //     // We got a click here!
      //     match btn {
      //       MouseButton::Left => return self.reveal(pos),
      //       MouseButton::Right => {
      //         self.flag(pos);
      //         return EventResult::Consumed(None);
      //       }
      //       MouseButton::Middle => return self.auto_reveal(pos),
      //       _ => (),
      //     }
      //   }

      //   self.focused = None;
      // }
    }
    _ => (),
  }

  EventResult::Ignored
}
