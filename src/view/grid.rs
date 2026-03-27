use std::sync::mpsc::Sender;

use cursive::event::Callback;
use cursive::event::Event;
use cursive::event::EventResult;
use cursive::event::MouseButton;
use cursive::event::MouseEvent;
use cursive::theme::ColorStyle;
use cursive::theme::ColorType;
use cursive::theme::Style;
use cursive::view::CannotFocus;
use cursive::view::Nameable;
use cursive::view::Resizable;
use cursive::views::Canvas;
use cursive::views::NamedView;
use cursive::views::ResizedView;
use cursive::views::TextView;
use cursive::Cursive;
use cursive::Printer;
use cursive::Vec2;

use ringbuffer::RingBuffer;

use crate::core::playhead::queue::EVENT_OPERATORS;
use crate::core::playhead::queue::QUEUE_OPERATORS;
use crate::core::playhead::PlayheadUI;
use crate::core::{consts, playhead::queue::PendingJumpPosition};
use crate::view::printer::Matrix;

use consts::BASE_OCTAVE;
use consts::KEYBOARD_MARGIN_BOTTOM;
use consts::KEYBOARD_MARGIN_LEFT;
use consts::KEYBOARD_MARGIN_TOP;
use consts::NOTE_NAMES;
use consts::QUEUE_MARGIN_RIGHT;

use crate::core::engine::regex;
use crate::core::playhead::{Direction, Message as PlayheadMessage};

pub struct GridEditor {
  size: Vec2,
  pub playhead_tx: Sender<PlayheadMessage>,
  pub grid: Matrix<char>,
  pub text_contents: Option<String>,
  pub playhead_ui: PlayheadUI,
  pub show_keyboard: bool,
  pub is_canvas_focused: bool,
  pub regex_tx: Option<Sender<regex::Message>>,
  pub last_pattern: String,
  pub last_flag_str: String,
}

impl GridEditor {
  pub fn new(playhead_tx: Sender<PlayheadMessage>) -> GridEditor {
    GridEditor {
      size: Vec2::zero(),
      playhead_tx,
      grid: Matrix::new(0, 0, '\0'),
      text_contents: None,
      playhead_ui: PlayheadUI::new(),
      show_keyboard: true,
      is_canvas_focused: false,
      regex_tx: None,
      last_pattern: String::new(),
      last_flag_str: String::new(),
    }
  }

  /// Map Y position to MIDI note information (for left keyboard)
  /// Y increases downward, so higher Y = lower note (inverted keyboard)
  pub fn y_to_note_left(&self, y: usize) -> (f32, u8, &'static str) {
    let total_rows = self.grid.height;
    if total_rows == 0 {
      return (0.0, BASE_OCTAVE, "C");
    }

    let (note_index, octave) = self.playhead_ui.scale_mode_left.pos_to_scale_note(
      y,
      total_rows,
      BASE_OCTAVE,
      self.playhead_ui.scale_root_left.to_root_offset(),
    );

    (
      note_index,
      octave,
      NOTE_NAMES[note_index.round() as usize % 12],
    )
  }

  /// Map Y position to MIDI note information (for top keyboard)
  /// Y increases downward, so higher Y = lower note (inverted keyboard)
  pub fn y_to_note_top(&self, y: usize) -> (f32, u8, &'static str) {
    let total_rows = self.grid.height;
    if total_rows == 0 {
      return (0.0, BASE_OCTAVE, "C");
    }

    let (note_index, octave) = self.playhead_ui.scale_mode_top.pos_to_scale_note(
      y,
      total_rows,
      BASE_OCTAVE,
      self.playhead_ui.scale_root_top.to_root_offset(),
    );

    (
      note_index,
      octave,
      NOTE_NAMES[note_index.round() as usize % 12],
    )
  }

