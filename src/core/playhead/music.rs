use std::sync::atomic::Ordering;
use std::sync::Arc;

use cursive::views::{Canvas, TextView};

use crate::core::command::types;
use crate::core::tonal::scale;
use crate::core::{consts, utils};
use crate::view::grid::GridEditor;

use super::Playhead;

impl Playhead {
  fn retrigger_drone_if_active(&self) {
    if !self.modes.drone_mode.load(Ordering::Relaxed) {
      return;
    }
    let drone_x = self.modes.drone_x.load(Ordering::Relaxed);
    let area = *self.area.lock().unwrap();
    self.midi_handler.trigger_drone_at_x(drone_x, &area);
  }

  pub fn cycle_scale_root(&self, dir: types::Adjustment) {
    let root_top = Arc::clone(&self.music.scale_root_top);
    let root_left = Arc::clone(&self.music.scale_root_left);

    let midi_handler = Arc::clone(&self.midi_handler);
    let drone_mode = Arc::clone(&self.modes.drone_mode);
    let drone_x = Arc::clone(&self.modes.drone_x);
    let area = Arc::clone(&self.area);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            if editor.playhead_ui.keyboard_top_active {
              let mut root = root_top.lock().unwrap();
              *root = root.cycle(dir);
              editor.playhead_ui.scale_root_top = *root;
            } else {
              let mut root = root_left.lock().unwrap();
              *root = root.cycle(dir);
              editor.playhead_ui.scale_root_left = *root;
              drop(root);
              if drone_mode.load(Ordering::Relaxed) {
                let x = drone_x.load(Ordering::Relaxed);
                let a = *area.lock().unwrap();
                midi_handler.trigger_drone_at_x(x, &a);
              }
            }
          },
        );
      }))
      .unwrap();
  }

  pub fn cycle_scale_mode(&self, dir: types::Adjustment) {
    let mode_top = Arc::clone(&self.music.scale_mode_top);
    let mode_left = Arc::clone(&self.music.scale_mode_left);

    let midi_handler = Arc::clone(&self.midi_handler);
    let drone_mode = Arc::clone(&self.modes.drone_mode);
    let drone_x = Arc::clone(&self.modes.drone_x);
    let area = Arc::clone(&self.area);

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            if editor.playhead_ui.keyboard_top_active {
              let mut mode = mode_top.lock().unwrap();
              *mode = mode.cycle(dir);
              editor.playhead_ui.scale_mode_top = *mode;
            } else {
              let mut mode = mode_left.lock().unwrap();
              *mode = mode.cycle(dir);
              editor.playhead_ui.scale_mode_left = *mode;
              drop(mode);
              if drone_mode.load(Ordering::Relaxed) {
                let x = drone_x.load(Ordering::Relaxed);
                let a = *area.lock().unwrap();
                midi_handler.trigger_drone_at_x(x, &a);
              }
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

    self.retrigger_drone_if_active();

    let cb_sink = self.cb_sink.clone();
    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          move |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.scale_mode_left = scale_mode;
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
            editor.playhead_ui.scale_mode_top = scale_mode;
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
            editor.playhead_ui.scale_root_top = scale_root;
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
