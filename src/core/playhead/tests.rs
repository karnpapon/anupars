#[cfg(test)]
use crate::core::engine::regex::Match;
use crate::core::io::midi;
use crate::core::playhead::movement::Movement;
use crate::core::playhead::Direction;
use crate::core::playhead::Playhead;
use crate::view::rect::Rect;
use cursive::Vec2;
use std::collections::HashMap;
use std::sync::atomic::Ordering;

fn make_playhead() -> Playhead {
  let (tx, _rx) = std::sync::mpsc::channel::<midi::Message>();
  let siv = cursive::Cursive::new();
  Playhead::new(tx, siv.cb_sink().clone())
}

#[test]
fn chn_str_no_splits_is_always_1_of_1() {
  let ph = make_playhead();
  ph.grid.width.store(8, Ordering::Relaxed);
  ph.grid.height.store(8, Ordering::Relaxed);
  ph.grid.v_splits.store(1, Ordering::Relaxed);
  ph.grid.h_splits.store(1, Ordering::Relaxed);
  assert_eq!(ph.compute_chn_str(Vec2::new(0, 0)), "1/1");
  assert_eq!(ph.compute_chn_str(Vec2::new(7, 7)), "1/1");
}

#[test]
fn chn_str_2x2_splits_maps_quadrants_correctly() {
  let ph = make_playhead();
  ph.grid.width.store(8, Ordering::Relaxed);
  ph.grid.height.store(8, Ordering::Relaxed);
  ph.grid.v_splits.store(2, Ordering::Relaxed);
  ph.grid.h_splits.store(2, Ordering::Relaxed);
  assert_eq!(ph.compute_chn_str(Vec2::new(0, 0)), "1/4"); // top-left
  assert_eq!(ph.compute_chn_str(Vec2::new(4, 0)), "2/4"); // top-right
  assert_eq!(ph.compute_chn_str(Vec2::new(0, 4)), "3/4"); // bottom-left
  assert_eq!(ph.compute_chn_str(Vec2::new(4, 4)), "4/4"); // bottom-right
}

#[test]
fn chn_str_position_at_boundary_does_not_overflow() {
  let ph = make_playhead();
  ph.grid.width.store(4, Ordering::Relaxed);
  ph.grid.height.store(2, Ordering::Relaxed);
  ph.grid.v_splits.store(2, Ordering::Relaxed);
  ph.grid.h_splits.store(1, Ordering::Relaxed);
  assert_eq!(ph.compute_chn_str(Vec2::new(3, 0)), "2/2");
}

#[test]
fn going_forward_for_forward_movement() {
  let ph = make_playhead();
  *ph.movement.lock().unwrap() = Movement::Forward;
  assert!(ph.is_going_forward());
}

#[test]
fn not_going_forward_for_reverse_movement() {
  let ph = make_playhead();
  *ph.movement.lock().unwrap() = Movement::Reverse;
  assert!(!ph.is_going_forward());
}

#[test]
fn pendulum_first_half_is_forward_second_half_is_backward() {
  let ph = make_playhead();
  *ph.movement.lock().unwrap() = Movement::Pendulum;
  *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (4, 1));

  *ph.step_index.lock().unwrap() = 3;
  assert!(ph.is_going_forward());

  *ph.step_index.lock().unwrap() = 4;
  assert!(!ph.is_going_forward());
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
fn check_contains_returns_true_when_match_is_inside_area() {
  let ph = make_playhead();
  ph.grid.width.store(10, Ordering::Relaxed);
  let area = Rect::from_size(Vec2::new(2, 1), (2, 1));
  let mut matcher: HashMap<usize, Match> = HashMap::new();
  matcher.insert(12, Match::make_test(12, 1));
  assert!(ph.check_contains(&area, &matcher));
}

#[test]
fn check_contains_returns_false_when_match_is_outside_area() {
  let ph = make_playhead();
  ph.grid.width.store(10, Ordering::Relaxed);
  let area = Rect::from_size(Vec2::new(2, 1), (2, 1));
  let mut matcher: HashMap<usize, Match> = HashMap::new();
  matcher.insert(5, Match::make_test(5, 1));
  assert!(!ph.check_contains(&area, &matcher));
}

#[test]
fn set_leap_moves_pos_and_keeps_area_size() {
  let ph = make_playhead();
  *ph.pos.lock().unwrap() = Vec2::new(2, 2);
  *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(2, 2), (3, 2));
  let canvas = Vec2::new(20, 20);
  ph.set_leap(Direction::Right, 2, canvas);
  let pos = *ph.pos.lock().unwrap();
  let area = *ph.area.lock().unwrap();
  assert_eq!(pos, Vec2::new(4, 2));
  assert_eq!(area.width(), 3); // size preserved
  assert_eq!(area.height(), 2);
  assert_eq!(area.top_left(), Vec2::new(4, 2));
}

#[test]
fn set_leap_rejects_move_that_would_exceed_canvas() {
  let ph = make_playhead();
  *ph.pos.lock().unwrap() = Vec2::new(18, 0);
  *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(18, 0), (3, 1)); // right edge at 20
  let canvas = Vec2::new(20, 20);
  ph.set_leap(Direction::Right, 1, canvas);
  let pos = *ph.pos.lock().unwrap();
  assert_eq!(pos.x, 18);
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
fn mode_string_all_off_is_lowercase() {
  let ph = make_playhead();
  let s = ph.build_mode_status_string();
  // All modes off → every character is lowercase (or hyphen)
  assert!(s.chars().all(|c| !c.is_uppercase()), "got: {s}");
}

#[test]
fn mode_string_arpeggiator_on_shows_uppercase_a() {
  let ph = make_playhead();
  ph.modes.arpeggiator_mode.store(true, Ordering::Relaxed);
  let s = ph.build_mode_status_string();
  assert!(s.contains('A'), "expected uppercase A in: {s}");
}

#[test]
fn mode_string_multiple_modes_reflected() {
  let ph = make_playhead();
  ph.modes.accumulation_mode.store(true, Ordering::Relaxed);
  ph.modes.sweep_mode.store(true, Ordering::Relaxed);
  let s = ph.build_mode_status_string();
  assert!(s.contains('U'), "accumulation (U) missing in: {s}");
  assert!(s.contains('S'), "sweep (S) missing in: {s}");
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