  /// Draw the keyboard visualization on the top margin
  fn draw_keyboard_top(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.height == 0 || self.grid.width == 0 {
      return;
    }

    let abs_active_x = self.playhead_ui.playhead_pos.x + self.playhead_ui.actived_pos.x;
    for x in 0..self.grid.width {
      let y_pos = x % self.grid.height;
      let (note_index, octave, note_name) = self.y_to_note_top(y_pos);

      let is_black_key = matches!(note_index.round() as u8, 1 | 3 | 6 | 8 | 10); // C#, D#, F#, G#, A#

      let style = if x == abs_active_x {
        if note_name == "C" {
          Style::from(ColorStyle::new(
            ColorType::rgb(0, 0, 0),
            ColorType::rgb(255, 255, 255),
          ))
        } else {
          Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)))
        }
      } else if note_name == "C" {
        Style::from(ColorStyle::new(
          ColorType::rgb(0, 0, 0),
          ColorType::rgb(100, 100, 100),
        ))
      } else {
        Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)))
      };

      printer.with_style(style, |printer| {
        if is_black_key {
          printer.print((x, 0), "#");
        } else if note_name == "C" {
          printer.print((x, 0), " ");
          printer.print((x, 1), &octave.to_string());
        } else {
          printer.print((x, 0), "━");
        }
      });
    }
  }

  fn draw_keyboard_left(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.height == 0 {
      return;
    }

    for y in 0..self.grid.height {
      let (note_index, octave, note_name) = self.y_to_note_left(y);

      // Determine text color based on note (white/black keys)
      let is_black_key = matches!(note_index.round() as u8, 1 | 3 | 6 | 8 | 10); // C#, D#, F#, G#, A#

      let text_color = if is_black_key {
        ColorType::rgb(50, 50, 50)
      } else {
        ColorType::rgb(100, 100, 100)
      };

      let style = Style::from(ColorStyle::front(text_color));

      // Format note label (e.g., "C3", "D#4")
      let label = format!("{}{}", note_name, octave);
      let symbol = if note_name == "C" { "┣" } else { "┃" };

      printer.with_style(style, |printer| {
        printer.print((0, y), &label);
        printer.print((3, y), &":".repeat(KEYBOARD_MARGIN_LEFT - 6));
        printer.print((KEYBOARD_MARGIN_LEFT - 2, y), symbol);
      });
    }
  }

  fn draw_queue_right(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.height == 0 {
      return;
    }

    let total_height = self.grid.height;
    let half_height = total_height / 2;

    let style = Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)));
    // let dimmed_style = Style::from(ColorStyle::front(ColorType::rgb(50, 50, 50)));

    // Top half: EVQ (Event Queue)
    // Read and display event queue items (bottom-aligned)
    let event_queue = self.playhead_ui.queue_manager.event_queue.lock().unwrap();
    let evq_items: Vec<String> = event_queue
      .iter()
      .map(|op| op.get_event_name().to_string())
      .collect();
    drop(event_queue);

    let evq_start_y = if evq_items.len() >= half_height - 2 {
      0
    } else {
      half_height.saturating_sub(evq_items.len() + 2)
    };

    // Draw empty queue placeholder blocks for EVQ
    for y in 0..(evq_start_y + 1) {
      let placeholder_color = if y % 2 == 0 {
        ColorType::rgb(50, 50, 50)
      } else {
        ColorType::rgb(70, 70, 70)
      };
      let placeholder_style = Style::from(ColorStyle::front(placeholder_color));
      printer.with_style(placeholder_style, |printer| {
        for x in 1..QUEUE_MARGIN_RIGHT {
          printer.print((x, y), consts::QUEUE_PLACEHOLDER_SYMBOL);
        }
      });
    }

    // Draw EVQ items bottom-up: item[0] (oldest) at bottom, newest stacks upward
    for (idx, item) in evq_items.iter().enumerate() {
      if idx + 2 >= half_height {
        break;
      }
      let y = half_height - 2 - idx;
      printer.with_style(style, |printer| {
        printer.print((3, y), item);
      });
    }

    // EVQ label at bottom of top half
    printer.with_style(style, |printer| {
      printer.print((3, half_height - 1), "EVNTQ");
    });

    // Draw separator line between EVQ and OPQ
    let center_offset = 1;
    printer.with_style(style, |printer| {
      let center_x = (QUEUE_MARGIN_RIGHT / 2) + center_offset;
      for x in 0..QUEUE_MARGIN_RIGHT {
        if x == center_x {
          printer.print((x, half_height), "v");
        } else {
          printer.print((x, half_height), " ");
        }
      }
    });

    // Bottom half: OPQ (Operator Queue)
    let opq_start_y = half_height;

    // Read and display operator queue items (bottom-aligned)
    let operator_queue = self
      .playhead_ui
      .queue_manager
      .operator_queue
      .lock()
      .unwrap();
    let opq_items: Vec<String> = operator_queue
      .iter()
      .map(|item| format!("{}", item))
      .collect();
    drop(operator_queue);

    let opq_display_start_y = if opq_items.len() >= total_height - opq_start_y - 2 {
      opq_start_y + 1 // Start after separator line
    } else {
      total_height.saturating_sub(opq_items.len() + 1)
    };

    // Draw empty queue placeholder blocks for OPQ
    for y in (opq_start_y + 1)..(opq_display_start_y) {
      let placeholder_color = if y % 2 == 0 {
        ColorType::rgb(70, 70, 70)
      } else {
        ColorType::rgb(50, 50, 50)
      };
      let placeholder_style = Style::from(ColorStyle::front(placeholder_color));
      printer.with_style(placeholder_style, |printer| {
        for x in 1..QUEUE_MARGIN_RIGHT {
          printer.print((x, y), consts::QUEUE_PLACEHOLDER_SYMBOL);
        }
      });
    }

    // Draw OPQ items bottom-up: item[0] (oldest) at bottom, newest stacks upward
    for (idx, item) in opq_items.iter().enumerate() {
      if idx + 2 >= total_height {
        break;
      }
      let y = total_height - 2 - idx;
      if y < opq_start_y + 1 {
        break;
      }
      printer.with_style(style, |printer| {
        printer.print((3, y), item);
      });
    }

    // Draw vertical separator
    for y in 0..total_height {
      let placeholder_color = if y % 2 == 0 {
        ColorType::rgb(50, 50, 50)
      } else {
        ColorType::rgb(70, 70, 70)
      };
      let placeholder_style = Style::from(ColorStyle::front(placeholder_color));
      printer.with_style(placeholder_style, |printer| {
        printer.print((0, y), " ┃ ");
      });
    }

    // OPQ label at bottom of bottom half
    printer.with_style(style, |printer| {
      printer.print((3, total_height - 1), "OPRTQ");
    });

    // Pending jump position indicator
    let pending = self
      .playhead_ui
      .queue_manager
      .pending_jump_position
      .lock()
      .unwrap();
    let (pending_str, pending_color) = match *pending {
      PendingJumpPosition::Empty => (None, ColorType::rgb(60, 60, 60)),
      PendingJumpPosition::Waiting(x, y) => {
        (Some(format!("{},{}", x, y)), ColorType::rgb(100, 100, 100))
      }
      PendingJumpPosition::Armed(x, y) => {
        (Some(format!("{},{}", x, y)), ColorType::rgb(255, 255, 255))
      }
    };
    drop(pending);
    if let Some(s) = pending_str {
      let pending_style = Style::from(ColorStyle::front(pending_color));
      printer.with_style(pending_style, |printer| {
        printer.print((3, total_height), &s);
      });
    }
  }

  fn draw_queue_operators_bottom(&self, printer: &Printer) {
    if !self.show_keyboard || self.grid.width == 0 {
      return;
    }

    let style = Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)));

    printer.with_style(style, |printer| {
      for x in 0..self.grid.width {
        printer.print((x, 0), "─");
      }
    });

    let abs_active_x = self.playhead_ui.playhead_pos.x + self.playhead_ui.actived_pos.x;
    if abs_active_x < self.grid.width {
      let arrow_style = Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)));
      printer.with_style(arrow_style, |printer| {
        printer.print((abs_active_x, 0), "v");
      });
    }

    let regex_indexes = self.playhead_ui.regex_indexes.lock().unwrap();
    let is_regex_match_x = regex_indexes.iter().any(|&idx| {
      let x_pos = idx % self.grid.width;
      x_pos == abs_active_x
    });

    // Draw queue operators on row 1
    let mut x = 0;
    let mut queue_index = 0;
    while x < self.grid.width {
      // Check if this position collides with an event operator position
      let is_event_position = x % consts::EVENT_OP_SPACING == 0;
      let display_char = if is_event_position {
        "-".to_string()
      } else if !self.playhead_ui.accumulation_mode {
        " ".to_string()
      } else {
        let op = QUEUE_OPERATORS[queue_index % QUEUE_OPERATORS.len()];
        op.to_string()
      };

      let is_active = x == abs_active_x;
      let style = if is_active && is_regex_match_x {
        Style::from(ColorStyle::new(
          ColorType::rgb(0, 0, 0),
          ColorType::rgb(255, 255, 255),
        ))
      } else if is_active {
        Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)))
      } else {
        Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)))
      };
      printer.with_style(style, |printer| {
        printer.print((x, 1), &display_char);
      });

      x += consts::QUEUE_OP_SPACING;
      queue_index += 1;
    }

    // Draw event operators on row 1 (same line)
    if self.playhead_ui.event_operator_mode {
      let mut x = 0;
      let mut event_index = 0;
      while x < self.grid.width {
        let op = EVENT_OPERATORS[event_index % EVENT_OPERATORS.len()];
        let is_active = x == abs_active_x;
        let style = if is_active && is_regex_match_x {
          Style::from(ColorStyle::new(
            ColorType::rgb(0, 0, 0),
            ColorType::rgb(255, 255, 255),
          ))
        } else if is_active {
          Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)))
        } else {
          Style::from(ColorStyle::front(ColorType::rgb(150, 150, 150))) // Slightly brighter for events
        };
        printer.with_style(style, |printer| {
          printer.print((x, 1), &op.to_string());
        });

        x += consts::EVENT_OP_SPACING;
        event_index += 1;
      }
    }
  }

  pub fn build(
    playhead_tx: Sender<PlayheadMessage>,
    regex_tx: Sender<regex::Message>,
  ) -> ResizedView<ResizedView<NamedView<Canvas<GridEditor>>>> {
    let mut editor = GridEditor::new(playhead_tx);
    editor.regex_tx = Some(regex_tx);
    Canvas::new(editor)
      .with_draw(draw)
      .with_layout(layout)
      .with_on_event(on_event)
      .with_take_focus(take_focus)
      .with_name(consts::canvas_editor_section_view)
      .full_height()
      .full_width()
  }

  pub fn update_text_contents(&mut self, contents: &str) {
    let contents = contents.replace("\r\n", "\n").replace("\r", "\n");
    self.text_contents = Some(contents);
  }

  pub fn update_grid_src(&mut self) {
    if self.text_contents.as_ref().is_none() {
      return;
    };

    let cols: usize = self.grid.width;
    let rows: usize = self.grid.height;

    for y in 0..rows {
      for x in 0..cols {
        self.grid.set(x, y, '\0');
      }
    }

    let mut x = 0;
    let mut y = 0;

    // Iterate through characters, preserving newlines
    for ch in self.text_contents.as_ref().unwrap().chars() {
      if y >= rows {
        break; // Stop if we've filled all rows
      }

      if ch == '\n' {
        // Fill rest of current row with '\0' (will render as rest/dot)
        while x < cols {
          self.grid.set(x, y, '\0');
          x += 1;
        }
        x = 0;
        y += 1;
      } else {
        // Place character at current position.
        // Spaces are treated as empty cells ('\0') so they render as '.'
        // rather than an invisible blank, consistent with padding cells.
        let cell = if ch == ' ' { '\0' } else { ch };
        if x < cols {
          self.grid.set(x, y, cell);
          x += 1;

          // If we've reached end of row, move to next row
          if x >= cols {
            x = 0;
            y += 1;
          }
        }
      }
    }
  }

  fn grid_size_xy(&self) -> cursive::XY<usize> {
    (self.grid.width, self.grid.height).into()
  }

  fn start_aim_if_needed(&mut self) {
    if self.playhead_ui.aimed_area.is_none() {
      let _ = self.playhead_tx.send(PlayheadMessage::StartAim());
    }
  }

  fn update_aim(&self, dir: Direction, step: usize) -> EventResult {
    let grid_size = self.grid_size_xy();
    let _ = self
      .playhead_tx
      .send(PlayheadMessage::UpdateAim(dir, grid_size, step));
    EventResult::Ignored
  }

  fn commit_or_move(&mut self, dir: Direction) -> EventResult {
    if self.playhead_ui.aimed_area.is_some() {
      let _ = self.playhead_tx.send(PlayheadMessage::CommitAim());
      return EventResult::Ignored;
    }
    let grid_size = self.grid_size_xy();
    let _ = self.playhead_tx.send(PlayheadMessage::Move(dir, grid_size));
    EventResult::Ignored
  }

  fn alt_action(&self, dir: Direction, step: usize) -> EventResult {
    let grid_size = self.grid_size_xy();
    if self.playhead_ui.aimed_area.is_some() {
      let _ = self
        .playhead_tx
        .send(PlayheadMessage::UpdateAim(dir, grid_size, step));
    } else {
      let _ = self.playhead_tx.send(PlayheadMessage::Leap(dir, grid_size));
    }
    EventResult::Ignored
  }

  fn scale_action(&self, dir: (i32, i32)) -> EventResult {
    let _ = self.playhead_tx.send(PlayheadMessage::Scale(dir));
    EventResult::Ignored
  }

  fn handle_mouse_press(&mut self, offset: Vec2, position: Vec2) -> EventResult {
    let x_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_LEFT
    } else {
      0
    };
    let y_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_TOP
    } else {
      0
    };

    let adjusted_position = Vec2::new(
      position.x.saturating_sub(x_offset),
      position.y.saturating_sub(y_offset),
    );

    self
      .playhead_tx
      .send(PlayheadMessage::SetCurrentPos(adjusted_position, offset))
      .unwrap();
    let grid_size = (self.grid.width, self.grid.height).into();
    self
      .playhead_tx
      .send(PlayheadMessage::Move(Direction::Idle, grid_size))
      .unwrap();
    self
      .playhead_tx
      .send(PlayheadMessage::UpdateInfoStatusView())
      .unwrap();

    EventResult::consumed()
  }

  fn handle_mouse_hold(&mut self, offset: Vec2, position: Vec2) -> EventResult {
    // Adjust position to account for keyboard margins
    let x_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_LEFT
    } else {
      0
    };
    let y_offset = if self.show_keyboard {
      KEYBOARD_MARGIN_TOP
    } else {
      0
    };

    let pos_x = position.x.saturating_sub(x_offset + 1);
    let pos_y = position.y.saturating_sub(offset.y + y_offset);

    // prevent dragging above or left of the origin flips the area.
    let origin = self.playhead_ui.playhead_pos;
    let clamped_x = pos_x.max(origin.x + 1);
    let clamped_y = pos_y.max(origin.y + 1);

    self
      .playhead_tx
      .send(PlayheadMessage::SetGridArea((clamped_x, clamped_y).into()))
      .unwrap();

    EventResult::Ignored
  }

  fn handle_split_char(&mut self, c: char) -> EventResult {
    let (v, h): (usize, usize) = match c {
      '0' => (1, 1),
      '1' => (1, 1),
      '2' => (2, 1),
      '3' => (3, 1),
      '4' => (4, 1),
      '5' => (4, 2),
      '6' => (4, 3),
      '7' => (4, 4),
      _ => (1, 1),
    };
    self.playhead_ui.grid_v_splits = v;
    self.playhead_ui.grid_h_splits = h;
    let _ = self.playhead_tx.send(PlayheadMessage::SetGridSplits(v, h));

    let pos = self.playhead_ui.playhead_pos;
    let gw = self.grid.width.max(1);
    let gh = self.grid.height.max(1);
    let col_w = if v > 1 { (gw / v).max(1) } else { gw };
    let row_h = if h > 1 { (gh / h).max(1) } else { gh };
    let col_idx = (pos.x / col_w).min(v.saturating_sub(1));
    let row_idx = (pos.y / row_h).min(h.saturating_sub(1));
    let ch = row_idx * v + col_idx;
    let label = format!("{}/{}", ch + 1, v * h);

    EventResult::Consumed(Some(Callback::from_fn(move |s: &mut Cursive| {
      s.call_on_name(consts::chn_status_unit_view, |view: &mut TextView| {
        view.set_content(label.clone());
      });
    })))
  }

  pub fn resize(&mut self, size: Vec2) {
    let grid_width = if self.show_keyboard {
      size
        .x
        .saturating_sub(KEYBOARD_MARGIN_LEFT + QUEUE_MARGIN_RIGHT)
    } else {
      size.x
    };

    let grid_height = if self.show_keyboard {
      size
        .y
        .saturating_sub(KEYBOARD_MARGIN_TOP + KEYBOARD_MARGIN_BOTTOM)
    } else {
      size.y
    };

    self.grid = Matrix::new(grid_width, grid_height, '\0');
    self.size = size;
    // Update grid width and height for precise timing calculations and note mapping
    let _ = self
      .playhead_tx
      .send(PlayheadMessage::SetGridSize(grid_width, grid_height));
    // Ensure playhead stays within new bounds
    let _ = self.playhead_tx.send(PlayheadMessage::Move(
      Direction::Idle,
      (grid_width, grid_height).into(),
    ));
  }

  pub fn clear_contents(&mut self) {
    self.grid = Matrix::new(self.grid.width, self.grid.height, '\0');
  }

  pub fn text_contents(&self) -> String {
    self
      .text_contents
      .as_ref()
      .unwrap_or(&"".to_string())
      .to_string()
  }
}

