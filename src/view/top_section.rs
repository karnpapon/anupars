use cfonts::{render, Fonts, Options};
use cursive::{
  event::{Event, EventResult},
  theme::Style,
  utils::span::SpannedString,
  view::{Nameable, Resizable},
  views::{Dialog, EditView, FocusTracker, LinearLayout, ListView, NamedView, TextView},
  Cursive, Printer, Vec2, View, With,
};

use crate::core::{anu::Anu, config, regex, utils};

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

  pub fn build(app: &mut Anu) -> FocusTracker<NamedView<Dialog>> {
    let regex_input_unit_view = EditView::new()
      .content(app.input_regex.clone())
      .style(Style::highlight_inactive())
      .on_edit(input_edit)
      .on_submit(input_submit)
      .with_name(config::regex_input_unit_view)
      .fixed_width(25);

    let flag_view = LinearLayout::horizontal()
      .child(app.flag_state.button(true, "i"))
      .child(app.flag_state.button(false, "m"))
      .child(app.flag_state.button(false, "s"))
      .child(app.flag_state.button(false, "x"))
      .child(app.flag_state.button(false, "U"))
      .with(|layout| {
        if app.boolean {
          layout.set_focus_index(1).unwrap();
        }
      });

    let mode_view = LinearLayout::horizontal()
      .child(app.mode_state.button(false, "Realtime"))
      .child(
        app
          .mode_state
          .button(true, "Lazy")
          .with_if(app.boolean, |button| {
            button.select();
          }),
      )
      .with(|layout| {
        if app.boolean {
          layout.set_focus_index(1).unwrap();
        }
      });

    let input_status_unit_view = TextView::new("-")
      .with_name(config::input_status_unit_view)
      .max_width(25);

    let input_controller_section_view = ListView::new()
      .child("RegExp: ", regex_input_unit_view)
      .child("Mode: ", mode_view)
      .child("flag: ", flag_view)
      .child("status: ", input_status_unit_view)
      .full_width();

    let status_controller_section_view = ListView::new()
      .child(
        "BPM: ",
        TextView::new(utils::build_bpm_status_str(app.top_section.bpm))
          .with_name(config::bpm_status_unit_view),
      )
      .child(
        "RTO: ",
        TextView::new(utils::build_ratio_status_str(app.top_section.ratio, ""))
          .with_name(config::ratio_status_unit_view),
      )
      .child(
        "LEN: ",
        TextView::new(utils::build_len_status_str(app.top_section.len))
          .with_name(config::len_status_unit_view),
      )
      .child(
        "POS: ",
        TextView::new(utils::build_pos_status_str(app.top_section.pos))
          .with_name(config::pos_status_unit_view),
      )
      .full_width();

    let protocol_controller_section_view = ListView::new()
      .child(
        "OSC: ",
        TextView::new("-")
          .with_name(config::osc_status_unit_view)
          .fixed_width(8),
      )
      .child(
        "MIDI: ",
        TextView::new("-").with_name(config::midi_status_unit_view),
      )
      .full_width();

    FocusTracker::new(
      Dialog::around(
        LinearLayout::horizontal()
          .child(input_controller_section_view.with_name(config::input_controller_section_view))
          .child(status_controller_section_view.with_name(config::status_controller_section_view))
          .child(
            protocol_controller_section_view.with_name(config::protocol_controller_section_view),
          ),
      )
      .title_position(cursive::align::HAlign::Right)
      .with_name(config::control_section_view),
    )
    .on_focus(|this| {
      this.get_mut().set_title(SpannedString::styled(
        format!(" {} ", config::control_section_view),
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

fn input_submit(siv: &mut Cursive, texts: &str) {
  let mut input_status_unit_view = siv
    .find_name::<TextView>(config::input_status_unit_view)
    .unwrap();
  input_status_unit_view.set_content(texts);
  regex::solve();
}

fn input_edit(siv: &mut Cursive, texts: &str, _cursor: usize) {
  let output = render(Options {
    text: String::from(texts),
    font: Fonts::FontTiny,
    ..Options::default()
  });

  let mut regex_display_unit_view = siv
    .find_name::<TextView>(config::regex_display_unit_view)
    .unwrap();

  regex_display_unit_view.set_content(output.text);
}
