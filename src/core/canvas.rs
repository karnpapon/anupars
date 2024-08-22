use std::usize;

use cursive::{
  direction::Direction,
  event::{Callback, Event, EventResult, Key, MouseEvent},
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  view::CannotFocus,
  views::Canvas,
  Printer, Vec2, XY,
};

#[derive(Clone)]
pub struct CanvasView {
  pub grid_row_spacing: usize,
  pub grid_col_spacing: usize,
  pub size: Vec2,
  pub marker: Marker,
  pub grid: Vec<Vec<char>>,
}

#[derive(Clone, Default)]
pub struct Marker {
  is_playing: bool,
  grid_h: usize,
  pos: Vec2,
  grid_w: usize,
  drag_start_x: usize,
  drag_start_y: usize,
}

impl Marker {
  pub fn set_current_pos(&mut self, pos: XY<usize>, offset: XY<usize>) {
    let pos_x = pos.x.abs_diff(1);
    let pos_y = pos.y.abs_diff(offset.y);
    self.pos = (pos_x, pos_y).into();
  }

  pub fn set_grid_area(&mut self, current_pos: XY<usize>) {
    let new_w = current_pos.x.abs_diff(self.pos.x).clamp(1, usize::MAX);
    let new_h = current_pos.y.abs_diff(self.pos.y).clamp(1, usize::MAX);
    let new_x = match current_pos.x.saturating_sub(self.pos.x) == 0 {
      true => current_pos.x,
      false => self.pos.x,
    };

    let new_y = match current_pos.y.saturating_sub(self.pos.y) == 0 {
      true => current_pos.y,
      false => self.pos.y,
    };

    self.grid_w = new_w;
    self.grid_h = new_h;
    self.drag_start_x = new_x;
    self.drag_start_y = new_y;
  }
}

impl CanvasView {
  pub fn new() -> Canvas<CanvasView> {
    Canvas::new(CanvasView {
      grid_row_spacing: 9,
      grid_col_spacing: 9,
      size: Vec2::new(0, 0),
      marker: Marker {
        pos: Vec2::new(0, 0),
        is_playing: false,
        grid_h: 1,
        grid_w: 1,
        drag_start_y: 0,
        drag_start_x: 0,
      },
      grid: vec![],
    })
    .with_draw(draw)
    .with_layout(layout)
    .with_on_event(on_event)
    .with_take_focus(take_focus)
  }

  pub fn char_at(&self, x: usize, y: usize) -> char {
    self.grid[y][x]
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

  // pub fn update_grid_src(&mut self, src: &str) -> Vec<Vec<char>> {
  pub fn update_grid_src(&mut self, src: &str) {
    let rows: usize = self.grid.len();
    let cols: usize = self.grid[0].len();

    for row in 0..rows {
      for col in 0..cols {
        if let Some(char) = src.chars().nth(col + (row * cols)) {
          self.set_char_at(row, col, char);
        }
      }
    }
    // self.grid.clone()
  }
}

fn layout(canvas: &mut CanvasView, size: Vec2) {
  canvas.resize(size)
}

pub fn draw(canvas: &CanvasView, printer: &Printer) {
  // if canvas.size > Vec2::new(0, 0) {
  //   for (x, row) in canvas.grid.iter().enumerate() {
  //     for (y, &value) in row.iter().enumerate() {
  //       printer.print_styled(
  //         (y, x),
  //         &SpannedString::styled(
  //           &value.to_string(),
  //           Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
  //         ),
  //       );
  //     }
  //   }
  // }
}

fn take_focus(_: &mut CanvasView, _: Direction) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(canvas: &mut CanvasView, event: Event) -> EventResult {
  match event {
    Event::Key(Key::Right) => {
      // canvas.marker.pos.x += 1;
      EventResult::consumed()
    }
    Event::Refresh => EventResult::consumed(),
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Press(_btn),
    } => {
      canvas.marker.set_current_pos(position, offset);

      let current_pos = canvas.marker.pos;

      EventResult::Consumed(Some(Callback::from_fn(move |siv| {
        siv.call_on_name(
          "canvas_section_view",
          move |view: &mut Canvas<CanvasView>| {
            view.set_draw(move |v, printer| {
              printer.print_styled(
                current_pos,
                &SpannedString::styled(
                  v.char_at(current_pos.x, current_pos.y).to_string(),
                  Style::highlight(),
                ),
              )
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

      canvas.marker.set_grid_area((pos_x, pos_y).into());

      let Marker {
        drag_start_x,
        drag_start_y,
        grid_w,
        grid_h,
        ..
      } = canvas.marker;

      let new_x = drag_start_x;
      let new_y = drag_start_y;
      let new_w = grid_w;
      let new_h = grid_h;

      EventResult::Consumed(Some(Callback::from_fn(move |siv| {
        siv.call_on_name(
          "canvas_section_view",
          move |view: &mut Canvas<CanvasView>| {
            view.set_draw(move |v, printer| {
              for w in 0..new_w {
                for h in 0..new_h {
                  printer.print_styled(
                    (new_x + w, new_y + h),
                    &SpannedString::styled(
                      v.char_at(new_x + w, new_y + h).to_string(),
                      Style::highlight(),
                    ),
                  );
                }
              }
            });
            // view.set_draw(move |v, printer| {
            //   for row in 0..v.grid.len() {
            //     for col in 0..v.grid[0].len() {
            //       if row == (new_x) || col == (new_y) {
            //         for w in 0..new_w {
            //           for h in 0..new_h {
            //             printer.print_styled(
            //               (new_x + w, new_y + h),
            //               &SpannedString::styled(
            //                 v.char_at(new_x + w, new_y + h).to_string(),
            //                 Style::highlight(),
            //               ),
            //             );
            //           }
            //         }
            //       } else {
            //         printer.print_styled(
            //           (col, row),
            //           &SpannedString::styled(
            //             ".",
            //             Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
            //           ),
            //         );
            //       }
            //     }
            //   }
            // });
          },
        );
      })))
    }
    _ => EventResult::Ignored,
  }
}