fn draw(canvas: &GridEditor, printer: &Printer) {
  if canvas.show_keyboard {
    let top_keyboard_printer = printer.offset((KEYBOARD_MARGIN_LEFT, 0));
    canvas.draw_keyboard_top(&top_keyboard_printer);

    // Draw corner symbol where keyboards meet
    let root_note_top = canvas.playhead_ui.scale_root_top;
    // let style = Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)));
    // printer.with_style(style, |printer| {
    //   printer.print((0, 1), root_note_top.name());
    // });

    let scale_mode_top = canvas.playhead_ui.scale_mode_top;
    let scale_mode_left = canvas.playhead_ui.scale_mode_left;
    let scale_root_left = canvas.playhead_ui.scale_root_left;
    let active_style = Style::from(ColorStyle::front(ColorType::rgb(255, 255, 255)));
    let inactive_style = Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)));
    let (style_top, style_left) = if !canvas.is_canvas_focused {
      (inactive_style, inactive_style)
    } else if canvas.playhead_ui.keyboard_top_active {
      (active_style, inactive_style)
    } else {
      (inactive_style, active_style)
    };

    printer.with_style(style_top, |printer| {
      printer.print(
        (0, 0),
        &format!("{} {}", scale_mode_top.short_name(), root_note_top.name()),
      );
    });
    printer.with_style(style_left, |printer| {
      printer.print(
        (0, 2),
        &format!(
          "{} {}",
          scale_mode_left.short_name(),
          scale_root_left.name()
        ),
      );
    });

    let left_keyboard_printer = printer.offset((0, KEYBOARD_MARGIN_TOP));
    canvas.draw_keyboard_left(&left_keyboard_printer);

    let bottom_y = KEYBOARD_MARGIN_TOP + canvas.grid.height;
    let bottom_operators_printer = printer.offset((KEYBOARD_MARGIN_LEFT, bottom_y));
    canvas.draw_queue_operators_bottom(&bottom_operators_printer);

    // draw bottom corners
    let style = Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)));
    printer.with_style(style, |printer| {
      printer.print((KEYBOARD_MARGIN_LEFT - 2, bottom_y), "┗");
    });
    printer.with_style(style, |printer| {
      printer.print((canvas.grid.width + QUEUE_MARGIN_RIGHT + 1, bottom_y), "┛");
    });

    // Draw right queue display
    let right_x = KEYBOARD_MARGIN_LEFT + canvas.grid.width;
    let right_queue_printer = printer.offset((right_x, KEYBOARD_MARGIN_TOP));
    canvas.draw_queue_right(&right_queue_printer);
  }

  let x_offset = if canvas.show_keyboard {
    KEYBOARD_MARGIN_LEFT
  } else {
    0
  };
  let y_offset = if canvas.show_keyboard {
    KEYBOARD_MARGIN_TOP
  } else {
    0
  };
  let grid_printer = printer.offset((x_offset, y_offset));

  canvas.grid.print(&grid_printer, &canvas.playhead_ui);

  let sep_style = Style::from(ColorStyle::front(ColorType::rgb(100, 100, 100)));

  let col_w = if canvas.playhead_ui.grid_v_splits >= 2 {
    canvas.grid.width / canvas.playhead_ui.grid_v_splits
  } else {
    0
  };
  let row_h = if canvas.playhead_ui.grid_h_splits >= 2 {
    canvas.grid.height / canvas.playhead_ui.grid_h_splits
  } else {
    0
  };

  let sep_xs: Vec<usize> = if canvas.playhead_ui.grid_v_splits >= 2 {
    (1..canvas.playhead_ui.grid_v_splits)
      .map(|i| col_w * i)
      .filter(|&x| x < canvas.grid.width)
      .collect()
  } else {
    vec![]
  };

  let sep_ys: Vec<usize> = if canvas.playhead_ui.grid_h_splits >= 2 {
    (1..canvas.playhead_ui.grid_h_splits)
      .map(|i| row_h * i)
      .filter(|&y| y < canvas.grid.height)
      .collect()
  } else {
    vec![]
  };

  let playhead_area = canvas.playhead_ui.playhead_area;

  for &sep_x in &sep_xs {
    for y in 0..canvas.grid.height {
      if playhead_area.contains((sep_x, y).into()) {
        continue;
      }
      let ch = if sep_ys.contains(&y) { "┼" } else { "│" };
      grid_printer.with_style(sep_style, |printer| {
        printer.print((sep_x, y), ch);
      });
    }
  }

  for &sep_y in &sep_ys {
    for x in 0..canvas.grid.width {
      if playhead_area.contains((x, sep_y).into()) {
        continue;
      }
      let ch = if sep_xs.contains(&x) { "┼" } else { "─" };
      grid_printer.with_style(sep_style, |printer| {
        printer.print((x, sep_y), ch);
      });
    }
  }

  // Canvas focus indicator: shown at top-left margin (row 2, free space)
  if canvas.show_keyboard {
    let keyboard_label = if canvas.playhead_ui.keyboard_top_active {
      "[  \u{2227}  ]"
    } else {
      "[  \u{2228}  ]"
    };
    let color = if canvas.is_canvas_focused {
      ColorType::rgb(200, 200, 200)
    } else {
      ColorType::rgb(100, 100, 100)
    };
    let style = Style::from(ColorStyle::front(color));
    printer.with_style(style, |printer| {
      printer.print((0, 1), keyboard_label);
    });
  }
}

