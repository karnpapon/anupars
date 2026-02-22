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
  playhead_w: usize,
  playhead_h: usize,
  actived_pos: &mut Vec2,
) {
  actived_pos.x = adjusted_pos % playhead_w;
  if actived_pos.x == 0 {
    actived_pos.y += 1;
    actived_pos.y %= playhead_h;
  }
}

/// Calculate position for reverse sequential mode
pub fn calculate_reverse_position(
  adjusted_pos: usize,
  playhead_w: usize,
  playhead_h: usize,
  actived_pos: &mut Vec2,
) {
  actived_pos.x = playhead_w - 1 - (adjusted_pos % playhead_w);
  if actived_pos.x == playhead_w - 1 {
    if actived_pos.y == 0 {
      actived_pos.y = playhead_h - 1;
    } else {
      actived_pos.y -= 1;
    }
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
  reverse: bool,
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
  playhead_w: usize,
  playhead_h: usize,
  reverse: bool,
  random: bool,
  actived_pos: &mut Vec2,
) {
  if random {
    calculate_random_position(adjusted_pos, playhead_w, playhead_h, actived_pos);
  } else if reverse {
    calculate_reverse_position(adjusted_pos, playhead_w, playhead_h, actived_pos);
  } else {
    calculate_normal_position(adjusted_pos, playhead_w, playhead_h, actived_pos);
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
