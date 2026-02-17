use crate::{
  core::{
    application::UserData,
    consts,
    midi::{self, MidiMsg},
    parser::{self},
    utils,
  },
  view::common::canvas_editor::CanvasEditor,
};
use cursive::{
  theme::Style,
  view::{Nameable, Resizable},
  views::{
    Button, Canvas, Dialog, DummyView, EditView, FocusTracker, LinearLayout, ListView, NamedView,
    PaddedView, ResizedView, TextView,
  },
  Cursive,
};
use cursive_tabs::{Align, TabPanel};
use std::sync::Arc;

use std::sync::mpsc::Sender;

use cfonts::{render, Fonts, Options};
use cursive::Vec2;

use crate::core::regex;

use super::anu::Anu;

pub enum RegexFlag {
  CaseSensitive,
  Multiline,
  Newline,
  IgnoreWhiteSpace,
  Lazy,
}

pub enum RegexMode {
  Realtime,
  OnEval,
}

#[derive(Clone)]
pub struct Console {
  bpm: usize,
  ratio: (i64, usize),
  pos: Vec2,
  len: (usize, usize),
}

impl Default for Console {
  fn default() -> Self {
    Self::new()
  }
}

impl Console {
  pub fn new() -> Self {
    Console {
      bpm: 120,
      ratio: (1, 16),
      pos: Vec2::zero(),
      len: (1, 1),
    }
  }

  pub fn build_main(app: &mut Anu, regex_tx: Sender<regex::Message>) -> NamedView<LinearLayout> {
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
      .min_width(10);

    let flag_view = LinearLayout::horizontal()
      .child(
        app
          .flag_state
          .button(RegexFlag::CaseSensitive, "i ")
          .selected(),
      )
      .child(app.flag_state.button(RegexFlag::Multiline, "m"));
    // .child(app.flag_state.button(RegexFlag::Newline, "s"))
    // .child(app.flag_state.button(RegexFlag::IgnoreWhiteSpace, "x"))
    // .child(app.flag_state.button(RegexFlag::Lazy, "U"));

    let mode_view = LinearLayout::horizontal()
      .child(app.mode_state.button(RegexMode::Realtime, "RT").selected())
      .child(app.mode_state.button(RegexMode::OnEval, "OE"));

    let input_status_unit_view = TextView::new("-").with_name(consts::input_status_unit_view);

    let input_controller_section_view = ListView::new()
      .child("RegExp: ", regex_input_unit_view)
      .child("Mode: ", mode_view)
      .child("flag: ", flag_view)
      .child("status: ", input_status_unit_view)
      .full_width()
      .max_width(30)
      .min_width(10);

    let status_controller_section_view = ListView::new()
      .child(
        "BPM:",
        TextView::new(utils::build_bpm_status_str(app.top_section.bpm))
          .with_name(consts::bpm_status_unit_view),
      )
      .child(
        "RTO:",
        TextView::new(utils::build_ratio_status_str(app.top_section.ratio, ""))
          .with_name(consts::ratio_status_unit_view),
      )
      .child(
        "LEN:",
        TextView::new(utils::build_len_status_str(app.top_section.len))
          .with_name(consts::len_status_unit_view),
      )
      .child(
        "POS:",
        TextView::new(utils::build_pos_status_str(app.top_section.pos))
          .with_name(consts::pos_status_unit_view),
      )
      .full_width()
      .max_width(20)
      .min_width(10);

    let protocol_controller_section_view = ListView::new()
      .child(
        "OSC:",
        TextView::new("-").with_name(consts::osc_status_unit_view), // .fixed_width(8),
      )
      .child(
        "MIDI:",
        TextView::new("-").with_name(consts::midi_status_unit_view),
      )
      .fixed_width(10);

    let padding_section_1 = DummyView::new().fixed_width(2);
    let padding_section_2 = DummyView::new().fixed_width(2);

    LinearLayout::horizontal()
      .child(DummyView.fixed_width(1))
      .child(
        LinearLayout::vertical()
          .child(DummyView.fixed_height(1))
          .child(
            LinearLayout::horizontal()
              .child(input_controller_section_view.with_name(consts::input_controller_section_view))
              .child(padding_section_1)
              .child(
                status_controller_section_view.with_name(consts::status_controller_section_view),
              )
              .child(padding_section_2)
              .child(
                protocol_controller_section_view
                  .with_name(consts::protocol_controller_section_view),
              ),
          )
          .child(DummyView.fixed_height(1)),
      )
      .child(DummyView.fixed_width(1))
      .with_name(consts::control_section_view)

    // FocusTracker::new(
    //   Dialog::around(
    //     LinearLayout::horizontal()
    //       .child(input_controller_section_view.with_name(config::input_controller_section_view))
    //       .child(status_controller_section_view.with_name(config::status_controller_section_view))
    //       .child(
    //         protocol_controller_section_view.with_name(config::protocol_controller_section_view),
    //       ),
    //   )
    //   .title_position(cursive::align::HAlign::Right)
    //   .with_name(config::control_section_view),
    // )
    // .on_focus(|this| {
    //   this.get_mut().set_title(SpannedString::styled(
    //     format!(" {} ", config::control_section_view),
    //     Style::highlight(),
    //   ));
    //   EventResult::consumed()
    // })
    // .on_focus_lost(|this| {
    //   this.get_mut().set_title("");
    //   EventResult::consumed()
    // })
  }

