use cfonts::{render, Fonts, Options};
use cursive::{
  event::{Event, EventResult},
  view::{Nameable, Resizable},
  views::{Dialog, EditView, FocusTracker, LinearLayout, ListView, RadioGroup, TextView},
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

pub struct Controller {
  // rx: mpsc::Receiver<Message>,
  // ui: Ui,
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

  let mut text_view = siv
    .find_name::<TextView>("interactive_display_view")
    .unwrap();

  text_view.get_shared_content().set_content(output.text);
}

impl Controller {
  pub fn new() -> Self {
    Controller {}
  }

  pub fn init(&mut self, current_data: &mut ControllerData) -> LinearLayout {
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
          .with_name("control_section_view"),
        )
        .on_focus(|_| {
          EventResult::with_cb(|s| {
            s.user_data::<ControllerData>().unwrap().show_regex_display = true;
          })
        })
        .on_focus_lost(|_| {
          EventResult::with_cb(|s| {
            s.user_data::<ControllerData>().unwrap().show_regex_display = false;
          })
        }),
      )
      .child(Dialog::around(
        TextView::new(utils::build_doc_string(&config::APP_WELCOME_MSG))
          .center()
          .with_name("interactive_display_view")
          .fixed_height(6),
      ))
      .child(FocusTracker::new(
        CanvasView::new(0, 0)
          .with_name("canvas_section_view")
          .full_width()
          .full_height(),
      ))
  }

  // pub fn handle_event(&mut self, event: Event) -> bool {
  //   println!("on_event controllerrr");
  //   match event {
  //     Event::Char('i') => true,
  //     Event::Char('n') => true,
  //     _ => false,
  //   }
  // }
}

impl View for Controller {
  fn draw(&self, printer: &Printer) {}

  fn on_event(&mut self, event: Event) -> EventResult {
    // self.handle_event(event);
    EventResult::Consumed(None)
  }
}
