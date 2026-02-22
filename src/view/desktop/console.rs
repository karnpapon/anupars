use std::sync::mpsc::Sender;

use cfonts::{render, Fonts, Options};
use cursive::{
  event::EventResult,
  theme::Style,
  utils::span::SpannedString,
  view::{Nameable, Resizable},
  views::{Canvas, Dialog, EditView, FocusTracker, LinearLayout, ListView, NamedView, TextView},
  Cursive, Vec2,
};

use crate::{
  app::UserData,
  core::{consts, regex, utils},
  view::{common::grid_editor::CanvasEditor, microcontroller::console::RegexFlag},
};

use super::app::Anu;

pub enum RegexMode {
  Realtime,
  OnEval,
}

#[derive(Clone)]
pub struct TopSection {
  bpm: usize,
  ratio: (i64, usize),
  pos: Vec2,
  len: (usize, usize),
}

impl Default for TopSection {
  fn default() -> Self {
    Self::new()
  }
}

impl TopSection {
  pub fn new() -> Self {
    TopSection {
      bpm: 120,
      ratio: (1, 16),
      pos: Vec2::zero(),
      len: (1, 1),
    }
  }

  pub fn build(app: &mut Anu, regex_tx: Sender<regex::Message>) -> FocusTracker<NamedView<Dialog>> {
    let regex_tx_on_edit = regex_tx.clone();
    let regex_tx_on_submit = regex_tx.clone();
    let regex_input_unit_view = EditView::new()
      .content(app.input_regex.clone())
      .style(Style::highlight_inactive())
      .on_edit(move |siv: &mut Cursive, texts: &str, pos: usize| {
        input_edit(siv, texts, pos, regex_tx_on_edit.clone())
      })
      .on_submit(move |siv: &mut Cursive, texts: &str| {
        input_submit(siv, texts, regex_tx_on_submit.clone())
      })
      .with_name(consts::regex_input_unit_view)
      .fixed_width(25);

    let flag_view = LinearLayout::horizontal()
      .child(
        app
          .flag_state
          .button(RegexFlag::CaseSensitive, "i ")
          .selected(),
      )
      .child(app.flag_state.button(RegexFlag::Multiline, "m "))
      .child(app.flag_state.button(RegexFlag::Newline, "s "))
      .child(app.flag_state.button(RegexFlag::IgnoreWhiteSpace, "x "))
      .child(app.flag_state.button(RegexFlag::Lazy, "U "));
    // .with(|layout| {
    //   if app.boolean {
    //     layout.set_focus_index(1).unwrap();
    //   }
    // });

    let _mode_view = LinearLayout::horizontal()
      .child(
        app
          .mode_state
          .button(RegexMode::Realtime, "Realtime ")
          .selected(),
      )
      .child(app.mode_state.button(RegexMode::OnEval, "On-Eval "));

    let input_status_unit_view = TextView::new("-")
      .with_name(consts::input_status_unit_view)
      .max_width(25);

    let input_controller_section_view = ListView::new()
      .child("RegExp: ", regex_input_unit_view)
      // .child("Mode: ", mode_view)
      .child("flag: ", flag_view)
      .child("", TextView::new("").fixed_height(1))
      .child(
        "MIDI: ",
        TextView::new("-").with_name(consts::midi_status_unit_view),
      )
      .full_width();

    let status_controller_section_view = ListView::new()
      .child(
        "BPM: ",
        TextView::new(utils::build_bpm_status_str(app.top_section.bpm))
          .with_name(consts::bpm_status_unit_view),
      )
      .child(
        "RTO: ",
        TextView::new(utils::build_ratio_status_str(app.top_section.ratio, ""))
          .with_name(consts::ratio_status_unit_view),
      )
      .child(
        "LEN: ",
        TextView::new(utils::build_len_status_str(app.top_section.len))
          .with_name(consts::len_status_unit_view),
      )
      .child(
        "POS: ",
        TextView::new(utils::build_pos_status_str(app.top_section.pos))
          .with_name(consts::pos_status_unit_view),
      )
      .full_width();

    let protocol_controller_section_view = ListView::new()
      .child(
        "Mode:",
        TextView::new("raud").with_name(consts::osc_status_unit_view), // R=Reverse, A=Arpeggiator, U=Accumulation, D=Random
      )
      .child("State: ", input_status_unit_view)
      .child(
        "OPQ:",
        TextView::new("[]").with_name(consts::op_queue_status_unit_view),
      )
      .child(
        "EVQ:",
        TextView::new("[]").with_name(consts::ev_queue_status_unit_view),
      )
      .full_width();

    FocusTracker::new(
      Dialog::around(
        LinearLayout::horizontal()
          .child(input_controller_section_view.with_name(consts::input_controller_section_view))
          .child(status_controller_section_view.with_name(consts::status_controller_section_view))
          .child(
            protocol_controller_section_view.with_name(consts::protocol_controller_section_view),
          ),
      )
      .title_position(cursive::align::HAlign::Right)
      .with_name(consts::control_section_view),
    )
    .on_focus(|this| {
      this.get_mut().set_title(SpannedString::styled(
        format!(" {} ", consts::control_section_view),
        Style::highlight(),
      ));
      EventResult::consumed()
    })
    .on_focus_lost(|this| {
      this.get_mut().set_title("");
      EventResult::consumed()
    })
  }
}

fn solve_regex(siv: &mut Cursive, texts: &str, regex_tx: Sender<regex::Message>) {
  let mut canvas_editor_section_view = siv
    .find_name::<Canvas<CanvasEditor>>(consts::canvas_editor_section_view)
    .unwrap();
  let state = canvas_editor_section_view.state_mut();
  let text = state.text_contents();
  let grid_width = state.grid.width;

  let flag = siv
    .user_data::<UserData>()
    .map(|user_data| *user_data.cmd.anu.flag_state.selection())
    .unwrap_or(RegexFlag::CaseSensitive);

  let flag_str = match flag {
    RegexFlag::CaseSensitive => "i",
    RegexFlag::Multiline => "m",
    RegexFlag::Newline => "s",
    RegexFlag::IgnoreWhiteSpace => "x",
    RegexFlag::Lazy => "U",
  };

  let input_regex = regex::EventData {
    text,
    pattern: texts.to_string(),
    flags: flag_str.to_string(),
    grid_width,
  };

  regex_tx.send(regex::Message::Solve(input_regex)).unwrap()
}

fn input_submit(siv: &mut Cursive, texts: &str, regex_tx: Sender<regex::Message>) {
  solve_regex(siv, texts, regex_tx);
}

fn input_edit(siv: &mut Cursive, texts: &str, _cursor: usize, regex_tx: Sender<regex::Message>) {
  let mut display_view = siv.find_name::<TextView>(consts::display_view).unwrap();

  if texts.is_empty() {
    display_view.set_content(utils::build_doc_string(&consts::APP_WELCOME_MSG));
    regex_tx.send(regex::Message::Clear).unwrap();
    return;
  }

  let output = render(Options {
    text: String::from(texts),
    font: Fonts::FontTiny,
    ..Options::default()
  });

  display_view.set_content(output.text);

  solve_regex(siv, texts, regex_tx);
}