  fn build_welcome_msg() -> NamedView<TextView> {
    TextView::new(utils::build_doc_string(&consts::APP_WELCOME_MSG))
      .center()
      .with_name(consts::display_view)
  }

  fn build_midi_input() -> NamedView<PaddedView<LinearLayout>> {
    PaddedView::lrtb(
      10,
      10,
      1,
      1,
      LinearLayout::vertical()
        .child(DummyView::new().full_height())
        .child(
          LinearLayout::horizontal()
            .child(
              ListView::new()
                .child(
                  " N:",
                  EditView::new()
                    .content("")
                    // .filler('X')
                    .style(Style::highlight_inactive())
                    .with_name("midi_note"),
                )
                .full_width(),
            )
            .child(
              ListView::new()
                .child(
                  " L:",
                  EditView::new()
                    .content("")
                    .style(Style::highlight_inactive())
                    .with_name("midi_len"),
                )
                .full_width(),
            )
            .child(
              ListView::new()
                .child(
                  " V:",
                  EditView::new()
                    .content("")
                    .style(Style::highlight_inactive())
                    .with_name("midi_vel"),
                )
                .full_width(),
            )
            .child(
              ListView::new()
                .child(
                  " C:",
                  EditView::new()
                    .content("")
                    .style(Style::highlight_inactive())
                    .with_name("midi_chan"),
                )
                .full_width(),
            )
            .child(
              Button::new_raw("[SET]", |s| {
                let nte = get_input_msg(s, "midi_note");
                let len = get_input_msg(s, "midi_len");
                let vel = get_input_msg(s, "midi_vel");
                let chn = get_input_msg(s, "midi_chan");
                let mut midi_msg_list = Vec::new();

                if [&nte, &len, &vel, &chn].iter().any(|s| s.is_empty()) {
                  println!("midi msg should not left blank");
                } else {
                  let midi_msg_str = [&nte, &len, &vel, &chn]
                    .iter()
                    .map(|arc_str| arc_str.as_str()) // Convert Arc<String> to &str
                    .collect::<Vec<&str>>()
                    .join(" ");

                  match parser::midi::parser::parse_midi_msg(&midi_msg_str) {
                    Ok((_remaining, (note_n_oct, length, velocity, channel))) => {
                      for (note, octave) in note_n_oct {
                        let note_idx = [
                          "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
                        ]
                        .iter()
                        .position(|nte| nte == &note)
                        .unwrap();

                        let midi_msg = MidiMsg::from(
                          note_idx.try_into().unwrap(),
                          octave,
                          length,
                          velocity,
                          channel,
                          false,
                        );

                        midi_msg_list.push(midi_msg);
                      }

                      input_submit_note(s, &midi_msg_list);
                    }
                    Err(e) => {
                      s.add_layer(Dialog::around(TextView::new(e.to_string())).button(
                        "Close",
                        |s| {
                          s.pop_layer();
                        },
                      ));
                    }
                  }
                }
              })
              .with_name("midi_submit_config")
              .full_width(),
            )
            .child(DummyView::new().full_height()),
        ), // .child(DummyView::new().full_height()),
    )
    .with_name("midi_input")
  }

  pub fn build_tab(app: &mut Anu, regex_tx: Sender<regex::Message>) -> NamedView<TabPanel> {
    let mut tab = TabPanel::new()
      .with_tab(Self::build_midi_input())
      .with_tab(Self::build_main(app, regex_tx))
      .with_bar_alignment(Align::End)
      .with_name(consts::interactive_display_section_view);

    tab.get_mut().add_tab_at(Self::build_welcome_msg(), 0);
    tab
      .get_mut()
      .set_active_tab(consts::display_view)
      .expect("View not found");

    tab
  }

  pub fn build(
    app: &mut Anu,
    regex_tx: Sender<regex::Message>,
  ) -> ResizedView<FocusTracker<NamedView<TabPanel>>> {
    let tab = Self::build_tab(app, regex_tx);
    FocusTracker::new(tab).fixed_height(8)
  }
}
fn input_submit_note(s: &mut Cursive, midi_msg: &[MidiMsg]) {
  if let Some(data) = s.user_data::<UserData>().cloned() {
    let _ = data.midi_tx.send(midi::Message::ClearMsgConfig());
    midi_msg.iter().for_each(|msg| {
      let set_midi_conf = midi::Message::SetMsgConfig(msg.clone());
      let _ = data.midi_tx.send(set_midi_conf);
    });
  };
}

fn get_input_msg(s: &mut Cursive, name: &str) -> Arc<String> {
  s.call_on_name(name, |view: &mut EditView| view.get_content())
    .unwrap_or(Arc::new("".to_string()))
}

fn solve_regex(siv: &mut Cursive, texts: &str, regex_tx: Sender<regex::Message>) {
  let mut canvas_editor_section_view = siv
    .find_name::<Canvas<CanvasEditor>>(consts::canvas_editor_section_view)
    .unwrap();
  let text = canvas_editor_section_view.state_mut().text_contents();
  let input_regex = regex::EventData {
    text,
    pattern: texts.to_string(),
    flags: String::new(),
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
