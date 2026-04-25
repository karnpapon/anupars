use std::collections::HashMap;
use std::sync::atomic::Ordering;

use cursive::Vec2;

use crate::core::consts;
use crate::core::engine::regex;
use crate::core::playhead::position::PositionCalculator;
use crate::core::playhead::queue::{PendingJumpPosition, QueueItem};
use crate::view::rect::Rect;

use super::{Playhead, UIUpdate};

struct GridParams {
  pub playhead_width: usize,
  pub playhead_height: usize,
  pub grid_width: usize,
  pub grid_height: usize,
}

impl Playhead {
  pub fn reset_accumulation_counter(&self) {
    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter = 0;
  }

  pub(super) fn update_accumulation_ui(&self, count: usize, total: usize) {
    let _ = self.ui_tx.send(UIUpdate::AccumulationCounter(count, total));
  }

  pub(super) fn handle_silent_step(&self, matcher: &HashMap<usize, regex::Match>) {
    if !self.modes.accumulation_mode.load(Ordering::Relaxed) {
      return;
    }
    let area = self.area.lock().unwrap();
    let has_some_pos = self.check_contains(&area, matcher);
    let playhead_area_size = area.width() * area.height();
    drop(area);

    if has_some_pos {
      return;
    };

    let clock_enabled = consts::CLOCK_ENABLED.load(Ordering::Relaxed);
    #[allow(clippy::if_same_then_else)]
    let counter_limit = if clock_enabled {
      playhead_area_size
    } else {
      playhead_area_size
    };

    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter += 1;
    let current_count = *counter;
    self.update_accumulation_ui(current_count, counter_limit);
    if *counter >= counter_limit {
      *counter = 0;
      drop(counter);
      self.update_accumulation_ui(0, counter_limit);
      self.perform_accumulation_jump();
    } else {
      drop(counter);
      self.update_accumulation_ui(current_count, counter_limit);
    }
  }

  pub(super) fn handle_accumulation_mode(&self, abs_x: usize) -> Option<Vec2> {
    use crate::core::playhead::queue::{QueueOperator, QUEUE_OPERATORS};

    self.queue_manager.check_and_execute_operators(
      abs_x,
      self.modes.event_operator_mode.load(Ordering::Relaxed),
    );

    if abs_x.is_multiple_of(consts::QUEUE_OP_SPACING)
      && !abs_x.is_multiple_of(consts::EVENT_OP_SPACING)
      && !self.queue_manager.is_drain_queue_mode()
    {
      let position_index = abs_x / consts::QUEUE_OP_SPACING;
      let operator_index = position_index % QUEUE_OPERATORS.len();
      if QUEUE_OPERATORS[operator_index] == QueueOperator::Push {
        let playhead_pos = self.pos.lock().unwrap();
        let push_pos = (playhead_pos.x, playhead_pos.y);
        drop(playhead_pos);
        self.queue_manager.handle_push(push_pos);
      }
    }

    if abs_x.is_multiple_of(consts::QUEUE_OP_SPACING) {
      let position_index = abs_x / consts::QUEUE_OP_SPACING;
      let operator_index = position_index % QUEUE_OPERATORS.len();
      if QUEUE_OPERATORS[operator_index] == QueueOperator::Pop {
        let front = self.queue_manager.get_front_item();
        match front {
          Some(QueueItem::Event(_)) => {
            self.execute_front_op_in_queue();
          }
          Some(QueueItem::Position(x, y)) => {
            self
              .queue_manager
              .set_pending_jump(PendingJumpPosition::Waiting(x, y));
          }
          None => {}
        }
      }
    }

    let area = self.area.lock().unwrap();
    let playhead_area_size = area.width() * area.height();
    drop(area);

    let clock_enabled = consts::CLOCK_ENABLED.load(Ordering::Relaxed);
    #[allow(clippy::if_same_then_else)]
    let counter_limit = if clock_enabled {
      playhead_area_size
    } else {
      playhead_area_size
    };

    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter += 1;
    let current_count = *counter;

    if *counter >= counter_limit {
      *counter = 0;
      drop(counter);

      self.update_accumulation_ui(0, counter_limit);
      Some(self.perform_accumulation_jump())
    } else {
      drop(counter);
      self.update_accumulation_ui(current_count, counter_limit);
      None
    }
  }

