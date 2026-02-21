//! Playback mode position calculation strategies
//!
//! This module contains position calculation logic for different playback modes:
//! - Normal: Sequential forward movement
//! - Reverse: Sequential backward movement
//! - Random: Deterministic pseudo-random positioning
//! - Arpeggiator: Pattern-based movement through regex matches

use cursive::Vec2;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

/// Calculate position for normal (forward) sequential mode
pub fn calculate_normal_position(
  adjusted_pos: usize,
  marker_w: usize,
  marker_h: usize,
  actived_pos: &mut Vec2,
) {
  actived_pos.x = adjusted_pos % marker_w;
  if actived_pos.x == 0 {
    actived_pos.y += 1;
    actived_pos.y %= marker_h;
  }
}

/// Calculate position for reverse sequential mode
pub fn calculate_reverse_position(
  adjusted_pos: usize,
  marker_w: usize,
  marker_h: usize,
  actived_pos: &mut Vec2,
) {
  actived_pos.x = marker_w - 1 - (adjusted_pos % marker_w);
  if actived_pos.x == marker_w - 1 {
    if actived_pos.y == 0 {
      actived_pos.y = marker_h - 1;
    } else {
      actived_pos.y -= 1;
    }
  }
}

/// Calculate random position within marker area using deterministic hashing
pub fn calculate_random_position(
  adjusted_pos: usize,
  marker_w: usize,
  marker_h: usize,
  actived_pos: &mut Vec2,
) {
  let mut hasher = DefaultHasher::new();
  adjusted_pos.hash(&mut hasher);
  let hash = hasher.finish() as usize;
  actived_pos.x = hash % marker_w;
  actived_pos.y = (hash / marker_w) % marker_h;
}

/// Get arpeggiator matches within marker area
pub fn get_arpeggiator_matches(
  regex_indexes: &std::collections::BTreeSet<usize>,
  marker_x: usize,
  marker_y: usize,
  marker_w: usize,
  marker_h: usize,
  canvas_w: usize,
  reverse: bool,
) -> Vec<(usize, usize)> {
  let mut matches: Vec<(usize, usize)> = regex_indexes
    .iter()
    .filter_map(|&idx| {
      let x = idx % canvas_w;
      let y = idx / canvas_w;
      if x >= marker_x && x < marker_x + marker_w && y >= marker_y && y < marker_y + marker_h {
        Some((x - marker_x, y - marker_y))
      } else {
        None
      }
    })
    .collect();

  matches.sort_by_key(|&(x, y)| (y, x));
  if reverse {
    matches.reverse();
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
  marker_w: usize,
  marker_h: usize,
  reverse: bool,
  random: bool,
  actived_pos: &mut Vec2,
) {
  if random {
    calculate_random_position(adjusted_pos, marker_w, marker_h, actived_pos);
  } else if reverse {
    calculate_reverse_position(adjusted_pos, marker_w, marker_h, actived_pos);
  } else {
    calculate_normal_position(adjusted_pos, marker_w, marker_h, actived_pos);
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
