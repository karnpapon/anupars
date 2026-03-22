use std::sync::atomic::Ordering;

use cursive::views::Canvas;
use cursive::views::TextView;

use super::movement::Movement;
use super::types::SweepRowMode;
use crate::app::AppMode;
use crate::core::consts;
use crate::view::grid::GridEditor;

use super::Playhead;

impl Playhead {
  /// Returns true when the playhead is currently advancing in the forward direction.
  pub(super) fn is_going_forward(&self) -> bool {
    let movement = *self.movement.lock().unwrap();
    match movement {
      Movement::Forward | Movement::Random => true,
      Movement::Reverse => false,
      Movement::Pendulum => {
        let step_idx = *self.step_index.lock().unwrap();
        let area = self.area.lock().unwrap();
        let total = area.width() * area.height();
        drop(area);
        if total <= 1 {
          return true;
        }
        let cycle_len = total * 2 - 2;
        (step_idx % cycle_len) < total
      }
    }
  }

  pub(super) fn build_mode_status_string(&self) -> String {
    let arpeggiator = self.modes.arpeggiator_mode.load(Ordering::Relaxed);
    let accumulation = self.modes.accumulation_mode.load(Ordering::Relaxed);
    let event_op = self.modes.event_operator_mode.load(Ordering::Relaxed);
    let drain_queue = self.queue_manager.is_drain_queue_mode();
    let sweep = self.modes.sweep_mode.load(Ordering::Relaxed);
    let dyn_length = self.modes.dyn_length_mode.load(Ordering::Relaxed);
    let freeze = self.modes.freeze_mode.load(Ordering::Relaxed);

    let mut active_modes = Vec::new();
    if arpeggiator {
      active_modes.push(AppMode::Arpeggiator);
    }
    if drain_queue {
      active_modes.push(AppMode::DrainQueue);
    }
    if accumulation {
      active_modes.push(AppMode::Accumulation);
    }
    if dyn_length {
      active_modes.push(AppMode::DynLength);
    }
    if event_op {
      active_modes.push(AppMode::EventOperator);
    }
    if sweep {
      active_modes.push(AppMode::Sweep);
    }
    if freeze {
      active_modes.push(AppMode::Freeze);
    }

    let base = AppMode::print_activated_modes_from_vec(&active_modes);

    if sweep {
      let sweep_row = *self.sweep_row_mode.lock().unwrap();
      if sweep_row == SweepRowMode::Normal {
        base
      } else {
        let sweep = &format!("{}", AppMode::Sweep).to_uppercase();
        base.replace(sweep.as_str(), &format!("{}<{}>", sweep, sweep_row.label()))
      }
    } else {
      base
    }
  }

  pub(super) fn build_movement_status_string(&self) -> String {
    let movement = self.movement.lock().unwrap();
    movement.print_movements()
  }