  pub(super) fn perform_accumulation_jump(&self) -> Vec2 {
    let area = self.area.lock().unwrap();
    let playhead_width = area.width();
    let playhead_height = area.height();
    drop(area);

    let grid_width = self.grid.width.load(Ordering::Relaxed);
    let grid_height = self.grid.height.load(Ordering::Relaxed);

    let current_playhead_pos = {
      let pos = self.pos.lock().unwrap();
      (pos.x, pos.y)
    };

    let (new_x, new_y) = {
      match self.queue_manager.peek_pending_jump() {
        PendingJumpPosition::Armed(x, y) => {
          self
            .queue_manager
            .set_pending_jump(PendingJumpPosition::Empty);
          (x, y)
        }
        PendingJumpPosition::Waiting(_x, _y) => {
          self.queue_manager.promote_waiting_to_armed();
          let params = GridParams {
            playhead_width,
            playhead_height,
            grid_width,
            grid_height,
          };
          self.get_jump_position(current_playhead_pos, params)
        }
        PendingJumpPosition::Empty => {
          let params = GridParams {
            playhead_width,
            playhead_height,
            grid_width,
            grid_height,
          };
          self.get_jump_position(current_playhead_pos, params)
        }
      }
    };

    self.ratchet_generation.fetch_add(1, Ordering::SeqCst);
    self.modes.is_ratcheting.store(false, Ordering::Relaxed);

    let mut pos = self.pos.lock().unwrap();
    pos.x = new_x;
    pos.y = new_y;
    let new_pos = *pos;
    drop(pos);

    let mut area = self.area.lock().unwrap();
    let prev_area = *area;
    *area = Rect::from_size((new_x, new_y), (playhead_width, playhead_height));
    let new_area = *area;
    drop(area);

    let mut actived = self.actived_pos.lock().unwrap();
    *actived = Vec2::zero();
    drop(actived);

    let _ = self.ui_tx.send(UIUpdate::PlayheadPosAndArea(new_pos, new_area));
    let _ = self.ui_tx.send(UIUpdate::ChnStatus(self.compute_chn_str(new_pos)));

    self.enqueue_sym_rpl_cycle(prev_area);
    self.enqueue_sym_space();

    Vec2::zero()
  }

  fn get_jump_position(
    &self,
    current_playhead_pos: (usize, usize),
    params: GridParams,
  ) -> (usize, usize) {
    if let Some(first_item) = self.queue_manager.get_front_item() {
      match first_item {
        QueueItem::Position(x, y) => {
          let first_pos = (x, y);
          if first_pos == current_playhead_pos {
            PositionCalculator::generate_random_position(
              params.playhead_width,
              params.playhead_height,
              params.grid_width,
              params.grid_height,
            )
          } else if let Some(QueueItem::Position(x, y)) = self.queue_manager.remove_front_item() {
            self.execute_front_op_in_queue();
            (x, y)
          } else {
            (0, 0)
          }
        }
        QueueItem::Event(_) => PositionCalculator::generate_random_position(
          params.playhead_width,
          params.playhead_height,
          params.grid_width,
          params.grid_height,
        ),
      }
    } else {
      PositionCalculator::generate_random_position(
        params.playhead_width,
        params.playhead_height,
        params.grid_width,
        params.grid_height,
      )
    }
  }
}

#[cfg(test)]
mod tests {
  use super::super::test_helpers::make_playhead;
  use super::*;
  use crate::core::engine::regex::Match;