fn layout(canvas: &mut GridEditor, size: Vec2) {
  if canvas.size != size {
    canvas.resize(size);
    if canvas.text_contents.is_some() {
      canvas.update_grid_src();
      if !canvas.last_pattern.is_empty() {
        if let Some(ref tx) = canvas.regex_tx {
          let input_regex = regex::EventData {
            text: canvas.text_contents(),
            pattern: canvas.last_pattern.clone(),
            flags: canvas.last_flag_str.clone(),
            grid_width: canvas.grid.width,
          };
          let _ = tx.send(regex::Message::Solve(input_regex));
        }
      }
    }
  }
}

fn take_focus(
  _: &mut GridEditor,
  _: cursive::direction::Direction,
) -> Result<EventResult, CannotFocus> {
  Ok(EventResult::Consumed(None))
}

fn on_event(canvas: &mut GridEditor, event: Event) -> EventResult {
  match event {
    Event::Refresh => EventResult::consumed(),

    // handle split midi-chan
    Event::Char(c) if ('1'..='7').contains(&c) => canvas.handle_split_char(c),

    // Vim keybindings for movement (h/j/k/l)
    Event::Char('h') => canvas.commit_or_move(Direction::Left),
    Event::Char('j') => canvas.commit_or_move(Direction::Down),
    Event::Char('k') => canvas.commit_or_move(Direction::Up),
    Event::Char('l') => canvas.commit_or_move(Direction::Right),
    Event::AltChar('h') => canvas.alt_action(Direction::Left, consts::MOVE_X_STEP_SIZE),
    Event::AltChar('j') => canvas.alt_action(Direction::Down, consts::MOVE_Y_STEP_SIZE),
    Event::AltChar('k') => canvas.alt_action(Direction::Up, consts::MOVE_Y_STEP_SIZE),
    Event::AltChar('l') => canvas.alt_action(Direction::Right, consts::MOVE_X_STEP_SIZE),

    // Option+h
    #[cfg(target_os = "macos")]
    Event::Char('˙') => canvas.alt_action(Direction::Left, consts::MOVE_X_STEP_SIZE),
    // Option+j
    #[cfg(target_os = "macos")]
    Event::Char('∆') => canvas.alt_action(Direction::Down, consts::MOVE_Y_STEP_SIZE),
    // Option+k
    #[cfg(target_os = "macos")]
    Event::Char('˚') => canvas.alt_action(Direction::Up, consts::MOVE_Y_STEP_SIZE),
    // Option+l
    #[cfg(target_os = "macos")]
    Event::Char('¬') => canvas.alt_action(Direction::Right, consts::MOVE_X_STEP_SIZE),

    // Shift + Vim keybindings for movement (h/j/k/l)
    Event::Char('H') => canvas.scale_action((-1, 0)),
    Event::Char('J') => canvas.scale_action((0, -1)),
    Event::Char('K') => canvas.scale_action((0, 1)),
    Event::Char('L') => canvas.scale_action((1, 0)),
    #[cfg(target_os = "macos")]
    Event::Char('Ó') => canvas.scale_action((-(consts::SCALE_X_STEP_SIZE as i32), 0)),
    #[cfg(target_os = "macos")]
    Event::Char('Ô') => canvas.scale_action((0, -(consts::SCALE_Y_STEP_SIZE as i32))),
    #[cfg(target_os = "macos")]
    Event::Char('\u{f8ff}') => canvas.scale_action((0, consts::SCALE_Y_STEP_SIZE as i32)),
    #[cfg(target_os = "macos")]
    Event::Char('Ò') => canvas.scale_action((consts::SCALE_X_STEP_SIZE as i32, 0)),
    Event::CtrlChar('h') => {
      canvas.start_aim_if_needed();
      canvas.update_aim(Direction::Left, 1)
    }
    Event::CtrlChar('j') => {
      canvas.start_aim_if_needed();
      canvas.update_aim(Direction::Down, 1)
    }
    Event::CtrlChar('k') => {
      canvas.start_aim_if_needed();
      canvas.update_aim(Direction::Up, 1)
    }
    Event::CtrlChar('l') => {
      canvas.start_aim_if_needed();
      canvas.update_aim(Direction::Right, 1)
    }
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Press(_btn),
    } => canvas.handle_mouse_press(offset, position),
    Event::Mouse {
      offset,
      position,
      event: MouseEvent::Hold(MouseButton::Left),
    } => canvas.handle_mouse_hold(offset, position),
    _ => EventResult::Ignored,
  }
}
