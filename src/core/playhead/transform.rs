use std::sync::atomic::Ordering;

use cursive::views::{Canvas, TextView};
use cursive::Vec2;
use cursive::XY;

use crate::core::{consts, utils};
use crate::view::grid::GridEditor;
use crate::view::rect::Rect;

use super::Direction;
use super::Playhead;

impl Playhead {
  pub(super) fn handle_movement(&self, direction: Direction, steps: usize, canvas_size: Vec2) {
    self.ratchet_generation.fetch_add(1, Ordering::SeqCst);
    self.modes.is_ratcheting.store(false, Ordering::Relaxed);
    self.match_span_remaining.store(0, Ordering::Relaxed);
    self.set_leap(direction, steps, canvas_size);
    self.reset_accumulation_counter();

    let pos_mutex = self.pos.lock().unwrap();
    let pos = *pos_mutex;
    drop(pos_mutex);

    let area_mutex = self.area.lock().unwrap();
    let area = *area_mutex;
    drop(area_mutex);

    let chn_str = self.compute_chn_str(pos);
    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_pos_status_str(pos));
        });
        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });
        siv.call_on_name(consts::chn_status_unit_view, |view: &mut TextView| {
          view.set_content(chn_str);
        });
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.playhead_pos = pos;
            editor.playhead_ui.playhead_area = area;
          },
        );
      }))
      .unwrap();
    self.enqueue_sym_space();
  }

  // pub(super) fn set_move(&self, direction: Direction, canvas_size: Vec2) {
  //   self.set_leap(direction, 1, canvas_size);
  // }

  pub(super) fn set_leap(&self, direction: Direction, steps: usize, canvas_size: Vec2) {
    let mut pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();
    let (dx, dy) = direction.get_direction();

    let leap_x = dx * steps as i32;
    let leap_y = dy * steps as i32;

    let next_pos = Vec2::new(
      pos.x.saturating_add_signed(leap_x as isize),
      pos.y.saturating_add_signed(leap_y as isize),
    );
    let next_pos_bottom_right: Vec2 = (
      next_pos.x + area.width() - 1,
      next_pos.y + area.height() - 1,
    )
      .into();

    if !next_pos_bottom_right.fits_in_rect(Vec2::ZERO, canvas_size) {
      return;
    }

    *pos = next_pos;

    let w = area.width();
    let h = area.height();

    *area = Rect::from_size(next_pos, (w, h));
  }

  pub fn set_current_pos(&self, pos: XY<usize>, offset: XY<usize>) {
    let mut mutex_pos = self.pos.lock().unwrap();
    let pos_x = pos.x.abs_diff(1);
    let pos_y = pos.y.abs_diff(offset.y);
    *mutex_pos = (pos_x, pos_y).into();
  }

  pub fn move_to(&self, current_pos: XY<usize>) {
    let pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();

    let new_w = current_pos.x.abs_diff(pos.x).clamp(1, usize::MAX);
    let new_h = current_pos.y.abs_diff(pos.y).clamp(1, usize::MAX);
    let new_x = match current_pos.x.saturating_sub(pos.x) == 0 {
      true => current_pos.x,
      false => pos.x,
    };

    let new_y = match current_pos.y.saturating_sub(pos.y) == 0 {
      true => current_pos.y,
      false => pos.y,
    };

    *area = Rect::from_size((new_x, new_y), (new_w, new_h));
  }

  pub fn set_grid_area(&self, current_pos: XY<usize>) {
    self.move_to(current_pos);

    let area = self.area.lock().unwrap();
    let top_left = area.top_left();

    self.drag_start_x.store(top_left.x, Ordering::Relaxed);
    self.drag_start_y.store(top_left.y, Ordering::Relaxed);
  }

  pub fn scale(&self, (w, h): (i32, i32)) {
    let pos = self.pos.lock().unwrap();
    let mut area = self.area.lock().unwrap();

    let new_width = ((area.width() as i32) + w).max(1);
    let new_height = ((area.height() as i32) - h).max(1);

    *area = Rect::from_size(*pos, (new_width, new_height));
  }

  pub(super) fn handle_set_current_pos(&self, position: XY<usize>, offset: XY<usize>) {
    self.ratchet_generation.fetch_add(1, Ordering::SeqCst);
    self.modes.is_ratcheting.store(false, Ordering::Relaxed);
    self.match_span_remaining.store(0, Ordering::Relaxed);
    self.set_current_pos(position, offset);
    self.reset_accumulation_counter();

    let mutex_pos = self.pos.lock().unwrap();
    let pos = *mutex_pos;
    drop(mutex_pos);
    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.playhead_pos = pos;
          },
        );
      }))
      .unwrap();
    self.enqueue_sym_space();
  }

  pub(super) fn handle_set_grid_area(&self, current_pos: XY<usize>) {
    self.set_grid_area(current_pos);
    self.reset_accumulation_counter();

    let area = self.area.lock().unwrap();
    let w = area.width();
    let h = area.height();
    let playhead_area = *area;
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str((w, h)));
        });

        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.playhead_area = playhead_area;
          },
        );
      }))
      .unwrap();
  }

  pub(super) fn handle_scale(&self, size: (i32, i32)) {
    self.scale(size);
    self.reset_accumulation_counter();

    let area = self.area.lock().unwrap();
    let playhead_area = *area;
    let area_size = area.size();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
          view.set_content(utils::build_len_status_str((area_size.x, area_size.y)));
        });

        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.playhead_area = playhead_area;
          },
        );
      }))
      .unwrap();
  }
}
