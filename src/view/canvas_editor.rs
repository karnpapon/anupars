use std::{thread, time::Instant, usize};

use cursive::{
  event::{Callback, Event, EventResult, Key, MouseButton, MouseEvent},
  theme::Style,
  utils::span::SpannedString,
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView, TextView},
  Cursive, Printer, Rect, Vec2, XY,
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
  text_contents: Option<String>,
  clock: u64,
}

#[derive(Clone)]
pub struct Marker {
  pos: Vec2,
  area: Rect,
  drag_start_x: usize,
  drag_start_y: usize,
  is_playing: bool,
  actived_pos: Vec2,
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

        if self.is_head((offset_x, offset_y).into()) {
          printer.print_styled(
            (offset_x, offset_y),
            &SpannedString::styled('>', Style::highlight()),
          );
          continue;
        }

        let (displayed_style, displayed_char) =
          if self.is_actived_position((offset_x, offset_y).into()) {
            (Style::none(), '*')
          } else {
            (
              Style::highlight(),
              editor
                .get(offset_x, offset_y)
                .display_char((offset_x, offset_y).into()),
            )
          };

        printer.print_styled(
          (offset_x, offset_y),
          &SpannedString::styled(displayed_char, displayed_style),
        );
      }
    }
  }

  fn is_head(&self, curr_pos: Vec2) -> bool {
    self.pos.eq(&curr_pos)
  }
  fn is_actived_position(&self, curr_pos: Vec2) -> bool {
    self.pos.saturating_add(self.actived_pos).eq(&curr_pos)
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
        view.set_content(utils::build_pos_status_str((pos_x, pos_y).into()));
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

    self.area = Rect::from_size((new_x, new_y), (new_w + 1, new_h + 1)); //offseting drag area
    self.drag_start_x = new_x;
    self.drag_start_y = new_y;
  }

  pub fn set_actived_pos(&mut self, pos: usize) {
    let prev_x = self.actived_pos.x;
    self.actived_pos.x = pos % self.area.width();

    if prev_x != 0 && self.actived_pos.x == 0 {
      self.actived_pos.y += 1;
      self.actived_pos.y %= self.area.height();
    }
    // crossbeam::scope(|scope| {
    //   scope.spawn(|_| {
    //   });
    // })
    // .unwrap();
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
        is_playing: false,
        actived_pos: Vec2::zero(),
      },
      grid: Matrix::new(0, 0, '\0'),
      text_contents: None,
      clock: 0,
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

  pub fn update_text_contents(&mut self, contents: &str) {
    self.text_contents = Some(String::from(contents));
  }

  pub fn update_grid_src(&mut self) {
    if self.text_contents.as_ref().is_none() {
      return;
    };

    let rows: usize = self.grid.width;
    let cols: usize = self.grid.height;

    let mut newline_idx_offset = 0;
    let mut prev_char_idx = 0;
    let mut mod_idx_offset = 0;
    let mut mod_idx_counter = 0;

    // TODO: clean up the mess. mostly, handling newline char logic.
    for row in 0..rows {
      for col in 0..cols {
        let char_idx = col + (row * cols);
        if let Some(char) = self.text_contents.as_ref().unwrap().chars().nth(char_idx) {
          if char == '\n' || char == '\r' {
            let line_pos = (char_idx - prev_char_idx) % rows;
            let placeholder_chars = rows - line_pos;
            for c in 0..placeholder_chars {
              self.grid.set(
                col + c + newline_idx_offset - prev_char_idx - mod_idx_offset,
                row,
                '\0'.display_char((char_idx + c + mod_idx_counter % rows, mod_idx_counter).into()),
              );
            }

            newline_idx_offset += line_pos + placeholder_chars;
            prev_char_idx = (char_idx + 1) % rows;
            mod_idx_counter += 1;
            if char_idx / rows > 0 {
              mod_idx_offset += rows;
            } else {
              mod_idx_offset = 0;
            }
          } else {
            self.grid.set(
              col + newline_idx_offset - prev_char_idx - mod_idx_offset,
              row,
              char,
            );
          }
        }
      }
    }
  }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Matrix::new(size.x, size.y, '\0');
    self.size = size;
  }

  pub fn get(&self, x: usize, y: usize) -> char {
    if self.marker.is_playing
      && self
        .marker
        .pos
        .saturating_add(self.marker.actived_pos)
        .eq(&(x, y))
    {
      return '*';
    }
    *self.grid.get(x, y).unwrap_or(&'.')
  }

  pub fn marker_mut(&mut self) -> &mut Marker {
    &mut self.marker
  }

  pub fn marker_area(&self) -> &Rect {
    &self.marker.area
  }

  pub fn set_playing(&mut self) -> bool {
    self.marker.is_playing = !self.marker.is_playing;
    self.marker.is_playing
  }
}

fn draw(canvas: &CanvasEditor, printer: &Printer) {
  canvas.marker.print(printer, canvas);
}

fn layout(canvas: &mut CanvasEditor, size: Vec2) {
  if canvas.size == Vec2::ZERO {
    canvas.resize(size)
  }
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
          view.set_content(utils::build_pos_status_str((pos_x, pos_y).into()))
        });

        siv.call_on_name(config::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str((w, h)));
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
          view.set_content(utils::build_len_status_str((w, h)));
        });
      })))
    }
    _ => EventResult::Ignored,
  }
}
