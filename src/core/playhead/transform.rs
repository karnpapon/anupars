use std::sync::atomic::Ordering;

use cursive::Vec2;
use cursive::XY;

use crate::core::utils;
use crate::view::rect::Rect;

use std::collections::HashMap;

use crate::core::engine::regex;

use super::movement::Movement;
use super::{Playhead, UIUpdate};

use super::Direction;

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
    let _ = self.ui_tx.send(UIUpdate::PlayheadPosAndArea(pos, area));
    let _ = self.ui_tx.send(UIUpdate::InputStatus("-".to_string()));
    let _ = self.ui_tx.send(UIUpdate::ChnStatus(chn_str));
    self.enqueue_sym_space();
  }

  pub fn set_actived_pos(&self, pos: usize) {
    let area = self.area.lock().unwrap();
    let playhead_w = area.width();
    let playhead_h = area.height();
    let playhead_x = area.left();
    let playhead_y = area.top();
    drop(area);

    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);
    self
      .position_calc
      .grid_height
      .store(self.grid.height.load(Ordering::Relaxed), Ordering::Relaxed);
    self.position_calc.arpeggiator_mode.store(
      self.modes.arpeggiator_mode.load(Ordering::Relaxed),
      Ordering::Relaxed,
    );

    let new_pos = self
      .position_calc
      .calculate_actived_pos(pos, playhead_x, playhead_y, playhead_w, playhead_h);

    let mut actived_pos = self.actived_pos.lock().unwrap();
    *actived_pos = new_pos;
  }

  /// Get absolute position the same way it's displayed in the console "POS: (x,y)" and flattened index.
  pub(super) fn calculate_absolute_position(&self, active_pos: Vec2) -> (usize, usize, usize) {
    let pos = self.pos.lock().unwrap();
    let playhead_pos = *pos;
    drop(pos);

    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);
    self
      .position_calc
      .calculate_absolute_position(playhead_pos, active_pos)
  }

  /// Find distance to the next closest trigger position within the playhead area.
  pub(super) fn find_distance_to_next_trigger(&self, curr_pos: usize) -> usize {
    let area = self.area.lock().unwrap();
    let playhead_x = area.left();
    let playhead_y = area.top();
    let playhead_width = area.width();
    let playhead_height = area.height();
    drop(area);

    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);

    self.position_calc.find_distance_to_next_trigger(
      curr_pos,
      playhead_x,
      playhead_y,
      playhead_width,
      playhead_height,
    )
  }

  pub(super) fn check_contains(&self, area: &Rect, matcher: &HashMap<usize, regex::Match>) -> bool {
    for dy in 0..area.height() {
      for dx in 0..area.width() {
        let x = area.top_left.x + dx;
        let y = area.top_left.y + dy;
        let pos_index = y * self.grid.width.load(Ordering::Relaxed) + x;
        if matcher.contains_key(&pos_index) {
          return true;
        }
      }
    }
    false
  }

  pub(super) fn update_active_pos_ui(&self, active_pos: Vec2) {
    let _ = self.ui_tx.send(UIUpdate::ActivePos(active_pos));
  }

  pub(super) fn handle_set_active_pos(&self, tick: usize) {
    let ratio = self.music.ratio.lock().unwrap();
    // Clock fires 16 ticks/beat (64 per 4-beat bar). Divider table:
    //   ratio.1=1  → every 64 ticks (DIV 1)    ratio.1=16 → every  4 ticks (DIV 16)
    //   ratio.1=32 → every  2 ticks (DIV 32)   ratio.1=64 → every  1 tick  (DIV 64)
    let base_divider = (64 / ratio.1).max(1);
    drop(ratio);

    if self.modes.freeze_mode.load(Ordering::Relaxed) {
      if tick.is_multiple_of(base_divider) && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        let active_pos = *self.frozen_active_pos.lock().unwrap();
        let going_forward = self.is_going_forward();
        let (abs_x, abs_y, curr_running_playhead) = self.calculate_absolute_position(active_pos);
        let (note_position, scale_mode) = self
          .midi_handler
          .determine_note_position_and_scale(active_pos, abs_x, abs_y);

        let matcher_guard = self.text_matcher.lock().unwrap();
        let match_at_pos = if going_forward {
          matcher_guard
            .as_ref()
            .and_then(|m| m.get(&curr_running_playhead))
            .cloned()
        } else {
          matcher_guard.as_ref().and_then(|m| {
            m.values()
              .find(|mv| curr_running_playhead == mv.i + mv.l.max(1) - 1)
              .cloned()
          })
        };
        drop(matcher_guard);
        let has_match = match_at_pos.is_some();
        let match_len = match_at_pos.as_ref().map(|m| m.l).unwrap_or(1).max(1);

        if has_match {
          self.midi_handler.trigger_midi_if_matched(
            curr_running_playhead,
            note_position,
            scale_mode,
            abs_x,
            abs_y,
            match_len,
          );
        }

        self.update_active_pos_ui(active_pos);
      }
      return;
    }

    let going_forward = self.is_going_forward();
    // Inside a match span: speed up when going forward (halve divider),
    // slow down when going reverse (double divider).
    let in_match_span = self.match_span_remaining.load(Ordering::Relaxed) > 0;
    let divider = if in_match_span {
      if going_forward {
        (base_divider / 2).max(1)
      } else {
        base_divider * 2
      }
    } else {
      base_divider
    };

    let should_advance =
      tick.is_multiple_of(divider) && !self.modes.is_ratcheting.load(Ordering::Relaxed);
    if should_advance {
      if in_match_span {
        self.match_span_remaining.fetch_sub(1, Ordering::Relaxed);
      }

      let mut step_idx = self.step_index.lock().unwrap();
      *step_idx += 1;
      self.set_actived_pos(*step_idx);
      drop(step_idx);

      let active_pos_mutex = self.actived_pos.lock().unwrap();
      let mut active_pos = *active_pos_mutex;
      drop(active_pos_mutex);

      let (abs_x, abs_y, curr_running_playhead) = self.calculate_absolute_position(active_pos);

      let (note_position, scale_mode) = self
        .midi_handler
        .determine_note_position_and_scale(active_pos, abs_x, abs_y);

      let mut did_jump = false;

      // Capture the full Match for the current cell.
      // Forward: trigger at the first cell (m.i == curr).
      // Reverse/Pendulum-reverse: trigger at the last cell (m.i + m.l - 1 == curr).
      let matcher_guard = self.text_matcher.lock().unwrap();
      let match_at_pos = if going_forward {
        matcher_guard
          .as_ref()
          .and_then(|m| m.get(&curr_running_playhead))
          .cloned()
      } else {
        matcher_guard.as_ref().and_then(|m| {
          m.values()
            .find(|mv| curr_running_playhead == mv.i + mv.l.max(1) - 1)
            .cloned()
        })
      };
      drop(matcher_guard);

      let has_match = match_at_pos.is_some();
      let match_len = match_at_pos.as_ref().map(|m| m.l).unwrap_or(1).max(1);

      // Retrigger on every fast step inside a span (cells 2, 3, … of a match word)
      if in_match_span && !has_match && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        self.midi_handler.trigger_midi_if_matched(
          curr_running_playhead,
          note_position,
          scale_mode,
          abs_x,
          abs_y,
          1,
        );
        self.enqueue_sym_buf_append(curr_running_playhead);
      }

      if has_match {
        if self.modes.accumulation_mode.load(Ordering::Relaxed) {
          if let Some(new_active_pos) = self.handle_accumulation_mode(abs_x) {
            active_pos = new_active_pos;
            did_jump = true;
          }
        }
        if !did_jump && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
          let distance_to_next = if self.modes.dyn_length_mode.load(Ordering::Relaxed)
            && !self.modes.arpeggiator_mode.load(Ordering::Relaxed)
          {
            self.find_distance_to_next_trigger(curr_running_playhead)
          } else {
            match_len
          };

          self.midi_handler.trigger_midi_if_matched(
            curr_running_playhead,
            note_position,
            scale_mode,
            abs_x,
            abs_y,
            distance_to_next,
          );
          self.enqueue_sym_buf_append(curr_running_playhead);
        }
        if self.modes.hold_next_note.load(Ordering::Relaxed) {
          self.modes.hold_next_note.store(false, Ordering::Relaxed);
        }
        // Start fast-stepping for remaining cells in the match span
        if match_len > 1 {
          self
            .match_span_remaining
            .store(match_len - 1, Ordering::Relaxed);
        }
      }

      let matcher_guard = self.text_matcher.lock().unwrap();
      if let Some(ref m) = *matcher_guard {
        self.handle_silent_step(m);
      }
      drop(matcher_guard);

      if !did_jump && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        let area = *self.area.lock().unwrap();
        let area_right = area.right();
        let active_pos_y = self.pos.lock().unwrap().y;

        // Compute the sweep crosshair x based on active sweep movement mode.
        let sweep_mv = *self.modes.sweep_movement.lock().unwrap();
        let sweep_abs_x = if let Some(mv) = sweep_mv {
          let area_width = area.width();
          let step = *self.step_index.lock().unwrap();
          // Sweep direction is absolute (computed from step_index), independent of normal movement.
          let sweep_rel = match mv {
            // Forward: always 0→W-1 regardless of normal movement.
            Movement::Forward => step % area_width.max(1),
            // Reverse: always W-1→0 regardless of normal movement.
            Movement::Reverse => area_width
              .saturating_sub(1)
              .saturating_sub(step % area_width.max(1)),
            Movement::Random => {
              crate::core::playhead::movement::get_random_index(step, area_width.max(1))
            }
            Movement::Pendulum => {
              // Reversed pendulum: starts at W-1, goes W-1→0→W-1→…
              let cycle = if area_width <= 1 {
                1
              } else {
                area_width * 2 - 2
              };
              let phase = step % cycle;
              if phase < area_width {
                area_width.saturating_sub(1).saturating_sub(phase)
              } else {
                phase.saturating_sub(area_width.saturating_sub(1))
              }
            }
          };
          let x = area.left() + sweep_rel.min(area_width.saturating_sub(1));
          self.modes.sweep_x.store(x, Ordering::Relaxed);
          let _ = self.ui_tx.send(UIUpdate::SweepX(x));
          x
        } else {
          abs_x
        };

        // Exclude the (note_pos, channel) that the playhead already triggered from sweep.
        let playhead_triggered = (in_match_span && !has_match) || (has_match && !did_jump);
        let exclude = if playhead_triggered {
          let grid_width = self.grid.width.load(Ordering::Relaxed);
          let grid_height = self.grid.height.load(Ordering::Relaxed);
          let channel = crate::core::playhead::midi::calculate_channel(
            abs_x,
            abs_y,
            grid_width,
            grid_height,
            self.grid.v_splits.load(Ordering::Relaxed),
            self.grid.h_splits.load(Ordering::Relaxed),
          );
          Some((note_position, channel))
        } else {
          None
        };
        // LFO phase [0.0, 1.0) derived from sweep position within playhead area.
        // Waveform shape comes for free from sweep movement type:
        //   Forward = sawtooth up, Reverse = sawtooth down,
        //   Pendulum = triangle,    Random  = S&H
        let cc_value = {
          let area_left = area.left() as f32;
          let area_w = area.width().max(1) as f32;
          let phase = ((sweep_abs_x as f32 - area_left) / area_w).clamp(0.0, 1.0);
          (phase * 127.0) as u8
        };
        self.midi_handler.trigger_midi_if_matched_sweep(
          curr_running_playhead,
          sweep_abs_x,
          active_pos_y,
          area_right,
          exclude,
          cc_value,
        );

        for idx in self.midi_handler.sweep_matched_indexes(
          curr_running_playhead,
          sweep_abs_x,
          area_right,
          exclude,
        ) {
          self.enqueue_sym_buf_append(idx);
        }
      }
      self.update_active_pos_ui(active_pos);
    }
  }

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

    let pos = *self.pos.lock().unwrap();
    let _ = self.ui_tx.send(UIUpdate::InputStatus("-".to_string()));
    let _ = self.ui_tx.send(UIUpdate::CanvasPlayheadPos(pos));
    self.enqueue_sym_space();
  }

  pub(super) fn handle_set_grid_area(&self, current_pos: XY<usize>) {
    self.set_grid_area(current_pos);
    self.reset_accumulation_counter();

    let area = self.area.lock().unwrap();
    let w = area.width();
    let h = area.height();
    let playhead_area = *area;
    drop(area);

    let _ = self.ui_tx.send(UIUpdate::InputStatus("-".to_string()));
    let _ = self.ui_tx.send(UIUpdate::LenStatus(utils::build_len_status_str((w, h))));
    let _ = self.ui_tx.send(UIUpdate::CanvasPlayheadArea(playhead_area));
  }

  pub(super) fn handle_scale(&self, size: (i32, i32)) {
    self.scale(size);
    self.reset_accumulation_counter();

    let area = self.area.lock().unwrap();
    let playhead_area = *area;
    let area_size = area.size();
    drop(area);

    let _ = self.ui_tx.send(UIUpdate::InputStatus("-".to_string()));
    let _ = self.ui_tx.send(UIUpdate::LenStatus(utils::build_len_status_str((area_size.x, area_size.y))));
    let _ = self.ui_tx.send(UIUpdate::CanvasPlayheadArea(playhead_area));
  }
}

