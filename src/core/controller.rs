use cursive::{
  align::{HAlign, VAlign},
  event::{Callback, Event, EventResult, Key},
  theme::{Color, ColorStyle},
  view::{Nameable, Resizable, Selector},
  views::{
    Dialog, EditView, LinearLayout, ListView, OnEventView, RadioGroup, SliderView, TextView,
  },
  Cursive, CursiveExt, Printer, View, With,
};

use super::{canvas::CanvasView, config, utils};

#[derive(Clone, Debug, Default)]
pub struct ControllerData {
  pub boolean: bool,
  pub string: String,
  pub number: usize,
}

pub struct Controller {
  // rx: mpsc::Receiver<Message>,
  // ui: Ui,
}

fn show_popup(s: &mut Cursive, name: &str) {
  if name.is_empty() {
    s.add_layer(Dialog::info("Please enter a name!"));
  } else {
    let content = format!("Hello {}!", name);
    s.pop_layer();
    s.add_layer(Dialog::around(TextView::new(content)).button("Quit", |s| s.quit()));
  }
}

impl Controller {
  pub fn new() -> Self {
    Controller {}
  }

  pub fn init(&mut self, siv: &mut Cursive) {
    let mut boolean_group: RadioGroup<bool> = RadioGroup::new();
    let current_data = siv
      .with_user_data(|controller_data: &mut ControllerData| controller_data.clone())
      .unwrap();

    let mut regex_view = EditView::new()
      .content(current_data.string.clone())
      .on_submit(show_popup)
      .disabled();

    regex_view.set_style(Color::TerminalDefault);

    let input_controller = ListView::new()
      .child(
        "RegExp: ",
        regex_view.with_name("ctr_regex").fixed_width(25),
      )
      .child(
        "Mode: ",
        LinearLayout::horizontal()
          .child(boolean_group.button(false, "Realtime"))
          .child(
            boolean_group
              .button(true, "True")
              .with_if(current_data.boolean, |button| {
                button.select();
              }),
          )
          .with(|layout| {
            if current_data.boolean {
              layout.set_focus_index(1).unwrap();
            }
          }),
      )
      .child("flag: ", TextView::new("-").with_name("ctr_flag"))
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

    siv.add_layer(
      LinearLayout::vertical()
        .child(Dialog::around(
          LinearLayout::horizontal()
            .child(input_controller.with_name("input_controller"))
            .child(status_view.with_name("status_view"))
            .child(protocol_view.with_name("protocol_view")),
        ))
        .child(Dialog::around(
          TextView::new(utils::build_doc_string(&config::APP_WELCOME_MSG))
            .h_align(HAlign::Center)
            .v_align(VAlign::Center),
        ))
        .child(
          CanvasView::new(0, 0)
            .with_name("canvas_view")
            .full_width()
            .full_height(),
        ),
    );
  }

  pub fn handle_event(&mut self, event: Event) -> bool {
    println!("on_event controllerrr");
    match event {
      Event::Char('i') => true,
      _ => false,
    }
  }
}

impl View for Controller {
  fn draw(&self, printer: &Printer) {}

  fn on_event(&mut self, event: Event) -> EventResult {
    self.handle_event(event);
    EventResult::Consumed(None)
  }
}
