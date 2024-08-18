use std::{borrow::BorrowMut, ops::DerefMut, sync::Arc};

use cfonts::{render, Fonts, Options};
use cursive::{
  event::{Event, EventResult},
  theme::Style,
  utils::span::SpannedString,
  view::{Nameable, Resizable, ViewWrapper},
  views::{
    Dialog, DialogFocus, EditView, FocusTracker, LinearLayout, ListView, NamedView, RadioGroup,
    TextView,
  },
  Cursive, Printer, View, With,
};

use super::{canvas::CanvasView, config, utils};

#[derive(Clone, Default)]
pub struct ControllerData {
  pub boolean: bool,
  pub string: String,
  pub number: usize,
  pub mode_state: RadioGroup<bool>,
  pub flag_state: RadioGroup<bool>,
  pub input_regex: String,
  pub show_regex_display: bool,
}

pub struct Controller {}

impl Default for Controller {
  fn default() -> Self {
    Self::new()
  }
}

impl View for Controller {
  fn draw(&self, _: &Printer) {}

  fn on_event(&mut self, _: Event) -> EventResult {
    EventResult::Consumed(None)
  }
}

impl Controller {
  pub fn new() -> Self {
    Controller {}
  }

  pub fn build(&mut self, current_data: &mut ControllerData) -> NamedView<LinearLayout> {
    let regex_view = EditView::new()
      .content(current_data.string.clone())
      .on_edit(input_edit)
      .on_submit(input_submit)
      .with_name("ctr_regex")
      .fixed_width(25);

    let flag_view = LinearLayout::horizontal()
      .child(current_data.flag_state.button(true, "g"))
      .child(current_data.flag_state.button(false, "i"))
      .child(current_data.flag_state.button(false, "m"))
      .child(current_data.flag_state.button(false, "u"))
      .child(current_data.flag_state.button(false, "s"))
      .child(current_data.flag_state.button(false, "y"))
      .with(|layout| {
        if current_data.boolean {
          layout.set_focus_index(1).unwrap();
        }
      });

    let mode_view =
      LinearLayout::horizontal()
        .child(current_data.mode_state.button(false, "Realtime"))
        .child(current_data.mode_state.button(true, "True").with_if(
          current_data.boolean,
          |button| {
            button.select();
          },
        ))
        .with(|layout| {
          if current_data.boolean {
            layout.set_focus_index(1).unwrap();
          }
        });

    let input_status_view = TextView::new("-")
      .with_name("input_status_view")
      .max_width(25);

    let input_controller = ListView::new()
      .child("RegExp: ", regex_view)
      .child("Mode: ", mode_view)
      .child("flag: ", flag_view)
      .child("status: ", input_status_view)
      .full_width();

    let status_view = ListView::new()
      .child("BPM: ", TextView::new("-").with_name("ctr_bpm"))
      .child("RTO: ", TextView::new("-").with_name("ctr_ratio"))
      .child("LEN: ", TextView::new("-").with_name("ctr_len"))
      .child("POS: ", TextView::new("-").with_name("ctr_pos"))
      .full_width();

    let protocol_view = ListView::new()
      .child(
        "OSC: ",
        TextView::new("-").with_name("ctr_osc").fixed_width(8),
      )
      .child("MIDI: ", TextView::new("-").with_name("ctr_midi"))
      .full_width();

    LinearLayout::vertical()
      .child(
        FocusTracker::new(
          Dialog::around(
            LinearLayout::horizontal()
              .child(input_controller.with_name("input_controller"))
              .child(status_view.with_name("status_view"))
              .child(protocol_view.with_name("protocol_view")),
          )
          .title_position(cursive::align::HAlign::Right)
          .with_name("control_section_view"),
        )
        .on_focus(|this| {
          this.get_mut().set_title(SpannedString::styled(
            " control_section_view ",
            Style::highlight(),
          ));
          EventResult::consumed()
        })
        .on_focus_lost(|this| {
          this.get_mut().set_title("");
          EventResult::consumed()
        }),
      )
      .child(
        FocusTracker::new(
          Dialog::around(
            TextView::new(utils::build_doc_string(&config::APP_WELCOME_MSG))
              .center()
              .with_name("regex_display_view")
              .fixed_height(6),
          )
          .title_position(cursive::align::HAlign::Right)
          .with_name("interactive_display_view"),
        )
        .on_focus(|this| {
          this.get_mut().set_title(SpannedString::styled(
            " interactive_display_view ",
            Style::highlight(),
          ));
          EventResult::consumed()
        })
        .on_focus_lost(|this| {
          this.get_mut().set_title("");
          EventResult::consumed()
        }),
      )
      .child(
        FocusTracker::new(
          CanvasView::new()
            .with_name("canvas_section_view")
            .full_width()
            .full_height(),
        )
        .on_focus(|_| EventResult::with_cb(|s| {}))
        .on_focus_lost(|_| EventResult::with_cb(|s| {})),
      )
      .with_name("main_view")
  }
}

fn input_submit(siv: &mut Cursive, texts: &str) {
  let mut text_view = siv.find_name::<TextView>("input_status_view").unwrap();
  text_view.set_content(texts);
}

fn input_edit(siv: &mut Cursive, texts: &str, _cursor: usize) {
  let output = render(Options {
    text: String::from(texts),
    font: Fonts::FontTiny,
    ..Options::default()
  });

  let mut text_view = siv.find_name::<TextView>("regex_display_view").unwrap();

  text_view.set_content(output.text);
}
