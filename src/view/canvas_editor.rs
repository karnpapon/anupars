use std::{
  sync::{Arc, Mutex},
  usize,
};

use cursive::{
  event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent},
  theme::Style,
  utils::span::SpannedString,
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView, TextView},
  Printer, Rect, Vec2, XY,
};

use crate::core::{
  config,
  traits::{Matrix, Printable},
  utils,
};

#[derive(Clone)]
pub struct CanvasEditor {
  size: Vec2,
  marker: Marker,
  grid: Matrix<char>,
}

#[derive(Clone)]
pub struct Marker {
  pos: Vec2,
  area: Rect,
  drag_start_x: usize,
  drag_start_y: usize,
}

enum Direction {
  Up,
  Down,
  Left,
  Right,
  Idle,
}

impl Direction {
  pub fn get_direction(&self) -> (i32, i32) {
    match self {
      Direction::Right => (1, 0),
      Direction::Up => (0, -1),
      Direction::Left => (-1, 0),
      Direction::Down => (0, 1),
      Direction::Idle => (0, 0),
    }
  }
}

impl Marker {
  pub fn print(&self, printer: &Printer, editor: &CanvasEditor) {
    for x in 0..self.area.width() {
      for y in 0..self.area.height() {
        let offset_x = self.pos.x + x;
        let offset_y = self.pos.y + y;
        printer.print_styled(
          (offset_x, offset_y),
          &SpannedString::styled(
            editor
              .get(offset_x, offset_y)
              .display_char((offset_x, offset_y).into())
              .to_string(),
            Style::highlight(),
          ),
        );
      }
    }
  }

  fn set_move(&mut self, direction: Direction, canvas_size: Vec2) -> EventResult {
    let next_pos = self.pos.saturating_add(direction.get_direction());
    let next_pos_bottom_right: Vec2 = (
      next_pos.x + self.area.width() - 1,
      next_pos.y + self.area.height() - 1,
    )
      .into();

    if !next_pos_bottom_right.fits_in_rect(Vec2::ZERO, canvas_size) {
      return EventResult::Ignored;
    }

    self.pos = next_pos;

    let pos_x = self.pos.x;
    let pos_y = self.pos.y;

    EventResult::Consumed(Some(Callback::from_fn(move |siv| {
      siv.call_on_name(config::pos_status_unit_view, move |view: &mut TextView| {
        view.set_content(utils::build_pos_status_str(pos_x, pos_y));
      });
    })))
  }

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

    self.area = Rect::from_size((new_x, new_y), (new_w, new_h));
    self.drag_start_x = new_x;
    self.drag_start_y = new_y;
  }
}

impl CanvasEditor {
  pub fn new() -> CanvasEditor {
    CanvasEditor {
      size: Vec2::zero(),
      marker: Marker {
        pos: Vec2::zero(),
        area: Rect::from_point(Vec2::zero()),
        drag_start_y: 0,
        drag_start_x: 0,
      },
      grid: Matrix::new(0, 0, '\0'),
      // text_contents: None,
    }
  }

  pub fn build() -> ResizedView<ResizedView<NamedView<Canvas<CanvasEditor>>>> {
    Canvas::new(CanvasEditor::new())
      .with_draw(draw)
      .with_layout(layout)
      .with_on_event(on_event)
      .with_take_focus(take_focus)
      .with_name(config::canvas_editor_section_view)
      .full_height()
      .full_width()
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Matrix::new(size.x, size.y, '\0');
    self.size = size;
  }

  pub fn get(&self, x: usize, y: usize) -> char {
    *self.grid.get(x, y).unwrap_or(&'.')
  }
}

fn draw(canvas: &CanvasEditor, printer: &Printer) {
  canvas.marker.print(printer, canvas);
}

fn layout(canvas: &mut CanvasEditor, size: Vec2) {
  canvas.resize(size)
}

fn take_focus(
  _: &mut CanvasEditor,
  _: cursive::direction::Direction,
) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(canvas: &mut CanvasEditor, event: Event) -> EventResult {
  match event {
    Event::Refresh => EventResult::consumed(),
    Event::Key(Key::Left) => canvas.marker.set_move(Direction::Left, canvas.size),
    Event::Key(Key::Right) => canvas.marker.set_move(Direction::Right, canvas.size),
    Event::Key(Key::Up) => canvas.marker.set_move(Direction::Up, canvas.size),
    Event::Key(Key::Down) => canvas.marker.set_move(Direction::Down, canvas.size),
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Press(_btn),
    } => {
      canvas.marker.set_current_pos(position, offset);
      canvas.marker.set_move(Direction::Idle, canvas.size);

      let pos_x = canvas.marker.pos.x;
      let pos_y = canvas.marker.pos.y;
      let w = canvas.marker.area.width();
      let h = canvas.marker.area.height();

      EventResult::Consumed(Some(Callback::from_fn(move |siv| {
        siv.call_on_name(config::pos_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_pos_status_str(pos_x, pos_y))
        });

        siv.call_on_name(config::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str(w, h));
        });
      })))
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Hold(MouseButton::Left),
    } => {
      // TODO: not sure why these (`MouseEvent::Hold`) sometimes being called twice (bug?) !?
      // need more investigate on this

      let pos_x = position.x.abs_diff(1);
      let pos_y = position.y.abs_diff(offset.y);
      canvas.marker.set_grid_area((pos_x, pos_y).into());
      let w = canvas.marker.area.width();
      let h = canvas.marker.area.height();

      EventResult::Consumed(Some(Callback::from_fn(move |siv| {
        siv.call_on_name(config::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str(w, h));
        });
      })))
    }
    _ => EventResult::Ignored,
  }
}

// ------------- (temp) helpers -------------------

fn run<F>(f: F) -> impl Fn(&mut cursive::Cursive)
where
  F: Fn(&mut CanvasEditor),
{
  move |s| {
    s.call_on_name(
      config::canvas_editor_section_view,
      |c: &mut Canvas<CanvasEditor>| {
        f(c.state_mut());
      },
    );
  }
}

// -----------------------------------------------
