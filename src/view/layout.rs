use std::sync::{Arc, RwLock};

use crate::app::AppMode;
use crate::core::playhead::movement::Movement;

/// Shared program-level state used by CommandManager.
#[derive(Clone)]
pub struct Program {
  pub mode: AppMode,
  pub movement: Movement,
  pub input_regex: String,
  pub toggle_regex_input: Arc<RwLock<bool>>,
}

impl Default for Program {
  fn default() -> Self {
    Self::new()
  }
}

impl Program {
  pub fn new() -> Self {
    Program {
      mode: AppMode::None,
      movement: Movement::Forward,
      input_regex: String::new(),
      toggle_regex_input: Arc::new(RwLock::new(true)),
    }
  }

  pub fn set_toggle_regex_input(&self) {
    let mut v = self.toggle_regex_input.write().unwrap();
    *v = !*v;
  }

  pub fn toggle_regex_input(&self) -> bool {
    *self.toggle_regex_input.read().unwrap()
  }
}
