//! movementt position calculation strategies
//!
//! This module contains position calculation logic for different playback modes:
//! - Normal: Sequential forward movement
//! - Reverse: Sequential backward movement
//! - Random: Deterministic pseudo-random positioning
//! - Arpeggiator: Pattern-based movement through regex matches

use std::fmt;

use crate::core::consts;

#[derive(Clone, PartialEq, Copy, Debug)]
pub enum Movement {
  Forward,
  Reverse,
  Random,
  Pendulum,
}

impl fmt::Display for Movement {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Movement::Forward => write!(f, "f"),
      Movement::Reverse => write!(f, "r"),
      Movement::Random => write!(f, "d"),
      Movement::Pendulum => write!(f, "p"),
    }
  }
}

impl Movement {
  pub fn print_movements(&self) -> String {
    self.print_movements_with_sweep(None)
  }

  pub fn print_movements_with_sweep(&self, sweep_movement: Option<Movement>) -> String {
    let movements = [
      Movement::Forward,
      Movement::Reverse,
      Movement::Random,
      Movement::Pendulum,
    ];
    movements
      .iter()
      .map(|mv| {
        let is_normal_active = *mv == *self;
        let is_sweep_active = sweep_movement.map(|s| *mv == s).unwrap_or(false);
        let so = consts::SWEEP_MOVEMENT_OPEN;
        let sc = consts::SWEEP_MOVEMENT_CLOSE;
        let mo = consts::MOVEMENT_OPEN;
        let mc = consts::MOVEMENT_CLOSE;
        match (is_normal_active, is_sweep_active) {
          (true, true) => format!("{so}{mo}{mv}{mc}{sc}"),
          (true, false) => format!("{mo}{mv}{mc}"),
          (false, true) => format!("{so}{mv}{sc}"),
          (false, false) => format!("{mv}"),
        }
      })
      .collect::<Vec<_>>()
      .join("")
  }
}
use crate::core::geom::Vec2;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// Calculate position for normal (forward) sequential mode
pub fn calculate_normal_position(
  adjusted_pos: usize,
  playhead_w: usize,
  playhead_h: usize,
  actived_pos: &mut Vec2,
) {
  let total_positions = playhead_w * playhead_h;
  let linear_pos = if total_positions > 0 {
    adjusted_pos % total_positions
  } else {
    0
  };
  actived_pos.x = if playhead_w > 0 {
    linear_pos % playhead_w
  } else {
    0
  };
  actived_pos.y = if playhead_w > 0 {
    linear_pos / playhead_w
  } else {
    0
  };
}

/// Calculate position for reverse sequential mode
pub fn calculate_reverse_position(
  adjusted_pos: usize,
  playhead_w: usize,
  playhead_h: usize,
  actived_pos: &mut Vec2,
) {
  let total_positions = playhead_w * playhead_h;
  if total_positions == 0 {
    actived_pos.x = 0;
    actived_pos.y = 0;
    return;
  }
  let linear_pos = adjusted_pos % total_positions;
  let reverse_linear = total_positions - 1 - linear_pos;
  actived_pos.x = if playhead_w > 0 {
    reverse_linear % playhead_w
  } else {
    0
  };
  actived_pos.y = if playhead_w > 0 {
    reverse_linear / playhead_w
  } else {
    0
  };
}

pub fn calculate_pendulum_position(
  adjusted_pos: usize,
  playhead_w: usize,
  playhead_h: usize,
  actived_pos: &mut Vec2,
) {
  let total_positions = playhead_w * playhead_h;
  if total_positions == 0 {
    actived_pos.x = 0;
    actived_pos.y = 0;
    return;
  }
  if total_positions == 1 {
    actived_pos.x = 0;
    actived_pos.y = 0;
    return;
  }
  let cycle_length = total_positions * 2 - 2;
  let cycle_pos = adjusted_pos % cycle_length;

  if cycle_pos < total_positions {
    calculate_normal_position(cycle_pos, playhead_w, playhead_h, actived_pos);
  } else {
    let reverse_pos = cycle_pos - total_positions + 1;
    calculate_reverse_position(reverse_pos, playhead_w, playhead_h, actived_pos);
  }
}

/// Calculate random position within playhead area using deterministic hashing
pub fn calculate_random_position(
  adjusted_pos: usize,
  playhead_w: usize,
  playhead_h: usize,
  actived_pos: &mut Vec2,
) {
  let mut hasher = DefaultHasher::new();
  adjusted_pos.hash(&mut hasher);
  let hash = hasher.finish() as usize;
  actived_pos.x = hash % playhead_w;
  actived_pos.y = (hash / playhead_w) % playhead_h;
}

/// Get arpeggiator matches within playhead area
pub fn get_arpeggiator_matches(
  regex_indexes: &std::collections::BTreeSet<usize>,
  playhead_x: usize,
  playhead_y: usize,
  playhead_w: usize,
  playhead_h: usize,
  canvas_w: usize,
  movement: Movement,
) -> Vec<(usize, usize)> {
  let mut matches: Vec<(usize, usize)> = regex_indexes
    .iter()
    .filter_map(|&idx| {
      let x = idx % canvas_w;
      let y = idx / canvas_w;
      if x >= playhead_x
        && x < playhead_x + playhead_w
        && y >= playhead_y
        && y < playhead_y + playhead_h
      {
        Some((x - playhead_x, y - playhead_y))
      } else {
        None
      }
    })
    .collect();

  matches.sort_by_key(|&(x, y)| (y, x));
  match movement {
    Movement::Reverse => matches.reverse(),
    Movement::Pendulum => {
      if matches.len() > 1 {
        let mut pendulum_matches = matches.clone();
        pendulum_matches.extend(matches.iter().rev().skip(1).take(matches.len() - 1));
        return pendulum_matches;
      }
    }
    _ => {} // Forward and Random use normal order
  }
  matches
}

/// Generate deterministic "random" index from position
pub fn get_random_index(pos: usize, max: usize) -> usize {
  let mut hasher = DefaultHasher::new();
  pos.hash(&mut hasher);
  (hasher.finish() as usize) % max
}

/// Calculate position when not using arpeggiator or as fallback
pub fn calculate_position_fallback(
  adjusted_pos: usize,
  playhead_w: usize,
  playhead_h: usize,
  movement: Movement,
  actived_pos: &mut Vec2,
) {
  match movement {
    Movement::Random => {
      calculate_random_position(adjusted_pos, playhead_w, playhead_h, actived_pos);
    }
    Movement::Reverse => {
      calculate_reverse_position(adjusted_pos, playhead_w, playhead_h, actived_pos);
    }
    Movement::Pendulum => {
      calculate_pendulum_position(adjusted_pos, playhead_w, playhead_h, actived_pos);
    }
    _ => {
      calculate_normal_position(adjusted_pos, playhead_w, playhead_h, actived_pos);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_normal_position() {
    let mut pos = Vec2::new(0, 0);
    calculate_normal_position(5, 4, 3, &mut pos);
    assert_eq!(pos.x, 1); // 5 % 4 = 1
  }

  #[test]
  fn test_reverse_position() {
    let mut pos = Vec2::new(0, 0);
    calculate_reverse_position(0, 4, 3, &mut pos);
    assert_eq!(pos.x, 3); // 4 - 1 - 0 = 3
  }

  #[test]
  fn test_random_is_deterministic() {
    let mut pos1 = Vec2::new(0, 0);
    let mut pos2 = Vec2::new(0, 0);
    calculate_random_position(42, 10, 10, &mut pos1);
    calculate_random_position(42, 10, 10, &mut pos2);
    assert_eq!(pos1, pos2); // Same input = same output
  }
}