  pub fn switch_movement(&self, new_movement: Movement) {
    let mut movement = self.movement.lock().unwrap();
    *movement = new_movement;
    drop(movement);

    let movement_status = self.build_movement_status_string();
    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
        siv.call_on_name(consts::movement_unit_view, |view: &mut TextView| {
          view.set_content(movement_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_arpeggiator_mode(&self) {
    let is_arp = !self.modes.arpeggiator_mode.load(Ordering::Relaxed);
    self.modes.arpeggiator_mode.store(is_arp, Ordering::Relaxed);
    self
      .position_calc
      .arpeggiator_mode
      .store(is_arp, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.arpeggiator_mode = is_arp;
            editor.playhead_ui.arpeggiator_mode = is_arp;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_event_operator_mode(&self) {
    let is_event_op = !self.modes.event_operator_mode.load(Ordering::Relaxed);
    self
      .modes
      .event_operator_mode
      .store(is_event_op, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.event_operator_mode = is_event_op;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_drain_queue_mode(&self) {
    let is_drain = !self.queue_manager.is_drain_queue_mode();
    self.queue_manager.set_drain_queue_mode(is_drain);

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.drain_queue_mode = is_drain;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_sweep_mode(&self) {
    let is_sweep = !self.modes.sweep_mode.load(Ordering::Relaxed);
    self.modes.sweep_mode.store(is_sweep, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.sweep_mode = is_sweep;
            editor.playhead_ui.sweep_mode = is_sweep;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn cycle_tilt_mode(&self) {
    let mut tilt = self.tilt_mode.lock().unwrap();
    *tilt = tilt.cycle_next();
    let new_tilt = *tilt;
    drop(tilt);

    let tilt_status = new_tilt.print_tilts();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.playhead_ui.tilt_mode = new_tilt;
          },
        );
        siv.call_on_name(consts::tilt_unit_view, |view: &mut TextView| {
          view.set_content(tilt_status);
        });
      }))
      .unwrap();
  }

  pub fn cycle_sweep_row_mode(&self) {
    let mut mode = self.sweep_row_mode.lock().unwrap();
    *mode = mode.cycle_next();
    let new_mode = *mode;
    drop(mode);

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.sweep_row_mode = new_mode;
            editor.playhead_ui.sweep_row_mode = new_mode;
          },
        );
        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_dyn_length_mode(&self) {
    let is_dyn_length = !self.modes.dyn_length_mode.load(Ordering::Relaxed);
    self
      .modes
      .dyn_length_mode
      .store(is_dyn_length, Ordering::Relaxed);

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub fn toggle_freeze_mode(&self) {
    let is_freeze = !self.modes.freeze_mode.load(Ordering::Relaxed);
    self.modes.freeze_mode.store(is_freeze, Ordering::Relaxed);

    if is_freeze {
      let current_pos = *self.actived_pos.lock().unwrap();
      *self.frozen_active_pos.lock().unwrap() = current_pos;
    }

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.freeze_mode = is_freeze;
          },
        );

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }

  pub(super) fn handle_toggle_accumulation_mode(&self) {
    let is_enabled = !self.modes.accumulation_mode.load(Ordering::Relaxed);
    self
      .modes
      .accumulation_mode
      .store(is_enabled, Ordering::Relaxed);
    self.reset_accumulation_counter();

    if !is_enabled {
      self.queue_manager.clear_all();
    }

    let mode_status = self.build_mode_status_string();
    let cb_sink = self.cb_sink.clone();

    cb_sink
      .send(Box::new(move |siv| {
        siv.call_on_name(
          consts::canvas_editor_section_view,
          |canvas: &mut Canvas<GridEditor>| {
            let editor = canvas.state_mut();
            editor.accumulation_mode = is_enabled;
            editor.playhead_ui.accumulation_mode = is_enabled;
          },
        );

        siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
          view.set_content("-");
        });

        siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
          view.set_content(mode_status);
        });
      }))
      .unwrap();
  }
}

#[cfg(test)]
mod tests {
  use super::super::movement::Movement;
  use super::super::test_helpers::make_playhead;
  use crate::view::rect::Rect;
  use cursive::Vec2;
  use std::sync::atomic::Ordering;

  #[test]
  fn going_forward_for_forward_movement() {
    let ph = make_playhead();
    *ph.movement.lock().unwrap() = Movement::Forward;
    assert!(ph.is_going_forward());
  }

  #[test]
  fn not_going_forward_for_reverse_movement() {
    let ph = make_playhead();
    *ph.movement.lock().unwrap() = Movement::Reverse;
    assert!(!ph.is_going_forward());
  }

  #[test]
  fn pendulum_first_half_is_forward_second_half_is_backward() {
    let ph = make_playhead();
    *ph.movement.lock().unwrap() = Movement::Pendulum;
    *ph.area.lock().unwrap() = Rect::from_size(Vec2::zero(), (4, 1));

    *ph.step_index.lock().unwrap() = 3;
    assert!(ph.is_going_forward());

    *ph.step_index.lock().unwrap() = 4;
    assert!(!ph.is_going_forward());
  }

  #[test]
  fn mode_string_all_off_is_lowercase() {
    let ph = make_playhead();
    let s = ph.build_mode_status_string();
    assert!(s.chars().all(|c| !c.is_uppercase()), "got: {s}");
  }

  #[test]
  fn mode_string_arpeggiator_on_shows_uppercase_a() {
    let ph = make_playhead();
    ph.modes.arpeggiator_mode.store(true, Ordering::Relaxed);
    let s = ph.build_mode_status_string();
    assert!(s.contains('A'), "expected uppercase A in: {s}");
  }

  #[test]
  fn mode_string_multiple_modes_reflected() {
    let ph = make_playhead();
    ph.modes.accumulation_mode.store(true, Ordering::Relaxed);
    ph.modes.sweep_mode.store(true, Ordering::Relaxed);
    let s = ph.build_mode_status_string();
    assert!(s.contains('U'), "accumulation (U) missing in: {s}");
    assert!(s.contains('S'), "sweep (S) missing in: {s}");
  }
}
