use std::sync::mpsc::Sender;

use cursive::views::{Canvas, FocusTracker, NamedView, ResizedView};

use crate::core::midi;

use super::canvas_editor::CanvasEditor;

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
    midi_tx: Sender<midi::Message>,
  ) -> FocusTracker<ResizedView<ResizedView<NamedView<Canvas<CanvasEditor>>>>> {
    FocusTracker::new(CanvasEditor::build(midi_tx))
  }
}
