use cursive::views::{Canvas, TextView};
use cursive::Vec2;

use super::Direction;
use crate::core::{consts, utils};
use crate::view::grid::GridEditor;
use crate::view::rect::Rect;

use super::{Playhead, UIUpdate};

impl Playhead {
  pub(super) fn handle_start_aim(&self) {
    let current_pos = *self.pos.lock().unwrap();
    let area_size = self.area.lock().unwrap().size();
    let start_area = Rect::from_size(current_pos, area_size);

    *self.aimed_area.lock().unwrap() = Some(start_area);
    let mut q = self.ui_update_queue.lock().unwrap();
    q.push_back(UIUpdate::AimedArea(Some(start_area)));
  }

  pub(super) fn handle_update_aim(&self, direction: Direction, canvas_size: Vec2, step: usize) {
    let mut aimed = self.aimed_area.lock().unwrap();
    let base_area = aimed.unwrap_or_else(|| *self.area.lock().unwrap());

    let (dx, dy) = direction.get_direction();
    let new_top_left = Vec2::new(
      (base_area.left() as i32 + dx * (step as i32)).max(0) as usize,
      (base_area.top() as i32 + dy * (step as i32)).max(0) as usize,
    );

    let size = base_area.size();
    let max_x = canvas_size.x.saturating_sub(size.x);
    let max_y = canvas_size.y.saturating_sub(size.y);

    let clamped_top_left = Vec2::new(new_top_left.x.min(max_x), new_top_left.y.min(max_y));
    let new_aimed_area = Rect::from_size(clamped_top_left, size);

    *aimed = Some(new_aimed_area);
    drop(aimed);

    let mut q = self.ui_update_queue.lock().unwrap();
    q.push_back(UIUpdate::AimedArea(Some(new_aimed_area)));
  }

  pub(super) fn handle_commit_aim(&self) {
    let aimed = self.aimed_area.lock().unwrap();
    let aimed_area_opt = *aimed;
    drop(aimed);

    if let Some(aimed_area) = aimed_area_opt {
      let target_pos = aimed_area.top_left();

      *self.pos.lock().unwrap() = target_pos;
      *self.aimed_area.lock().unwrap() = None;

      let mut actived = self.actived_pos.lock().unwrap();
      *actived = Vec2::zero();
      drop(actived);

      let chn_str = self.compute_chn_str(target_pos);
      let area_size = aimed_area.size();
      let pos_status = utils::build_pos_status_str(target_pos);
      let len_status = utils::build_len_status_str((area_size.x, area_size.y));
      let cb_sink = self.cb_sink.clone();

      cb_sink
        .send(Box::new(move |siv| {
          siv.call_on_name(
            consts::canvas_editor_section_view,
            move |canvas: &mut Canvas<GridEditor>| {
              let editor = canvas.state_mut();
              editor.playhead_ui.playhead_pos = target_pos;
              editor.playhead_ui.playhead_area = aimed_area;
              editor.playhead_ui.aimed_area = None;
            },
          );

          siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
            view.set_content(pos_status.clone());
          });

          siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
            view.set_content(len_status.clone());
          });

          siv.call_on_name(consts::chn_status_unit_view, move |view: &mut TextView| {
            view.set_content(chn_str.clone());
          });
        }))
        .unwrap();
    } else {
      let cb_sink = self.cb_sink.clone();
      cb_sink
        .send(Box::new(move |siv| {
          siv.call_on_name(
            consts::canvas_editor_section_view,
            move |canvas: &mut Canvas<GridEditor>| {
              let editor = canvas.state_mut();
              editor.playhead_ui.aimed_area = None;
            },
          );
        }))
        .unwrap();
    }
  }

  pub(super) fn handle_cancel_aim(&self) {
    *self.aimed_area.lock().unwrap() = None;
    let mut q = self.ui_update_queue.lock().unwrap();
    q.push_back(UIUpdate::AimedArea(None));
  }
}

#[cfg(test)]
mod tests {
  use super::super::test_helpers::make_playhead;
  use super::Direction;
  use crate::view::rect::Rect;
  use cursive::Vec2;

