use cursive::Vec2;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::movement;
use crate::core::engine::regex::Match;
use crate::core::playhead::movement::Movement;

/// Helper struct for position calculations
pub struct PositionCalculator {
  pub grid_width: Arc<AtomicUsize>,
  pub grid_height: Arc<AtomicUsize>,
  pub arpeggiator_mode: Arc<AtomicBool>,
  pub movement: Arc<Mutex<Movement>>,
  pub regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  pub text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,
}

impl PositionCalculator {
  pub fn new() -> Self {
    PositionCalculator {
      grid_width: Arc::new(AtomicUsize::new(0)),
      grid_height: Arc::new(AtomicUsize::new(0)),
      arpeggiator_mode: Arc::new(AtomicBool::new(false)),
      movement: Arc::new(Mutex::new(Movement::Forward)),
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      text_matcher: Arc::new(Mutex::new(None)),
    }
  }

  /// Calculate activated position based on step index and playhead area
  pub fn calculate_actived_pos(
    &self,
    pos: usize,
    playhead_x: usize,
    playhead_y: usize,
    playhead_w: usize,
    playhead_h: usize,
  ) -> Vec2 {
    let mut actived_pos = Vec2::zero();
    let canvas_w = self.grid_width.load(Ordering::Relaxed);
    let arpeggiator = self.arpeggiator_mode.load(Ordering::Relaxed);

    if arpeggiator {
      let regex_indexes = self.regex_indexes.lock().unwrap();
      let movement = self.movement.lock().unwrap();
      let movement_random = *movement == Movement::Random;
      let matches = movement::get_arpeggiator_matches(
        &regex_indexes,
        playhead_x,
        playhead_y,
        playhead_w,
        playhead_h,
        canvas_w,
        *movement,
      );
      drop(regex_indexes);
      drop(movement);

      if !matches.is_empty() {
        let step = if movement_random {
          movement::get_random_index(pos, matches.len())
        } else {
          pos % matches.len()
        };
        let (x, y) = matches[step];
        actived_pos.x = x;
        actived_pos.y = y;
      } else {
        let movement = self.movement.lock().unwrap();
        // No matches, fallback to normal running
        movement::calculate_position_fallback(
          pos,
          playhead_w,
          playhead_h,
          *movement,
          &mut actived_pos,
        );
        drop(movement);
      }
    } else {
      let movement = self.movement.lock().unwrap();
      // Normal running without arpeggiator
      movement::calculate_position_fallback(
        pos,
        playhead_w,
        playhead_h,
        *movement,
        &mut actived_pos,
      );
      drop(movement);
    }

    actived_pos
  }

  /// Get absolute position coordinates and flattened index
  pub fn calculate_absolute_position(
    &self,
    playhead_pos: Vec2,
    active_pos: Vec2,
  ) -> (usize, usize, usize) {
    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let abs_y = playhead_pos.y + active_pos.y;
    let abs_x = playhead_pos.x + active_pos.x;
    let curr_running_playhead = (abs_y * grid_width) + abs_x;
    (abs_x, abs_y, curr_running_playhead)
  }

  /// Generate random position within grid bounds
  pub fn generate_random_position(
    playhead_width: usize,
    playhead_height: usize,
    grid_width: usize,
    grid_height: usize,
    rng: &mut impl rand::Rng,
  ) -> (usize, usize) {
    let max_x = grid_width.saturating_sub(playhead_width);
    let max_y = grid_height.saturating_sub(playhead_height);

    if max_x > 0 && max_y > 0 {
      (rng.gen_range(0..=max_x), rng.gen_range(0..=max_y))
    } else {
      (0, 0)
    }
  }

