use std::{collections::HashMap, sync::mpsc::Sender, usize};

use cursive::{
  event::{Event, EventResult, Key, MouseButton, MouseEvent},
  view::{CannotFocus, Nameable, Resizable},
  views::{Canvas, NamedView, ResizedView},
  Printer, Vec2,
};

use crate::core::{config, rect::Rect, regex::Match, traits::Matrix};

use super::marker::{self, Direction, Message};

pub struct MarkerUI {
  pub marker_area: Rect,
  pub marker_pos: Vec2,
  pub actived_pos: Vec2,
}

pub struct CanvasEditor {
  size: Vec2,
  marker_tx: Sender<Message>,
  pub grid: Matrix<char>,
  pub text_contents: Option<String>,
  pub text_matcher: Option<HashMap<usize, Match>>,
  pub marker_ui: MarkerUI, // midi_tx: Sender<midi::Message>,
                           // hold_key: bool,
}

impl MarkerUI {
  fn new() -> Self {
    MarkerUI {
      marker_area: Rect::from_point(Vec2::zero()),
      marker_pos: Vec2::zero(),
      actived_pos: Vec2::zero(),
    }
  }
}

impl CanvasEditor {
  pub fn new(marker_tx: Sender<marker::Message>) -> CanvasEditor {
    CanvasEditor {
      size: Vec2::zero(),
      marker_tx,
      grid: Matrix::new(0, 0, '\0'),
      text_contents: None,
      text_matcher: None,
      marker_ui: MarkerUI::new(), // midi_tx,
                                  // hold_key: false,
    }
  }

  pub fn build(
    marker_tx: Sender<marker::Message>,
  ) -> ResizedView<ResizedView<NamedView<Canvas<CanvasEditor>>>> {
    Canvas::new(CanvasEditor::new(marker_tx))
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

  // pub fn update_grid_src(&mut self) {
  //   if self.text_contents.as_ref().is_none() {
  //     return;
  //   };

  //   let rows: usize = self.grid.width;
  //   let cols: usize = self.grid.height;

  //   let mut newline_idx_offset = 0;
  //   let mut prev_char_idx = 0;
  //   let mut mod_idx_offset = 0;
  //   let mut mod_idx_counter = 0;

  //   // TODO: clean up the mess. mostly, handling newline char logic.
  //   for row in 0..rows {
  //     for col in 0..cols {
  //       let char_idx = col + (row * cols);
  //       if let Some(char) = self.text_contents.as_ref().unwrap().chars().nth(char_idx) {
  //         if char == '\n' || char == '\r' {
  //           let line_pos = (char_idx - prev_char_idx) % rows;
  //           let placeholder_chars = rows - line_pos;
  //           for c in 0..placeholder_chars {
  //             self.grid.set(
  //               col + c + newline_idx_offset - prev_char_idx - mod_idx_offset,
  //               row,
  //               '\0'.display_char((char_idx + c + mod_idx_counter % rows, mod_idx_counter).into()),
  //             );
  //           }

  //           newline_idx_offset += line_pos + placeholder_chars;
  //           prev_char_idx = (char_idx + 1) % rows;
  //           mod_idx_counter += 1;
  //           if char_idx / rows > 0 {
  //             mod_idx_offset += rows;
  //           } else {
  //             mod_idx_offset = 0;
  //           }
  //         } else {
  //           self.grid.set(
  //             col + newline_idx_offset - prev_char_idx - mod_idx_offset,
  //             row,
  //             char,
  //           );
  //         }
  //       }
  //     }
  //   }
  // }

  pub fn resize(&mut self, size: Vec2) {
    self.grid = Matrix::new(size.x, size.y, '\0');
    self.size = size;
  }

  pub fn get(&self, x: usize, y: usize) -> char {
    // if self
    //   .marker
    //   .pos
    //   .saturating_add(self.marker.actived_pos)
    //   .eq(&(x, y))
    // {
    //   return '*';
    // }
    *self.grid.get(x, y).unwrap_or(&'.')
  }

  // pub fn marker_mut(&mut self) -> &mut Marker {
  //   &mut self.marker
  // }

  pub fn set_text_matcher(&mut self, text_matcher: Option<HashMap<usize, Match>>) {
    self.text_matcher = text_matcher
  }

  // pub fn clear_marker_midi_msg_config_list(&mut self) {
  //   let mut midi_msg_config_list = self.marker.midi_msg_config_list.lock().unwrap();
  //   midi_msg_config_list.clear();
  // }

  // pub fn set_marker_midi_msg_config_list(&mut self, midi: midi::MidiMsg) {
  //   let mut midi_msg_config_list = self.marker.midi_msg_config_list.lock().unwrap();
  //   midi_msg_config_list.push(midi);
  // }

  fn index_to_xy(&self, index: &usize) -> Vec2 {
    let x = index % self.size.x;
    let y = index / self.size.x;
    (x, y).into()
  }

  pub fn text_contents(&self) -> String {
    self
      .text_contents
      .as_ref()
      .unwrap_or(&"".to_string())
      .to_string()
  }
}

fn draw(canvas: &CanvasEditor, printer: &Printer) {
  canvas
    .grid
    .print(printer, &canvas.text_matcher, &canvas.marker_ui);
  // canvas.marker.print(printer, canvas);
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
    Event::Key(Key::Left) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Left, canvas.size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Key(Key::Right) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Right, canvas.size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Key(Key::Up) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Up, canvas.size))
        .unwrap();
      EventResult::consumed()
    }
    Event::Key(Key::Down) => {
      canvas
        .marker_tx
        .send(Message::Move(Direction::Down, canvas.size))
        .unwrap();
      EventResult::Ignored
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Press(_btn),
    } => {
      canvas
        .marker_tx
        .send(Message::SetCurrentPos(position, offset))
        .unwrap();
      canvas
        .marker_tx
        .send(Message::Move(Direction::Idle, canvas.size))
        .unwrap();
      canvas
        .marker_tx
        .send(Message::UpdateInfoStatusView())
        .unwrap();

      EventResult::consumed()
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

      canvas
        .marker_tx
        .send(Message::SetGridArea((pos_x, pos_y).into()))
        .unwrap();

      EventResult::Ignored
    }
    _ => EventResult::Ignored,
  }
}
