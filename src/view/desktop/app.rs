use cursive::view::Nameable;
use cursive::view::Resizable;
use cursive::views::DummyView;
use cursive::views::LinearLayout;
use cursive::views::NamedView;
use cursive::views::RadioGroup;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::RwLock;

use crate::app::AppMode;
use crate::app::Movement;
use crate::core::consts;
use crate::core::regex;
use crate::view::common::grid::GridSection;
use crate::view::common::playhead_controller;
use crate::view::microcontroller::console::RegexFlag;

use super::console::RegexMode;
use super::console::TopSection;
use super::display::MiddleSection;

#[derive(Clone)]
pub struct Anu {
  pub mode: AppMode,
  pub movement: Movement,
  pub regex_mode_state: RadioGroup<RegexMode>,
  pub regex_flag_state: RadioGroup<RegexFlag>,
  pub input_regex: String,
  pub toggle_regex_input: Arc<RwLock<bool>>,
  pub top_section: TopSection,
}

impl Anu {
  pub fn new() -> Self {
    Anu {
      regex_mode_state: RadioGroup::new(),
      regex_flag_state: RadioGroup::new(),
      input_regex: String::new(),
      toggle_regex_input: Arc::new(RwLock::new(false)),
      top_section: TopSection::new(),
      mode: AppMode::None,
      movement: Movement::Forward,
    }
  }

  pub fn build(
    &mut self,
    regex_tx: Sender<regex::Message>,
    playhead_tx: Sender<playhead_controller::Message>,
  ) -> NamedView<LinearLayout> {
    let top_section = TopSection::build(self, regex_tx);
    let middle_section = MiddleSection::build();
    let padding_section = DummyView::new().fixed_width(1);
    let canvas_section = GridSection::build(playhead_tx);

    LinearLayout::vertical()
      .child(top_section)
      .child(middle_section)
      .child(padding_section)
      .child(canvas_section)
      .with_name(consts::main_section_view)
  }

  pub fn set_toggle_regex_input(&self) {
    let mut toggle_regex_input = self.toggle_regex_input.write().unwrap();
    *toggle_regex_input = !*toggle_regex_input;
  }

  pub fn toggle_regex_input(&self) -> bool {
    *self.toggle_regex_input.read().unwrap()
  }
}
