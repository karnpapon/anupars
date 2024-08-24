use cursive::{
  event::{Event, EventResult},
  views::{stack_view::Transparent, FocusTracker, StackView},
  Printer, View,
};

use super::{canvas_base::CanvasBase, canvas_editor::CanvasEditor};

pub struct CanvasSection {}

impl Default for CanvasSection {
  fn default() -> Self {
    Self::new()
  }
}

impl View for CanvasSection {
  fn draw(&self, _: &Printer) {}

  fn on_event(&mut self, _: Event) -> EventResult {
    EventResult::Consumed(None)
  }
}

impl CanvasSection {
  pub fn new() -> Self {
    CanvasSection {}
  }

  pub fn build() -> FocusTracker<StackView> {
    FocusTracker::new(
      StackView::new()
        .layer(Transparent(CanvasBase::build()))
        .layer(Transparent(CanvasEditor::build())),
    )
    // .on_focus(|view| {
    //   view
    //     .get_mut(LayerPosition::FromBack(0))
    //     .unwrap()
    //     .downcast_mut::<ResizedView<ResizedView<NamedView<Canvas<CanvasBase>>>>>()
    //     .expect("error downcast base canvas")
    //     .get_inner_mut()
    //     .get_inner_mut()
    //     .get_mut()
    //     .set_draw(move |s, printer| {
    //       for (x, row) in s.grid().iter().enumerate() {
    //         for (y, &value) in row.iter().enumerate() {
    //           printer.print((y, x), &value.to_string())
    //         }
    //       }
    //     });
    //   EventResult::consumed()
    // })
    // .on_focus_lost(|view| {
    //   view
    //     // .get_mut()
    //     .get_mut(LayerPosition::FromBack(0))
    //     .unwrap()
    //     .downcast_mut::<ResizedView<ResizedView<NamedView<Canvas<CanvasBase>>>>>()
    //     .expect("error downcast base canvas")
    //     .get_inner_mut()
    //     .get_inner_mut()
    //     .get_mut()
    //     .set_draw(move |s, printer| {
    //       for (x, row) in s.grid().iter().enumerate() {
    //         for (y, &value) in row.iter().enumerate() {
    //           printer.print_styled(
    //             (y, x),
    //             &SpannedString::styled(
    //               &value.to_string(),
    //               Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100))),
    //             ),
    //           );
    //         }
    //       }
    //     });
    //   EventResult::consumed()
    // })
    // .full_width()
    // .full_height()
  }
}
