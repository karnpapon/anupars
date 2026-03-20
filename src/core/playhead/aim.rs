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
              editor.is_aiming = false;
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
              editor.is_aiming = false;
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