#[cfg(test)]
mod tests {
  use super::super::test_helpers::make_playhead;
  use super::*;

  #[test]
  fn set_leap_moves_left() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(5, 3);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(5, 3), (2, 2));
    ph.set_leap(Direction::Left, 2, Vec2::new(20, 20));
    assert_eq!(ph.pos.lock().unwrap().x, 3);
  }

  #[test]
  fn set_leap_moves_up() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(3, 5);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(3, 5), (2, 2));
    ph.set_leap(Direction::Up, 3, Vec2::new(20, 20));
    assert_eq!(ph.pos.lock().unwrap().y, 2);
  }

  #[test]
  fn set_leap_moves_down() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(0, 0);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (1, 1));
    ph.set_leap(Direction::Down, 4, Vec2::new(20, 20));
    assert_eq!(ph.pos.lock().unwrap().y, 4);
  }

  #[test]
  fn set_leap_saturates_at_origin_when_moving_left() {
    // pos.x = 0, moving left → saturating_add_signed(-1) = 0, pos unchanged
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(0, 3);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(0, 3), (1, 1));
    ph.set_leap(Direction::Left, 1, Vec2::new(20, 20));
    assert_eq!(ph.pos.lock().unwrap().x, 0);
  }

  #[test]
  fn set_leap_multi_step_moves_by_exact_amount() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(0, 0);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (1, 1));
    ph.set_leap(Direction::Right, 8, Vec2::new(20, 20));
    assert_eq!(ph.pos.lock().unwrap().x, 8);
  }

  #[test]
  fn set_leap_rejects_down_when_area_bottom_would_exceed_canvas() {
    // pos=(0,17), area h=3 → bottom_right.y = 17+3-1=19 fits; one more step → 20 doesn't fit
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(0, 17);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(0, 17), (1, 3));
    let canvas = Vec2::new(20, 20);
    ph.set_leap(Direction::Down, 1, canvas);
    assert_eq!(ph.pos.lock().unwrap().y, 17, "move should be rejected");
  }

  #[test]
  fn set_leap_idle_direction_leaves_pos_unchanged() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(5, 5);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(5, 5), (2, 2));
    ph.set_leap(Direction::Idle, 1, Vec2::new(20, 20));
    let pos = *ph.pos.lock().unwrap();
    assert_eq!(pos, Vec2::new(5, 5));
  }

  #[test]
  fn move_to_cursor_left_of_pos_anchors_area_at_cursor() {
    // pos=(5,2), cursor=(2,5): cursor is to the LEFT → area starts at cursor.x
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(5, 2);
    ph.move_to((2, 5).into());
    let area = *ph.area.lock().unwrap();
    assert_eq!(area.top_left().x, 2, "area should anchor at cursor x");
    assert_eq!(area.width(), 3, "|5-2|=3");
    assert_eq!(area.height(), 3, "|2-5|=3");
  }

  #[test]
  fn move_to_width_is_always_at_least_one() {
    // cursor.x == pos.x → abs_diff = 0 → clamp(1, MAX) = 1
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(4, 4);
    ph.move_to((4, 6).into());
    let area = *ph.area.lock().unwrap();
    assert_eq!(area.width(), 1);
    assert_eq!(area.height(), 2);
  }

  #[test]
  fn scale_only_width_leaves_height_unchanged() {
    let ph = make_playhead();
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (4, 4));
    ph.scale((3, 0));
    let area = ph.area.lock().unwrap();
    assert_eq!(area.width(), 7);
    assert_eq!(area.height(), 4);
  }

  #[test]
  fn scale_only_height_leaves_width_unchanged() {
    // height grows when h is negative (subtract negative = add)
    let ph = make_playhead();
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (4, 4));
    ph.scale((0, -3));
    let area = ph.area.lock().unwrap();
    assert_eq!(area.width(), 4);
    assert_eq!(area.height(), 7);
  }

  #[test]
  fn scale_shrink_width_clamps_at_one() {
    let ph = make_playhead();
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (2, 2));
    ph.scale((-99, 0));
    let area = ph.area.lock().unwrap();
    assert_eq!(area.width(), 1);
  }

  #[test]
  fn set_grid_area_stores_drag_start_at_area_top_left() {
    use std::sync::atomic::Ordering;
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(2, 3);
    ph.set_grid_area((6, 7).into());
    assert_eq!(ph.drag_start_x.load(Ordering::Relaxed), 2);
    assert_eq!(ph.drag_start_y.load(Ordering::Relaxed), 3);
  }

  #[test]
  fn check_contains_returns_true_when_match_is_inside_area() {
    use std::sync::atomic::Ordering;
    let ph = make_playhead();
    ph.grid.width.store(10, Ordering::Relaxed);
    let area = Rect::from_size(Vec2::new(2, 1), (2, 1));
    let mut matcher: HashMap<usize, regex::Match> = HashMap::new();
    matcher.insert(12, regex::Match::make_test(12, 1));
    assert!(ph.check_contains(&area, &matcher));
  }

  #[test]
  fn check_contains_returns_false_when_match_is_outside_area() {
    use std::sync::atomic::Ordering;
    let ph = make_playhead();
    ph.grid.width.store(10, Ordering::Relaxed);
    let area = Rect::from_size(Vec2::new(2, 1), (2, 1));
    let mut matcher: HashMap<usize, regex::Match> = HashMap::new();
    matcher.insert(5, regex::Match::make_test(5, 1));
    assert!(!ph.check_contains(&area, &matcher));
  }

  #[test]
  fn set_leap_moves_pos_and_keeps_area_size() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(2, 2);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(2, 2), (3, 2));
    ph.set_leap(Direction::Right, 2, Vec2::new(20, 20));
    let pos = *ph.pos.lock().unwrap();
    let area = *ph.area.lock().unwrap();
    assert_eq!(pos, Vec2::new(4, 2));
    assert_eq!(area.width(), 3);
    assert_eq!(area.height(), 2);
    assert_eq!(area.top_left(), Vec2::new(4, 2));
  }

  #[test]
  fn set_leap_rejects_move_that_would_exceed_canvas() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(18, 0);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(18, 0), (3, 1)); // right edge at 20
    ph.set_leap(Direction::Right, 1, Vec2::new(20, 20));
    assert_eq!(ph.pos.lock().unwrap().x, 18);
  }

  #[test]
  fn move_to_sets_area_from_pos_to_cursor() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(2, 3);
    ph.move_to((6, 7).into());
    let area = *ph.area.lock().unwrap();
    assert_eq!(area.width(), 4);
    assert_eq!(area.height(), 4);
    assert_eq!(area.top_left(), Vec2::new(2, 3));
  }

  #[test]
  fn move_to_cursor_same_as_pos_gives_1x1() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(5, 5);
    ph.move_to((5, 5).into());
    let area = *ph.area.lock().unwrap();
    assert_eq!(area.width(), 1);
    assert_eq!(area.height(), 1);
  }

  #[test]
  fn scale_increases_width_and_height() {
    let ph = make_playhead();
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (4, 4));
    ph.scale((2, -2));
    let area = ph.area.lock().unwrap();
    assert_eq!(area.width(), 6);
    assert_eq!(area.height(), 6);
  }

  #[test]
  fn scale_clamps_to_minimum_1x1() {
    let ph = make_playhead();
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (1, 1));
    ph.scale((-99, 99));
    let area = ph.area.lock().unwrap();
    assert_eq!(area.width(), 1);
    assert_eq!(area.height(), 1);
  }

  #[test]
  fn set_current_pos_applies_offset_correctly() {
    let ph = make_playhead();
    ph.set_current_pos((5, 3).into(), (0, 1).into());
    let pos = *ph.pos.lock().unwrap();
    assert_eq!(pos.x, 4);
    assert_eq!(pos.y, 2);
  }

  #[test]
  fn set_current_pos_at_origin_stays_at_zero() {
    let ph = make_playhead();
    ph.set_current_pos((1, 1).into(), (0, 1).into());
    let pos = *ph.pos.lock().unwrap();
    assert_eq!(pos.x, 0);
    assert_eq!(pos.y, 0);
  }
}
