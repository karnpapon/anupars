use cursive::views::{stack_view::Transparent, FocusTracker, StackView};

use super::{canvas_base::CanvasBase, canvas_editor::CanvasEditor};

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

  pub fn build() -> FocusTracker<StackView> {
    FocusTracker::new(
      StackView::new()
        .layer(Transparent(CanvasBase::build()))
        .layer(Transparent(CanvasEditor::build())),
    )
  }
}
