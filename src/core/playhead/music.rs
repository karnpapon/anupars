use std::sync::atomic::Ordering;

use crate::core::command::types;
use crate::core::tonal::scale;
use crate::core::utils;

use super::easing::EasingMode;
use super::types::UIUpdate;
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
    let keyboard_top = self.modes.keyboard_top_active.load(Ordering::Relaxed);
    if keyboard_top {
      let mut root = self.music.scale_root_top.lock().unwrap();
      *root = root.cycle(dir);
      let new_root = *root;
      drop(root);
      let _ = self.ui_tx.send(UIUpdate::CanvasScaleRootTop(new_root));
    } else {
      let mut root = self.music.scale_root_left.lock().unwrap();
      *root = root.cycle(dir);
      let new_root = *root;
      drop(root);
      let _ = self.ui_tx.send(UIUpdate::CanvasScaleRootLeft(new_root));
      self.retrigger_drone_if_active();
    }
  }

  pub fn cycle_scale_mode(&self, dir: types::Adjustment) {
    let keyboard_top = self.modes.keyboard_top_active.load(Ordering::Relaxed);
    if keyboard_top {
      let mut mode = self.music.scale_mode_top.lock().unwrap();
      *mode = mode.cycle(dir);
      let new_mode = *mode;
      drop(mode);
      let _ = self.ui_tx.send(UIUpdate::CanvasScaleModeTop(new_mode));
    } else {
      let mut mode = self.music.scale_mode_left.lock().unwrap();
      *mode = mode.cycle(dir);
      let new_mode = *mode;
      drop(mode);
      let _ = self.ui_tx.send(UIUpdate::CanvasScaleModeLeft(new_mode));
      self.retrigger_drone_if_active();
    }
  }

  pub fn cycle_scale_root_left(&self, dir: types::Adjustment) {
    let mut root = self.music.scale_root_left.lock().unwrap();
    *root = root.cycle(dir);
    let new_root = *root;
    drop(root);
    let _ = self.ui_tx.send(UIUpdate::CanvasScaleRootLeft(new_root));
    self.retrigger_drone_if_active();
  }

  pub fn cycle_scale_mode_left(&self, dir: types::Adjustment) {
    let mut mode = self.music.scale_mode_left.lock().unwrap();
    *mode = mode.cycle(dir);
    let new_mode = *mode;
    drop(mode);
    let _ = self.ui_tx.send(UIUpdate::CanvasScaleModeLeft(new_mode));
    self.retrigger_drone_if_active();
  }

  pub(super) fn handle_set_scale_mode_left(&self, scale_mode: scale::ScaleMode) {
    let mut mode = self.music.scale_mode_left.lock().unwrap();
    *mode = scale_mode;
    drop(mode);
    self.retrigger_drone_if_active();
    let _ = self.ui_tx.send(UIUpdate::CanvasScaleModeLeft(scale_mode));
  }

  pub(super) fn handle_set_scale_mode_top(&self, scale_mode: scale::ScaleMode) {
    let mut mode = self.music.scale_mode_top.lock().unwrap();
    *mode = scale_mode;
    drop(mode);
    let _ = self.ui_tx.send(UIUpdate::CanvasScaleModeTop(scale_mode));
  }

  pub(super) fn handle_set_scale_root_top(&self, scale_root: scale::ScaleRoot) {
    let mut root = self.music.scale_root_top.lock().unwrap();
    *root = scale_root;
    drop(root);
    let _ = self.ui_tx.send(UIUpdate::CanvasScaleRootTop(scale_root));
  }

  pub(super) fn handle_set_tempo(&self, bpm: usize) {
    self.music.tempo.store(bpm, Ordering::Relaxed);
  }

  pub(super) fn handle_set_ratio(&self, new_ratio: (usize, usize)) {
    let mut ratio = self.music.ratio.lock().unwrap();
    *ratio = new_ratio;
    drop(ratio);
    // When easing is active, snap the playhead back to the initial position
    // so the easing curve restarts cleanly from step 0.
    if *self.easing_mode.lock().unwrap() != EasingMode::None {
      let mut step_idx = self.step_index.lock().unwrap();
      *step_idx = 0;
      drop(step_idx);
      self.set_actived_pos(0);
      let pos = *self.actived_pos.lock().unwrap();
      self.update_active_pos_ui(pos);
    }
    let _ = self
      .ui_tx
      .send(UIUpdate::RatioStatus(utils::build_ratio_status_str(
        new_ratio,
      )));
  }
}
