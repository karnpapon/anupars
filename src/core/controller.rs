use std::ops::DerefMut;

use cfonts::{render, Fonts, Options};
use cursive::{
  event::{Callback, Event, EventResult},
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  view::{AnyView, Nameable, Resizable},
  views::{
    stack_view::Transparent, Canvas, Dialog, DummyView, EditView, FocusTracker, LayerPosition,
    LinearLayout, ListView, NamedView, RadioGroup, ResizedView, StackView, TextView,
  },
  Cursive, Printer, View, With,
};

use super::{canvas_base::CanvasBase, canvas_editor::CanvasEditor, config, utils};

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
    let regex_input_unit_view = EditView::new()
      .content(current_data.string.clone())
      .style(Style::highlight_inactive())
      .on_edit(input_edit)
      .on_submit(input_submit)
      .with_name(config::regex_input_unit_view)
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
        TextView::new("-").with_name(config::bpm_status_unit_view),
      )
      .child(
        "RTO: ",
        TextView::new("-").with_name(config::ratio_status_unit_view),
      )
      .child(
        "LEN: ",
        TextView::new("-").with_name(config::len_status_unit_view),
      )
      .child(
        "POS: ",
        TextView::new("-").with_name(config::pos_status_unit_view),
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

    LinearLayout::vertical()
      .child(
        FocusTracker::new(
          Dialog::around(
            LinearLayout::horizontal()
              .child(input_controller_section_view.with_name(config::input_controller_section_view))
              .child(
                status_controller_section_view.with_name(config::status_controller_section_view),
              )
              .child(
                protocol_controller_section_view
                  .with_name(config::protocol_controller_section_view),
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
        }),
      )
      .child(
        FocusTracker::new(
          Dialog::around(
            LinearLayout::vertical()
              .child(DummyView::new().fixed_width(1))
              .child(
                TextView::new(utils::build_doc_string(&config::APP_WELCOME_MSG))
                  .center()
                  .with_name(config::regex_display_unit_view)
                  .fixed_height(5),
              ),
          )
          .button("", |s| {})
          .title_position(cursive::align::HAlign::Right)
          .with_name(config::interactive_display_section_view),
        )
        .on_focus(|this| {
          this.get_mut().set_title(SpannedString::styled(
            format!(" {} ", config::interactive_display_section_view),
            Style::highlight(),
          ));
          EventResult::consumed()
        })
        .on_focus_lost(|this| {
          this.get_mut().set_title("");
          EventResult::consumed()
        }),
      )
      .child(DummyView::new().fixed_width(1))
      .child(
        FocusTracker::new(
          StackView::new()
            .layer(Transparent(
              CanvasBase::new()
                .with_name(config::canvas_base_section_view)
                .full_height()
                .full_width(),
            ))
            .layer(Transparent(
              CanvasEditor::new()
                .with_name(config::canvas_editor_section_view)
                .full_height()
                .full_width(),
            )),
        )
        .on_focus(|view| {
          view
            .get_mut(LayerPosition::FromBack(0))
            .unwrap()
            .downcast_mut::<ResizedView<ResizedView<NamedView<Canvas<CanvasBase>>>>>()
            .expect("error downcast base canvas")
            .get_inner_mut()
            .get_inner_mut()
            .get_mut()
            .set_draw(move |s, printer| {
              for (x, row) in s.grid.iter().enumerate() {
                for (y, &value) in row.iter().enumerate() {
                  printer.print((y, x), &value.to_string())
                }
              }
            });
          EventResult::consumed()
        })
        .on_focus_lost(|view| {
          view
            // .get_mut()
            .get_mut(LayerPosition::FromBack(0))
            .unwrap()
            .downcast_mut::<ResizedView<ResizedView<NamedView<Canvas<CanvasBase>>>>>()
            .expect("error downcast base canvas")
            .get_inner_mut()
            .get_inner_mut()
            .get_mut()
            .set_draw(move |s, printer| {
              for (x, row) in s.grid.iter().enumerate() {
                for (y, &value) in row.iter().enumerate() {
                  printer.print_styled(
                    (y, x),
                    &SpannedString::styled(
                      &value.to_string(),
                      Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
                    ),
                  );
                }
              }
            });
          EventResult::consumed()
        })
        .full_width()
        .full_height(),
      )
      .with_name(config::main_section_view)
  }
}

fn input_submit(siv: &mut Cursive, texts: &str) {
  let mut input_status_unit_view = siv
    .find_name::<TextView>(config::input_status_unit_view)
    .unwrap();
  input_status_unit_view.set_content(texts);
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
