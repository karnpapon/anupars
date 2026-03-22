use std::sync::atomic::Ordering;
use std::sync::Arc;

use cursive::views::{Canvas, TextView};

use crate::core::command::types;
use crate::core::tonal::scale;
use crate::core::{consts, utils};
use crate::view::grid::GridEditor;

use super::Playhead;

impl Playhead {
  pub fn cycle_scale_root(&self, dir: types::Adjustment) {
    let root_top = Arc::clone(&self.music.scale_root_top);
    let root_left = Arc::clone(&self.music.scale_root_left);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            if editor.keyboard_top_active {
              let mut root = root_top.lock().unwrap();
              *root = root.cycle(dir);
              editor.scale_root_top = *root;
            } else {
              let mut root = root_left.lock().unwrap();
              *root = root.cycle(dir);
              editor.scale_root_left = *root;
            }
          },
        );
      }))
      .unwrap();
  }

  pub fn cycle_scale_mode(&self, dir: types::Adjustment) {
    let mode_top = Arc::clone(&self.music.scale_mode_top);
    let mode_left = Arc::clone(&self.music.scale_mode_left);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            if editor.keyboard_top_active {
              let mut mode = mode_top.lock().unwrap();
              *mode = mode.cycle(dir);
              editor.scale_mode_top = *mode;
            } else {
              let mut mode = mode_left.lock().unwrap();
              *mode = mode.cycle(dir);
              editor.scale_mode_left = *mode;
            }
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
