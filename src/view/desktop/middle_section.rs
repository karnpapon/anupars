use crate::core::{
  application::UserData,
  consts,
  midi::{self, MidiMsg},
  parser::{self},
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
use std::sync::Arc;

pub struct MiddleSection {}

impl MiddleSection {
  pub fn new() -> Self {
    MiddleSection {}
  }

  fn build_welcome_msg() -> NamedView<TextView> {
    TextView::new(utils::build_doc_string(&consts::APP_WELCOME_MSG))
      .center()
      .with_name(consts::display_view)
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
                  "   NTE:",
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
                  "   LEN:",
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
                  "   VEL:",
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
                  "   CHN:",
                  EditView::new()
                    .content("")
                    .style(Style::highlight_inactive())
                    .with_name("midi_chan"),
                )
                .full_width(),
            )
            .child(
              Button::new_raw("[ SET ]", |s| {
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
      .with_name(consts::interactive_display_section_view);

    tab
      .get_mut()
      .set_active_tab(consts::display_view)
      .expect("View not found");

    tab
  }

  pub fn build() -> ResizedView<FocusTracker<NamedView<TabPanel>>> {
    let tab = Self::build_tab();
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
