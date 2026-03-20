use std::sync::atomic::Ordering;

use cursive::views::{Canvas, TextView};

use crate::core::command::types;
use crate::core::tonal::scale;
use crate::core::{consts, utils};
use crate::view::grid::GridEditor;

use super::Playhead;

impl Playhead {
  pub fn cycle_scale_root(&self, dir: types::Adjustment) {
    let mut root = self.music.scale_root_top.lock().unwrap();
    *root = root.cycle(dir);
    let new_root = *root;
    drop(root);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.scale_root_top = new_root;
          },
        );
      }))
      .unwrap();
  }

  pub fn cycle_scale_mode(&self, dir: types::Adjustment) {
    let mut mode = self.music.scale_mode_top.lock().unwrap();
    *mode = mode.cycle(dir);
    let new_mode = *mode;
    drop(mode);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.scale_mode_top = new_mode;
          },
        );
      }))
      .unwrap();
  }

  pub(super) fn handle_set_scale_mode_left(&self, scale_mode: scale::ScaleMode) {
    let mut mode = self.music.scale_mode_left.lock().unwrap();
    *mode = scale_mode;
    drop(mode);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.scale_mode_left = scale_mode;
          },
        );
      }))
      .unwrap();
  }

  pub(super) fn handle_set_scale_mode_top(&self, scale_mode: scale::ScaleMode) {
    let mut mode = self.music.scale_mode_top.lock().unwrap();
    *mode = scale_mode;
    drop(mode);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.scale_mode_top = scale_mode;
          },
        );
      }))
      .unwrap();
  }

  pub(super) fn handle_set_scale_root_top(&self, scale_root: scale::ScaleRoot) {
    let mut root = self.music.scale_root_top.lock().unwrap();
    *root = scale_root;
    drop(root);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.scale_root_top = scale_root;
          },
        );
      }))
      .unwrap();
  }

  pub(super) fn handle_set_tempo(&self, bpm: usize) {
    self.music.tempo.store(bpm, Ordering::Relaxed);
  }

  pub(super) fn handle_set_ratio(&self, new_ratio: (usize, usize)) {
    let mut ratio = self.music.ratio.lock().unwrap();
    *ratio = new_ratio;
    drop(ratio);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::ratio_status_unit_view, |view: &mut TextView| {
          view.set_content(utils::build_ratio_status_str(new_ratio));
        });
      }))
      .unwrap();
  }
}
