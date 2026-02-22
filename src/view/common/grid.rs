use std::sync::mpsc::Sender;

use cursive::views::{Canvas, FocusTracker, NamedView, ResizedView};

use super::{grid_editor::GridEditor, playhead_controller};

pub struct GridSection {}

impl Default for GridSection {
  fn default() -> Self {
    Self::new()
  }
}

impl GridSection {
  pub fn new() -> Self {
    GridSection {}
  }

  pub fn build(
    playhead_tx: Sender<playhead_controller::Message>,
  ) -> FocusTracker<ResizedView<ResizedView<NamedView<Canvas<GridEditor>>>>> {
    FocusTracker::new(GridEditor::build(playhead_tx))
  }
}
