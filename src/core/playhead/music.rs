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
      self.cycle_scale_root_left(dir);
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
      self.cycle_scale_mode_left(dir);
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::core::command::types::Adjustment;
  use crate::core::playhead::test_helpers::make_playhead;
  use crate::core::tonal::scale::{ScaleMode, ScaleRoot};
  use std::sync::atomic::Ordering;

  #[test]
  fn cycle_scale_root_left_advances_through_the_root_list() {
    let playhead = make_playhead();
    playhead.cycle_scale_root_left(Adjustment::Increase);
    assert_eq!(
      *playhead.music.scale_root_left.lock().unwrap(),
      ScaleRoot::CSharp
    );
  }

  #[test]
  fn cycle_scale_mode_left_advances_through_the_mode_list() {
    let playhead = make_playhead();
    playhead.cycle_scale_mode_left(Adjustment::Increase);
    assert_eq!(
      *playhead.music.scale_mode_left.lock().unwrap(),
      ScaleMode::Major
    );
  }

  #[test]
  fn cycle_scale_root_routes_to_top_or_left_by_keyboard_focus() {
    let playhead = make_playhead();

    playhead
      .modes
      .keyboard_top_active
      .store(true, Ordering::Relaxed);
    playhead.cycle_scale_root(Adjustment::Increase);
    assert_eq!(
      *playhead.music.scale_root_top.lock().unwrap(),
      ScaleRoot::CSharp
    );
    assert_eq!(
      *playhead.music.scale_root_left.lock().unwrap(),
      ScaleRoot::C
    );

    playhead
      .modes
      .keyboard_top_active
      .store(false, Ordering::Relaxed);
    playhead.cycle_scale_root(Adjustment::Increase);
    assert_eq!(
      *playhead.music.scale_root_left.lock().unwrap(),
      ScaleRoot::CSharp
    );
    // the top keyboard's root is untouched by the left-routed call
    assert_eq!(
      *playhead.music.scale_root_top.lock().unwrap(),
      ScaleRoot::CSharp
    );
  }

  #[test]
  fn cycle_scale_mode_routes_to_top_or_left_by_keyboard_focus() {
    let playhead = make_playhead();

    playhead
      .modes
      .keyboard_top_active
      .store(false, Ordering::Relaxed);
    playhead.cycle_scale_mode(Adjustment::Increase);
    assert_eq!(
      *playhead.music.scale_mode_left.lock().unwrap(),
      ScaleMode::Major
    );
    assert_eq!(
      *playhead.music.scale_mode_top.lock().unwrap(),
      ScaleMode::Chromatic
    );
  }

  #[test]
  fn handle_set_tempo_stores_the_bpm() {
    let playhead = make_playhead();
    playhead.handle_set_tempo(140);
    assert_eq!(playhead.music.tempo.load(Ordering::Relaxed), 140);
  }

  #[test]
  fn handle_set_ratio_leaves_step_index_alone_when_easing_is_off() {
    let playhead = make_playhead();
    *playhead.step_index.lock().unwrap() = 5;

    playhead.handle_set_ratio((3, 4));

    assert_eq!(*playhead.music.ratio.lock().unwrap(), (3, 4));
    assert_eq!(*playhead.step_index.lock().unwrap(), 5);
  }

  #[test]
  fn handle_set_ratio_resets_step_index_when_easing_is_active() {
    let playhead = make_playhead();
    *playhead.easing_mode.lock().unwrap() = EasingMode::EaseIn;
    *playhead.step_index.lock().unwrap() = 5;

    playhead.handle_set_ratio((3, 4));

    assert_eq!(*playhead.music.ratio.lock().unwrap(), (3, 4));
    assert_eq!(*playhead.step_index.lock().unwrap(), 0);
  }

  #[test]
  fn direct_setters_overwrite_regardless_of_previous_value() {
    let playhead = make_playhead();
    playhead.cycle_scale_mode_left(Adjustment::Increase); // move it off the default first

    playhead.handle_set_scale_mode_left(ScaleMode::Blues);
    assert_eq!(
      *playhead.music.scale_mode_left.lock().unwrap(),
      ScaleMode::Blues
    );

    playhead.handle_set_scale_mode_top(ScaleMode::Dorian);
    assert_eq!(
      *playhead.music.scale_mode_top.lock().unwrap(),
      ScaleMode::Dorian
    );

    playhead.handle_set_scale_root_top(ScaleRoot::G);
    assert_eq!(*playhead.music.scale_root_top.lock().unwrap(), ScaleRoot::G);
  }

  #[test]
  fn drone_retrigger_path_does_not_panic_when_drone_is_active() {
    let playhead = make_playhead();
    playhead.modes.drone_mode.store(true, Ordering::Relaxed);
    playhead.cycle_scale_root_left(Adjustment::Increase);
  }
}