  /// Find distance to the next closest trigger position within the playhead area
  pub fn find_distance_to_next_trigger(
    &self,
    curr_pos: usize,
    playhead_x: usize,
    playhead_y: usize,
    playhead_width: usize,
    playhead_height: usize,
  ) -> usize {
    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let default_distance = 4;

    let movement = *self.movement.lock().unwrap();
    if matches!(movement, Movement::Random) {
      return default_distance;
    }

    // Get playhead area bounds and extract triggers in one lock
    let trigger_positions = {
      if playhead_width == 0 || playhead_height == 0 {
        return default_distance;
      }

      // Extract and sort trigger positions in playhead area
      let matcher = self.text_matcher.lock().unwrap();
      let positions: Vec<usize> = if let Some(ref map) = *matcher {
        let mut pos_vec: Vec<usize> = map
          .keys()
          .filter(|&&p| {
            let x = p % grid_width;
            let y = p / grid_width;
            x >= playhead_x
              && x < playhead_x + playhead_width
              && y >= playhead_y
              && y < playhead_y + playhead_height
          })
          .copied()
          .collect();
        pos_vec.sort_unstable();
        pos_vec
      } else {
        return default_distance;
      };

      positions
    };

    if trigger_positions.is_empty() {
      return default_distance;
    }

    match movement {
      Movement::Reverse => {
        let idx = trigger_positions.binary_search(&curr_pos);
        let prev_idx = match idx {
          Ok(i) if i > 0 => i - 1,
          Err(i) if i > 0 => i - 1,
          _ => {
            // No previous trigger, we're at the first trigger
            // calculate distance to playhead start (where reverse wraps)
            let curr_x = curr_pos % grid_width;
            let distance_to_start = curr_x.saturating_sub(playhead_x);
            return distance_to_start.clamp(1, 16);
          }
        };

        let prev_pos = trigger_positions[prev_idx];
        let distance = curr_pos.saturating_sub(prev_pos);
        distance.clamp(1, 16)
      }
      Movement::Forward | Movement::Pendulum => {
        let idx = trigger_positions.binary_search(&curr_pos);
        let next_idx = match idx {
          Ok(i) => i + 1,
          Err(i) => i,
        };

        if next_idx < trigger_positions.len() {
          let next_pos = trigger_positions[next_idx];
          let distance = next_pos.saturating_sub(curr_pos);
          return distance.clamp(1, 16);
        }

        let curr_x = curr_pos % grid_width;
        let distance_to_edge = (playhead_x + playhead_width).saturating_sub(curr_x);
        distance_to_edge.clamp(1, 16)
      }
      Movement::Random => unreachable!(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::engine::regex::Match;
  use crate::core::playhead::movement::Movement;

  fn make_calc() -> PositionCalculator {
    PositionCalculator::new()
  }

  #[test]
  fn absolute_position_adds_playhead_and_active_offsets() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);

    let (abs_x, abs_y, idx) = calc.calculate_absolute_position(Vec2::new(2, 3), Vec2::new(1, 2));

    assert_eq!(abs_x, 3);
    assert_eq!(abs_y, 5);
    assert_eq!(idx, 53); // 5*10 + 3
  }

  #[test]
  fn absolute_position_zero_offsets_returns_zero() {
    let calc = make_calc();
    calc.grid_width.store(8, Ordering::Relaxed);

    let (abs_x, abs_y, idx) = calc.calculate_absolute_position(Vec2::zero(), Vec2::zero());

    assert_eq!((abs_x, abs_y, idx), (0, 0, 0));
  }

  #[test]
  fn absolute_position_flattened_index_uses_grid_width() {
    let calc = make_calc();
    calc.grid_width.store(5, Ordering::Relaxed);

    let (_, _, idx) = calc.calculate_absolute_position(Vec2::new(0, 2), Vec2::new(3, 0));
    assert_eq!(idx, 13); // abs_y=2, abs_x=3 → 2*5+3
  }

  #[test]
  fn random_position_returns_origin_when_grid_equals_playhead_size() {
    let mut rng = rand::thread_rng();
    // max_x = 4 - 4 = 0, so returns (0,0)
    let (x, y) = PositionCalculator::generate_random_position(4, 4, 4, 4, &mut rng);
    assert_eq!((x, y), (0, 0));
  }

  #[test]
  fn random_position_returns_origin_when_grid_smaller_than_playhead() {
    let mut rng = rand::thread_rng();
    let (x, y) = PositionCalculator::generate_random_position(5, 5, 3, 3, &mut rng);
    assert_eq!((x, y), (0, 0));
  }

  #[test]
  fn random_position_stays_within_grid_bounds() {
    let mut rng = rand::thread_rng();
    let grid_w = 10;
    let grid_h = 8;
    let ph_w = 3;
    let ph_h = 2;

    for _ in 0..50 {
      let (x, y) =
        PositionCalculator::generate_random_position(ph_w, ph_h, grid_w, grid_h, &mut rng);
      assert!(x + ph_w <= grid_w, "x={x} overflows grid_w={grid_w}");
      assert!(y + ph_h <= grid_h, "y={y} overflows grid_h={grid_h}");
    }
  }

  #[test]
  fn actived_pos_forward_first_step_is_origin() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);

