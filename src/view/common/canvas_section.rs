use std::sync::mpsc::Sender;

use cursive::views::{Canvas, FocusTracker, NamedView, ResizedView};

use super::{canvas_editor::CanvasEditor, marker};

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
    marker_tx: Sender<marker::Message>,
  ) -> FocusTracker<ResizedView<ResizedView<NamedView<Canvas<CanvasEditor>>>>> {
    FocusTracker::new(CanvasEditor::build(marker_tx))
  }
}
