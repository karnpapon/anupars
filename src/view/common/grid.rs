use std::sync::mpsc::Sender;

use cursive::views::{Canvas, FocusTracker, NamedView, ResizedView};

use super::{grid_editor::CanvasEditor, playhead_controller};

pub struct CanvasSection {}

impl Default for CanvasSection {
  fn default() -> Self {
    Self::new()
  }
}

impl CanvasSection {
  pub fn new() -> Self {
    CanvasSection {}
  }

  pub fn build(
    marker_tx: Sender<playhead_controller::Message>,
  ) -> FocusTracker<ResizedView<ResizedView<NamedView<Canvas<CanvasEditor>>>>> {
    FocusTracker::new(CanvasEditor::build(marker_tx))
  }
}
