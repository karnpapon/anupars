use cursive::{
  event::{Event, EventResult},
  theme::Style,
  utils::span::SpannedString,
  view::{Nameable, Resizable},
  views::{Dialog, DummyView, FocusTracker, LinearLayout, NamedView, TextView},
  Printer, View,
};

use crate::core::{config, utils};

pub struct MiddleSection {}

impl Default for MiddleSection {
  fn default() -> Self {
    Self::new()
  }
}

impl View for MiddleSection {
  fn draw(&self, _: &Printer) {}

  fn on_event(&mut self, _: Event) -> EventResult {
    EventResult::Consumed(None)
  }
}

impl MiddleSection {
  pub fn new() -> Self {
    MiddleSection {}
  }

  pub fn build() -> FocusTracker<NamedView<Dialog>> {
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
    })
  }
}
