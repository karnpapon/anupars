use std::sync::mpsc::Sender;
use std::sync::Arc;

use crate::core::engine::regex;
use crate::core::engine::symspell::SymSpellState;
use crate::core::playhead::{Playhead, UIUpdate};
use crate::view::rect::Rect;

impl Playhead {
  pub(crate) fn enqueue_sym_space(&self) {
    let _ = self.ui_tx.send(UIUpdate::TmpAppendSpace);
  }

  pub(crate) fn enqueue_sym_buf_append(&self, idx: usize) {
    use std::sync::atomic::Ordering;
    if !self.modes.accumulation_mode.load(Ordering::Relaxed) {
      return;
    }
    let _ = self.ui_tx.send(UIUpdate::TmpAppend(idx));
  }

  pub(crate) fn enqueue_sym_rpl_cycle(&self, area: Rect) {
    let _ = self.ui_tx.send(UIUpdate::RplCycle(area));
  }
}

/// Apply symspell animation tick to the grid and app state.
/// Called from the event loop each frame when an animation is active.
#[cfg(not(target_arch = "wasm32"))]
pub fn apply_sym_anim_tick(
  sym_state: &Arc<SymSpellState>,
  grid: &mut crate::view::grid::GridEditor,
  app_state: &mut crate::state::AppState,
  regex_tx: &Sender<regex::Message>,
) {
  if let Some(tick) = sym_state.advance_anim_frame() {
    sym_state.render_anim_tick(grid, app_state, tick, regex_tx);
  }
}
