use cursive::{
  event::{Callback, Event, EventResult},
  views::Canvas,
  Cursive, Printer, Vec2,
};

use super::grid::Grid;

pub struct CanvasView {
  pub canvas_view: Canvas<Grid>,
}

impl CanvasView {
  pub fn new() -> CanvasView {
    let c = Canvas::new(Grid::new(0, 0))
      .with_draw(draw)
      .with_layout(layout)
      .with_on_event(on_event);

    CanvasView { canvas_view: c }
  }
}

fn layout(s: &mut Grid, size: Vec2) {
  s.resize(size)
}

fn draw(s: &Grid, printer: &Printer) {
  for (x, row) in s.grid.iter().enumerate() {
    for (y, &value) in row.iter().enumerate() {
      let display_value = if value != '\0' {
        value
      } else if x % s.grid_row_spacing == 0 && y % s.grid_col_spacing == 0 {
        '+'
      } else {
        '.'
      };

      printer.print((x, y), &display_value.to_string())
    }
  }
  // printer.print((0, 1), &format!("{:.2?}", s.elapsed()));
}

fn on_event(_: &mut Grid, event: Event) -> EventResult {
  if event == Event::Refresh {
    // println!("refresh");
    // self.frame_idx += 1;
    // if self.frame_idx == self.max_frame_idx {
    //   self.frame_idx = 0;
    // } else {
    //   return EventResult::Ignored;
    // }
  }

  match event {
    Event::Char(' ') => EventResult::Consumed(Some(Callback::from_fn(run(Grid::toggle_play)))),
    _ => EventResult::Ignored,
  }
}

pub fn run<F>(f: F) -> impl Fn(&mut Cursive)
where
  F: Fn(&mut Grid),
{
  move |s| {
    s.call_on_name("canvas_view", |c: &mut Canvas<Grid>| {
      f(c.state_mut());
    });
  }
}
