use std::collections::HashMap;
use std::sync::atomic::Ordering;

use cursive::Vec2;

use crate::core::consts;
use crate::core::engine::regex;
use crate::core::playhead::position::PositionCalculator;
use crate::core::playhead::queue::{PendingJumpPosition, QueueItem};
use crate::view::rect::Rect;

use super::{Playhead, UIUpdate};

struct GridParams<'a, R: rand::Rng> {
  pub playhead_width: usize,
  pub playhead_height: usize,
  pub grid_width: usize,
  pub grid_height: usize,
  pub rng: &'a mut R,
}

impl Playhead {
  pub fn reset_accumulation_counter(&self) {
    let mut counter = self.accumulation_counter.lock().unwrap();
    *counter = 0;
  }

  pub(super) fn update_accumulation_ui(&self, count: usize, total: usize) {
    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::AccumulationCounter(count, total));
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
    let mut rng = rand::thread_rng();

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
            rng: &mut rng,
          };
          self.get_jump_position(current_playhead_pos, params)
        }
        PendingJumpPosition::Empty => {
          let params = GridParams {
            playhead_width,
            playhead_height,
            grid_width,
            grid_height,
            rng: &mut rng,
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

    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::PlayheadPosAndArea(new_pos, new_area));
    queue.push_back(UIUpdate::ChnStatus(self.compute_chn_str(new_pos)));
    drop(queue);

    self.enqueue_sym_rpl_cycle(prev_area);
    self.enqueue_sym_space();

    Vec2::zero()
  }

  fn get_jump_position<R: rand::Rng>(
    &self,
    current_playhead_pos: (usize, usize),
    params: GridParams<R>,
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
              params.rng,
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
          params.rng,
        ),
      }
    } else {
      PositionCalculator::generate_random_position(
        params.playhead_width,
        params.playhead_height,
        params.grid_width,
        params.grid_height,
        params.rng,
      )
    }
  }
}