  #[test]
  fn reset_accumulation_counter_zeroes_nonzero_value() {
    let ph = make_playhead();
    *ph.accumulation_counter.lock().unwrap() = 5;
    ph.reset_accumulation_counter();
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 0);
  }

  #[test]
  fn handle_silent_step_does_nothing_when_mode_off() {
    let ph = make_playhead();
    ph.modes.accumulation_mode.store(false, Ordering::Relaxed);
    let matcher: HashMap<usize, Match> = HashMap::new();
    ph.handle_silent_step(&matcher);
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 0);
  }

  #[test]
  fn handle_silent_step_does_not_increment_when_match_inside_area() {
    let ph = make_playhead();
    ph.modes.accumulation_mode.store(true, Ordering::Relaxed);
    ph.grid.width.store(10, Ordering::Relaxed);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(0, 0), (2, 2));

    // index 0 = (0,0) which is inside the 2×2 area
    let mut matcher: HashMap<usize, Match> = HashMap::new();
    matcher.insert(0, Match::make_test(0, 1));

    ph.handle_silent_step(&matcher);
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 0);
  }

  #[test]
  fn handle_silent_step_increments_counter_when_no_match_in_area() {
    let ph = make_playhead();
    ph.modes.accumulation_mode.store(true, Ordering::Relaxed);
    ph.grid.width.store(10, Ordering::Relaxed);
    ph.grid.height.store(10, Ordering::Relaxed);
    // large area so limit isn't hit in one call
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(0, 0), (4, 4));

    let matcher: HashMap<usize, Match> = HashMap::new(); // no matches

    ph.handle_silent_step(&matcher);
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 1);

    ph.handle_silent_step(&matcher);
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 2);
  }

  #[test]
  fn handle_silent_step_resets_counter_and_jumps_when_limit_reached() {
    let ph = make_playhead();
    ph.modes.accumulation_mode.store(true, Ordering::Relaxed);
    ph.grid.width.store(10, Ordering::Relaxed);
    ph.grid.height.store(10, Ordering::Relaxed);
    // 1×1 area → limit = 1: the very first step should trigger a jump
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(5, 5), (1, 1));

    let matcher: HashMap<usize, Match> = HashMap::new();
    ph.handle_silent_step(&matcher);

    // counter must have wrapped back to 0
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 0);
  }

  #[test]
  fn handle_accumulation_mode_returns_none_before_limit() {
    let ph = make_playhead();
    ph.grid.width.store(10, Ordering::Relaxed);
    ph.grid.height.store(10, Ordering::Relaxed);
    // 4×4 area → limit = 16, so 15 calls should all return None
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (4, 4));

    for _ in 0..15 {
      let result = ph.handle_accumulation_mode(0);
      assert!(result.is_none(), "expected None before counter hits limit");
    }
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 15);
  }

  #[test]
  fn handle_accumulation_mode_returns_some_and_resets_at_limit() {
    let ph = make_playhead();
    ph.grid.width.store(10, Ordering::Relaxed);
    ph.grid.height.store(10, Ordering::Relaxed);
    // 1×1 area → limit = 1: first call triggers jump
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (1, 1));

    let result = ph.handle_accumulation_mode(0);
    assert!(result.is_some(), "expected Some at the limit");
    assert_eq!(*ph.accumulation_counter.lock().unwrap(), 0);
  }

  #[test]
  fn perform_accumulation_jump_uses_armed_pending_position() {
    use crate::core::playhead::queue::PendingJumpPosition;

    let ph = make_playhead();
    ph.grid.width.store(20, Ordering::Relaxed);
    ph.grid.height.store(20, Ordering::Relaxed);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (2, 2));

    ph.queue_manager
      .set_pending_jump(PendingJumpPosition::Armed(7, 3));

    ph.perform_accumulation_jump();

    let pos = *ph.pos.lock().unwrap();
    assert_eq!(pos.x, 7);
    assert_eq!(pos.y, 3);
  }

  #[test]
  fn perform_accumulation_jump_resets_active_pos_to_zero() {
    let ph = make_playhead();
    ph.grid.width.store(20, Ordering::Relaxed);
    ph.grid.height.store(20, Ordering::Relaxed);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (2, 2));
    *ph.actived_pos.lock().unwrap() = Vec2::new(3, 3);

    ph.perform_accumulation_jump();

    assert_eq!(*ph.actived_pos.lock().unwrap(), Vec2::zero());
  }

  #[test]
  fn perform_accumulation_jump_preserves_playhead_size() {
    let ph = make_playhead();
    ph.grid.width.store(20, Ordering::Relaxed);
    ph.grid.height.store(20, Ordering::Relaxed);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (3, 2));

    ph.perform_accumulation_jump();

    let area = *ph.area.lock().unwrap();
    assert_eq!(area.width(), 3);
    assert_eq!(area.height(), 2);
  }

  #[test]
  fn perform_accumulation_jump_random_stays_within_grid_bounds() {
    let ph = make_playhead();
    let grid_w = 10_usize;
    let grid_h = 8_usize;
    ph.grid.width.store(grid_w, Ordering::Relaxed);
    ph.grid.height.store(grid_h, Ordering::Relaxed);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (2, 2));

    for _ in 0..50 {
      ph.perform_accumulation_jump();
      let pos = *ph.pos.lock().unwrap();
      assert!(
        pos.x + 2 <= grid_w,
        "x={} out of bounds (grid_w={})",
        pos.x,
        grid_w
      );
      assert!(
        pos.y + 2 <= grid_h,
        "y={} out of bounds (grid_h={})",
        pos.y,
        grid_h
      );
    }
  }
}
