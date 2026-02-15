use cursive::{
  view::{Nameable, Resizable},
  views::{DummyView, LinearLayout, NamedView, RadioGroup},
};
use std::sync::{mpsc::Sender, Arc, RwLock};

use crate::{
  core::{config, midi, regex},
  view::common::{canvas_section::CanvasSection, marker},
};

use super::{
  middle_section::MiddleSection,
  top_section::{RegexFlag, RegexMode, TopSection},
};

#[derive(Clone)]
pub struct Anu {
  pub mode_state: RadioGroup<RegexMode>,
  pub flag_state: RadioGroup<RegexFlag>,
  pub input_regex: String,
  pub toggle_regex_input: Arc<RwLock<bool>>,
  pub top_section: TopSection,
}

impl Anu {
  pub fn new() -> Self {
    Anu {
      mode_state: RadioGroup::new(),
      flag_state: RadioGroup::new(),
      input_regex: String::new(),
      toggle_regex_input: Arc::new(RwLock::new(false)),
      top_section: TopSection::new(),
    }
  }

  pub fn build(
    &mut self,
    regex_tx: Sender<regex::Message>,
    midi_tx: Sender<midi::Message>,
    marker_tx: Sender<marker::Message>,
  ) -> NamedView<LinearLayout> {
    let top_section = TopSection::build(self, regex_tx);
    let middle_section = MiddleSection::build();
    let padding_section = DummyView::new().fixed_width(1);
    let canvas_section = CanvasSection::build(marker_tx);

    LinearLayout::vertical()
      .child(top_section)
      .child(middle_section)
      .child(padding_section)
      .child(canvas_section)
      .with_name(config::main_section_view)
  }

  pub fn set_toggle_regex_input(&self) {
    let mut toggle_regex_input = self.toggle_regex_input.write().unwrap();
    *toggle_regex_input = !*toggle_regex_input;
  }

  pub fn toggle_regex_input(&self) -> bool {
    *self.toggle_regex_input.read().unwrap()
  }
}
