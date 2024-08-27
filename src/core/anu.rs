use cursive::{
  view::{Nameable, Resizable},
  views::{DummyView, LinearLayout, NamedView, RadioGroup},
};

use super::config;
use crate::view::{
  canvas_section::CanvasSection, middle_section::MiddleSection, top_section::TopSection,
};

#[derive(Clone, Default)]
pub struct Anu {
  pub boolean: bool,
  pub mode_state: RadioGroup<bool>,
  pub flag_state: RadioGroup<bool>,
  pub input_regex: String,
  pub show_regex_display: bool,
  pub top_section: TopSection,
}

impl Anu {
  pub fn new() -> Self {
    Anu {
      boolean: false,
      mode_state: RadioGroup::new(),
      flag_state: RadioGroup::new(),
      input_regex: String::new(),
      show_regex_display: false,
      top_section: TopSection::new(),
    }
  }

  pub fn build(&mut self) -> NamedView<LinearLayout> {
    let top_section = TopSection::build(self);
    let middle_section = MiddleSection::build();
    let canvas_section = CanvasSection::build();
    let padding_section = DummyView::new().fixed_width(1);

    LinearLayout::vertical()
      .child(top_section)
      .child(middle_section)
      .child(padding_section)
      .child(canvas_section)
      .with_name(config::main_section_view)
  }
}
