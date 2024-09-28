use cursive::{
  event::EventResult,
  view::{Nameable, Resizable, ViewWrapper},
  views::{DummyView, LinearLayout, NamedView, RadioGroup},
  wrap_impl, Printer, View,
};
// use std::sync::mpsc::{channel, Receiver, Sender};
use std::{
  sync::mpsc::{channel, Receiver, Sender},
  thread,
};

use super::{
  clock::{clock, metronome},
  config, regex,
};
use crate::view::{
  canvas_section::CanvasSection,
  middle_section::MiddleSection,
  top_section::{RegexFlag, RegexMode, TopSection},
};

#[derive(Clone, Copy, Debug)]
pub enum Message {
  Time(clock::Time),
  Signature(clock::Signature),
  Tempo(clock::Tempo),
  Start,
  Pause,
}

#[derive(Clone)]
pub struct Anu {
  pub mode_state: RadioGroup<RegexMode>,
  pub flag_state: RadioGroup<RegexFlag>,
  pub input_regex: String,
  pub toggle_regex_input: bool,
  pub top_section: TopSection,
}

// impl ViewWrapper for Anu {
//   wrap_impl!(self.toggle_regex_input: bool);
// }

impl Anu {
  pub fn new() -> Self {
    Anu {
      mode_state: RadioGroup::new(),
      flag_state: RadioGroup::new(),
      input_regex: String::new(),
      toggle_regex_input: false,
      top_section: TopSection::new(),
    }
  }

  pub fn build(&mut self, regex_tx: Sender<regex::Message>) -> NamedView<LinearLayout> {
    let top_section = TopSection::build(self, regex_tx);
    let middle_section = MiddleSection::build();
    let padding_section = DummyView::new().fixed_width(1);
    let canvas_section = CanvasSection::build();
    // let canvas_section = CanvasSection::build().on_focus(|_| {
    //   cursive::event::EventResult::with_cb(|s| {
    //     s.call_on_name(config::main_section_view, |v: &mut Anu| {
    //       v.toggle_regex_input = false;
    //     });
    //   })
    // });

    LinearLayout::vertical()
      .child(top_section)
      .child(middle_section)
      .child(padding_section)
      .child(canvas_section)
      .with_name(config::main_section_view)
  }
}
