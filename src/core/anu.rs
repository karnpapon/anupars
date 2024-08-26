use cursive::{
  event::{Event, EventResult},
  view::{Nameable, Resizable},
  views::{DummyView, LinearLayout, NamedView, RadioGroup},
  Printer, Vec2, View,
};

use super::config;
use crate::view::{
  canvas_section::CanvasSection, middle_section::MiddleSection, top_section::TopSection,
};

#[derive(Clone, Default)]
pub struct AnuData {
  pub boolean: bool,
  pub mode_state: RadioGroup<bool>,
  pub flag_state: RadioGroup<bool>,
  pub input_regex: String,
  pub show_regex_display: bool,
}

pub struct Anu {}

impl Default for Anu {
  fn default() -> Self {
    Self::new()
  }
}

impl cursive::view::View for Anu {
  fn draw(&self, _: &Printer) {
    println!(" Anu draw()")
  }

  fn on_event(&mut self, _: Event) -> EventResult {
    EventResult::Consumed(None)
  }

  fn layout(&mut self, size: Vec2) {
    println!("layout Anu = {:?}", size);
  }
}

impl Anu {
  pub fn new() -> Self {
    Anu {}
  }

  pub fn build(&mut self, current_data: &mut AnuData) -> NamedView<LinearLayout> {
    let top_section = TopSection::build(current_data);
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