  #[test]
  fn start_aim_initialises_aimed_area_at_current_pos_and_size() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(3, 4);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(3, 4), (5, 2));

    ph.handle_start_aim();

    let aimed = ph.aimed_area.lock().unwrap();
    let aimed_area = aimed.expect("aimed_area should be Some after start_aim");
    assert_eq!(aimed_area.top_left(), Vec2::new(3, 4));
    assert_eq!(aimed_area.width(), 5);
    assert_eq!(aimed_area.height(), 2);
  }

  #[test]
  fn start_aim_when_already_aimed_overwrites_with_current_state() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(1, 1);
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::new(1, 1), (2, 2));
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(9, 9), (3, 3)));

    ph.handle_start_aim();

    let aimed = ph.aimed_area.lock().unwrap().unwrap();
    assert_eq!(aimed.top_left(), Vec2::new(1, 1));
    assert_eq!(aimed.width(), 2);
  }

  #[test]
  fn update_aim_right_shifts_top_left_by_step() {
    let ph = make_playhead();
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(2, 3), (2, 2)));

    ph.handle_update_aim(Direction::Right, Vec2::new(20, 20), 3);

    let aimed = ph.aimed_area.lock().unwrap().unwrap();
    assert_eq!(aimed.top_left(), Vec2::new(5, 3)); // x: 2+3=5
  }

  #[test]
  fn update_aim_down_shifts_top_left_by_step() {
    let ph = make_playhead();
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(2, 3), (2, 2)));

    ph.handle_update_aim(Direction::Down, Vec2::new(20, 20), 4);

    let aimed = ph.aimed_area.lock().unwrap().unwrap();
    assert_eq!(aimed.top_left(), Vec2::new(2, 7)); // y: 3+4=7
  }

  #[test]
  fn update_aim_clamps_at_zero_when_moving_left_past_origin() {
    let ph = make_playhead();
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(1, 0), (2, 2)));

    ph.handle_update_aim(Direction::Left, Vec2::new(20, 20), 5);

    let aimed = ph.aimed_area.lock().unwrap().unwrap();
    assert_eq!(aimed.top_left().x, 0); // saturates at 0
  }

  #[test]
  fn update_aim_clamps_at_canvas_boundary_when_moving_right() {
    let ph = make_playhead();
    // area width=3, so max_x = canvas_x(10) - size(3) = 7
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(6, 0), (3, 1)));

    ph.handle_update_aim(Direction::Right, Vec2::new(10, 10), 99);

    let aimed = ph.aimed_area.lock().unwrap().unwrap();
    assert_eq!(aimed.top_left().x, 7); // clamped to max_x=7
  }

  #[test]
  fn update_aim_preserves_area_size() {
    let ph = make_playhead();
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(0, 0), (4, 3)));

    ph.handle_update_aim(Direction::Right, Vec2::new(20, 20), 2);

    let aimed = ph.aimed_area.lock().unwrap().unwrap();
    assert_eq!(aimed.width(), 4);
    assert_eq!(aimed.height(), 3);
  }

  #[test]
  fn commit_aim_sets_pos_to_aimed_top_left_and_clears_aimed_area() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(0, 0);
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(5, 6), (2, 2)));

    ph.handle_commit_aim();

    assert_eq!(*ph.pos.lock().unwrap(), Vec2::new(5, 6));
    assert!(ph.aimed_area.lock().unwrap().is_none());
  }

  #[test]
  fn commit_aim_resets_actived_pos_to_zero() {
    let ph = make_playhead();
    *ph.actived_pos.lock().unwrap() = Vec2::new(4, 7);
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(1, 2), (2, 2)));

    ph.handle_commit_aim();

    assert_eq!(*ph.actived_pos.lock().unwrap(), Vec2::zero());
  }

  #[test]
  fn commit_aim_with_no_aimed_area_leaves_pos_unchanged() {
    let ph = make_playhead();
    *ph.pos.lock().unwrap() = Vec2::new(3, 3);
    *ph.aimed_area.lock().unwrap() = None;

    ph.handle_commit_aim();

    assert_eq!(*ph.pos.lock().unwrap(), Vec2::new(3, 3));
  }

  #[test]
  fn cancel_aim_clears_aimed_area() {
    let ph = make_playhead();
    *ph.aimed_area.lock().unwrap() = Some(Rect::from_size(Vec2::new(2, 2), (3, 3)));

    ph.handle_cancel_aim();

    assert!(ph.aimed_area.lock().unwrap().is_none());
  }

  #[test]
  fn cancel_aim_is_idempotent_when_already_none() {
    let ph = make_playhead();
    *ph.aimed_area.lock().unwrap() = None;

    ph.handle_cancel_aim(); // should not panic

    assert!(ph.aimed_area.lock().unwrap().is_none());
  }
}