    let v = calc.calculate_actived_pos(0, 0, 0, 4, 2);
    assert_eq!(v, Vec2::new(0, 0));
  }

  #[test]
  fn actived_pos_forward_wraps_to_second_row() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    // w=4, h=2 → total=8; pos=4 → linear=4, x=4%4=0, y=4/4=1
    let v = calc.calculate_actived_pos(4, 0, 0, 4, 2);
    assert_eq!(v, Vec2::new(0, 1));
  }

  #[test]
  fn actived_pos_forward_wraps_back_after_full_cycle() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    // w=4, h=2 → total=8; pos=8 → linear=0
    let v = calc.calculate_actived_pos(8, 0, 0, 4, 2);
    assert_eq!(v, Vec2::new(0, 0));
  }

  #[test]
  fn actived_pos_reverse_first_step_is_last_cell() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    *calc.movement.lock().unwrap() = Movement::Reverse;
    // w=4, h=1 → total=4; pos=0 → reverse_linear=3, x=3, y=0
    let v = calc.calculate_actived_pos(0, 0, 0, 4, 1);
    assert_eq!(v, Vec2::new(3, 0));
  }

  #[test]
  fn distance_returns_default_for_random_movement() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    *calc.movement.lock().unwrap() = Movement::Random;

    let d = calc.find_distance_to_next_trigger(5, 0, 0, 4, 4);
    assert_eq!(d, 4);
  }

  #[test]
  fn distance_returns_default_when_no_matcher() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    // text_matcher is None by default

    let d = calc.find_distance_to_next_trigger(5, 0, 0, 4, 4);
    assert_eq!(d, 4);
  }

  #[test]
  fn distance_returns_default_when_no_triggers_in_area() {
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    // Trigger at index 50 is outside the 0,0 area of size 3x1
    let mut map = HashMap::new();
    map.insert(50, Match::make_test(50, 1));
    *calc.text_matcher.lock().unwrap() = Some(map);

    let d = calc.find_distance_to_next_trigger(0, 0, 0, 3, 1);
    assert_eq!(d, 4);
  }

  #[test]
  fn distance_forward_to_next_trigger() {
    // grid_width=10, area x=0,y=0 w=10 h=1 → row 0
    // curr_pos=2, trigger at index 5 → distance = 3
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    let mut map = HashMap::new();
    map.insert(5, Match::make_test(5, 1));
    *calc.text_matcher.lock().unwrap() = Some(map);

    let d = calc.find_distance_to_next_trigger(2, 0, 0, 10, 1);
    assert_eq!(d, 3);
  }

  #[test]
  fn distance_forward_when_at_trigger_points_to_next() {
    // curr_pos=5 (at trigger), next trigger at 8 → distance = 3
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    let mut map = HashMap::new();
    map.insert(5, Match::make_test(5, 1));
    map.insert(8, Match::make_test(8, 1));
    *calc.text_matcher.lock().unwrap() = Some(map);

    let d = calc.find_distance_to_next_trigger(5, 0, 0, 10, 1);
    assert_eq!(d, 3);
  }

  #[test]
  fn distance_forward_no_trigger_ahead_returns_distance_to_edge() {
    // area: x=0,y=0 w=6 h=1; curr_pos=4, trigger only at 2 (behind) → edge = 6-4=2
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    let mut map = HashMap::new();
    map.insert(2, Match::make_test(2, 1));
    *calc.text_matcher.lock().unwrap() = Some(map);

    let d = calc.find_distance_to_next_trigger(4, 0, 0, 6, 1);
    assert_eq!(d, 2); // (0+6) - 4 = 2
  }

  #[test]
  fn distance_reverse_to_previous_trigger() {
    // curr_pos=7, trigger at 4 → distance = 3
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    *calc.movement.lock().unwrap() = Movement::Reverse;
    let mut map = HashMap::new();
    map.insert(4, Match::make_test(4, 1));
    *calc.text_matcher.lock().unwrap() = Some(map);

    let d = calc.find_distance_to_next_trigger(7, 0, 0, 10, 1);
    assert_eq!(d, 3);
  }

  #[test]
  fn distance_clamps_to_1_minimum() {
    // trigger is at same position as curr_pos (forward, Ok(i) → next_idx i+1, no next trigger)
    // edge = 6 - 5 = 1
    let calc = make_calc();
    calc.grid_width.store(10, Ordering::Relaxed);
    let mut map = HashMap::new();
    map.insert(5, Match::make_test(5, 1));
    *calc.text_matcher.lock().unwrap() = Some(map);

    let d = calc.find_distance_to_next_trigger(5, 0, 0, 6, 1);
    assert!(d >= 1, "distance must be at least 1, got {d}");
  }
}
