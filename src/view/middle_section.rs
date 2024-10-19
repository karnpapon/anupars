use std::sync::{mpsc::Sender, Arc};

use crate::core::{
  application::UserData,
  config,
  midi::{self, Midi},
  utils,
};
use cursive::{
  theme::Style,
  view::{Nameable, Resizable},
  views::{
    Button, Dialog, DummyView, EditView, FocusTracker, LinearLayout, ListView, NamedView,
    PaddedView, ResizedView, TextView,
  },
  Cursive,
};
use cursive_tabs::{Align, TabPanel};

use nom::{
  branch::alt,
  bytes::complete::tag,
  character::complete::{char, digit1, one_of},
  combinator::{map_res, opt},
  sequence::{pair, tuple},
  IResult,
};

pub struct MiddleSection {}

// impl Default for MiddleSection {
//   fn default() -> Self {
//     Self::new()
//   }
// }

// impl View for MiddleSection {
//   fn draw(&self, _: &Printer) {}

//   fn on_event(&mut self, _: Event) -> EventResult {
//     EventResult::Consumed(None)
//   }
// }

impl MiddleSection {
  pub fn new() -> Self {
    MiddleSection {}
  }

  fn build_welcome_msg() -> NamedView<TextView> {
    TextView::new(utils::build_doc_string(&config::APP_WELCOME_MSG))
      .center()
      .with_name(config::regex_display_unit_view)
  }

  fn build_osc_input() -> NamedView<PaddedView<LinearLayout>> {
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
                  "   PATH:",
                  EditView::new()
                    .content("")
                    .style(Style::highlight_inactive())
                    // .on_edit(input_edit)
                    // .on_submit(input_submit)
                    .with_name("osc_path"),
                )
                .full_width(),
            )
            .child(
              ListView::new()
                .child(
                  "   MSG:",
                  EditView::new()
                    .content("")
                    .style(Style::highlight_inactive())
                    .with_name("osc_msg"),
                )
                .full_width(),
            ),
        )
        .child(DummyView::new().full_height()),
    )
    .with_name("osc_input")
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
                  "   NOTE:",
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
                  "   LENGTH:",
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
                  "   VELOCITY:",
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
                  "   CHANNEL:",
                  EditView::new()
                    .content("")
                    .style(Style::highlight_inactive())
                    .with_name("midi_chan"),
                )
                .full_width(),
            )
            .child(
              Button::new_raw("[ SET ]", |s| {
                let midi_note = get_input_msg(s, "midi_note");
                let midi_len = get_input_msg(s, "midi_len");
                let midi_vel = get_input_msg(s, "midi_vel");
                let midi_chan = get_input_msg(s, "midi_chan");

                if [&midi_note, &midi_len, &midi_vel, &midi_chan]
                  .iter()
                  .any(|s| s.is_empty())
                {
                  println!("some msg is missing");
                } else {
                  match parse_note_octave(&midi_note) {
                    Ok((remaining, (note, octave))) => {
                      println!("Note: {}", note);
                      println!("Octave: {}", octave);
                      println!("remaining: {}", remaining);
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
        )
        .child(DummyView::new().full_height()),
    )
    .with_name("midi_input")
  }

  pub fn build_tab() -> NamedView<TabPanel> {
    let mut tab = TabPanel::new()
      .with_tab(Self::build_welcome_msg())
      .with_tab(Self::build_midi_input())
      // .with_tab(Self::build_osc_input())
      .with_bar_alignment(Align::End)
      .with_name(config::interactive_display_section_view);

    tab
      .get_mut()
      .set_active_tab(config::regex_display_unit_view)
      .expect("View not found");

    tab
  }

  pub fn build() -> ResizedView<FocusTracker<NamedView<TabPanel>>> {
    let tab = Self::build_tab();
    FocusTracker::new(tab).fixed_height(8)
  }
}
// fn input_submit_note(s: &mut Cursive, notes: &str) {
//   if let Some(data) = s.user_data::<UserData>().cloned() {
//     let midi_msg = midi::MidiMsg::from(notes.to_string(), 4, 4, 8, 8, false);
//     // data.midi_tx.send(midi::Message::Push(midi_msg)).unwrap();
//   };
// }

fn get_input_msg(s: &mut Cursive, name: &str) -> Arc<String> {
  s.call_on_name(name, |view: &mut EditView| view.get_content())
    .unwrap_or(Arc::new("".to_string()))
}

fn parse_note_octave(input: &str) -> IResult<&str, (String, i32)> {
  let (input, note) = one_of("CDEFGAB")(input)?;
  let (input, sharp) = opt(tag("#"))(input)?;
  let (input, octave) = map_res(digit1, |s: &str| s.parse::<i32>())(input)?;

  let note_with_sharp = format!("{}{}", note, sharp.unwrap_or(""));

  // Ensure no extra characters are left
  if !input.is_empty() {
    return Err(nom::Err::Error(nom::error::Error {
      input,
      code: nom::error::ErrorKind::Eof,
    }));
  }

  Ok((input, (note_with_sharp, octave)))
}
