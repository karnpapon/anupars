use crate::core::{config, utils};
use cursive::{
  theme::Style,
  view::{Nameable, Resizable},
  views::{
    DummyView, EditView, FocusTracker, LinearLayout, ListView, NamedView, PaddedView, ResizedView,
    TextView,
  },
};
use cursive_tabs::{Align, TabPanel};

pub struct MiddleSection {}

impl Default for MiddleSection {
  fn default() -> Self {
    Self::new()
  }
}

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
                    .style(Style::highlight_inactive())
                    // .on_edit(input_edit)
                    // .on_submit(input_submit)
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
      .with_tab(Self::build_osc_input())
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
