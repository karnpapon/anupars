use std::{
  mem::replace,
  sync::{atomic::AtomicUsize, Arc},
};

use cursive::{
  direction::Direction,
  event::{Event, EventResult, Key, MouseEvent},
  theme::{ColorStyle, ColorType, Style},
  utils::{self, span::SpannedString},
  view::CannotFocus,
  views::Canvas,
  Printer, Vec2,
};

#[derive(Clone)]
pub struct CanvasView {
  pub grid_row_spacing: usize,
  pub grid_col_spacing: usize,
  pub size: Vec2,
  pub selector: Selector,
  pub grid: Vec<Vec<char>>,
  pub counter: cursive::utils::Counter,
}

#[derive(Clone, Default)]
pub struct Selector {
  pos: Vec2,
}

impl CanvasView {
  pub fn new() -> Canvas<CanvasView> {
    Canvas::new(CanvasView {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      selector: Selector {
        pos: Vec2::new(0, 0),
      },
      grid: vec![],
      counter: cursive::utils::Counter::new(0),
    })
    .with_draw(draw)
    .with_layout(layout)
    .with_on_event(on_event)
    .with_take_focus(take_focus)
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = vec![vec!['\0'; size.x]; size.y];
    self.size = size;
  }

  pub fn update_grid_src(&mut self, src: &str) -> Vec<Vec<char>> {
    let rows: usize = self.grid.len();
    let cols: usize = self.grid[0].len();
    let mut new_grid = vec![vec!['\0'; cols]; rows];

    for row in 0..rows {
      for col in 0..cols {
        if let Some(char) = src.chars().nth(col + (row * cols)) {
          new_grid[row][col] = char
        }
      }
    }
    new_grid
  }
}

fn layout(canvas: &mut CanvasView, size: Vec2) {
  canvas.resize(size)
}

pub fn draw(canvas: &CanvasView, printer: &Printer) {
  if canvas.size > Vec2::new(0, 0) {
    for (x, row) in canvas.grid.iter().enumerate() {
      for (y, &value) in row.iter().enumerate() {
        let display_value = match value {
          '\0' => {
            if x % canvas.grid_row_spacing == 0 && y % canvas.grid_col_spacing == 0 {
              '+'
            } else {
              '.'
            }
          }
          _ => value,
        };
        printer.print_styled(
          (y, x),
          &SpannedString::styled(
            &display_value.to_string(),
            Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
          ),
        );
      }
    }
  }
}

fn take_focus(_: &mut CanvasView, _: Direction) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(canvas: &mut CanvasView, event: Event) -> EventResult {
  match event {
    Event::Key(Key::Right) => {
      // canvas.selector.pos.x += 1;
      EventResult::consumed()
    }
    Event::Refresh => EventResult::consumed(),
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Press(_btn),
    } => {
      canvas.selector.pos = position;
      EventResult::consumed()
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Release(_),
    } => {
      // println!("mouse released");
      EventResult::consumed()
    }
    _ => EventResult::Ignored,
  };

  EventResult::Ignored
}
