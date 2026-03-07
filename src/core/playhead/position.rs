use cursive::Vec2;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use super::movement;
use crate::core::engine::regex::Match;
use crate::core::playhead::movement::Movement;

// ============================================================================
// Position Calculator
// ============================================================================

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
