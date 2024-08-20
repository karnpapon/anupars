use std::borrow::Borrow;

use cursive::{
  direction::Direction,
  event::{Callback, Event, EventResult, Key, MouseEvent},
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
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
      let pos_x = position.x.abs_diff(1);
      let pos_y = position.y.abs_diff(offset.y);
      canvas.selector.pos = (pos_x, pos_y).into();
      EventResult::Consumed(Some(Callback::from_fn(move |siv| {
        siv.call_on_name(
          "canvas_section_view",
          move |view: &mut Canvas<CanvasView>| {
            view.set_draw(move |_, printer| {
              printer.print_styled(
                (pos_x, pos_y),
                &SpannedString::styled(".".to_string(), Style::highlight()),
              );
            });
          },
        );
      })))
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Hold(_),
    } => {
      let pos_x = position.x.abs_diff(1);
      let pos_y = position.y.abs_diff(offset.y);

      let new_w = pos_x.abs_diff(canvas.selector.pos.x);
      let new_h = pos_y.abs_diff(canvas.selector.pos.y);
      let new_x = match pos_x.saturating_sub(canvas.selector.pos.x) == 0 {
        true => pos_x,
        false => canvas.selector.pos.x,
      };

      let new_y = match pos_y.saturating_sub(canvas.selector.pos.y) == 0 {
        true => pos_y,
        false => canvas.selector.pos.y,
      };

      EventResult::Consumed(Some(Callback::from_fn(move |siv| {
        siv.call_on_name(
          "canvas_section_view",
          move |view: &mut Canvas<CanvasView>| {
            view.set_draw(move |_, printer| {
              for w in 0..new_w {
                for h in 0..new_h {
                  printer.print_styled(
                    (new_x + w, new_y + h),
                    &SpannedString::styled(".".to_string(), Style::highlight()),
                  );
                }
              }
            });
          },
        );
      })))

      // let tcy = pos_y;
      // let tcx = pos_x;

      // Usz tcy = a->drag_start_y;
      // Usz tcx = a->drag_start_x;
      // Usz loy = y < tcy ? y : tcy;
      // Usz lox = x < tcx ? x : tcx;
      // Usz hiy = y > tcy ? y : tcy;
      // Usz hix = x > tcx ? x : tcx;
      // a->ged_cursor.y = loy;
      // a->ged_cursor.x = lox;
      // a->ged_cursor.h = hiy - loy + 1;
      // a->ged_cursor.w = hix - lox + 1;
      // a->is_draw_dirty = true;
    }
    _ => EventResult::Ignored,
  }
}

// fn view_to_scrolled_grid(field_len: i32, visual_coord: i32, scroll_offset: i32) -> i32 {
// if field_len == 0 { return 0; }
// if scroll_offset < 0 {
// if (-scroll_offset) <= visual_coord {
// visual_coord -= (-scroll_offset);
// } else {
// visual_coord = 0;
// }
// } else {
// visual_coord += scroll_offset;
// }
// if visual_coord >= field_len {
//   visual_coord = field_len - 1;
// }
// return visual_coord;
// }
