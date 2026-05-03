use std::sync::atomic::Ordering;

use super::movement::Movement;
use super::types::Direction;
use super::types::SweepOutputMode;
use super::types::SweepRowMode;
use super::types::UIUpdate;
use crate::app::AppMode;
use crate::core::command::types::Adjustment;

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
    let drone = self.modes.drone_mode.load(Ordering::Relaxed);
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
    if drone {
      active_modes.push(AppMode::Drone);
    }
    if freeze {
      active_modes.push(AppMode::Freeze);
    }

    let base = AppMode::print_activated_modes_from_vec(&active_modes);

    let base = if sweep {
      let sweep_row = *self.sweep_row_mode.lock().unwrap();
      let sweep_output = *self.modes.sweep_output_mode.lock().unwrap();
      let cc_number = self.modes.sweep_cc_number.load(Ordering::Relaxed);
      let sweep_str = &format!("{}", AppMode::Sweep).to_uppercase();

      let row_suffix = if sweep_row != SweepRowMode::Normal {
        format!("<{}>", sweep_row.label())
      } else {
        String::new()
      };

      let output_suffix = match sweep_output {
        SweepOutputMode::Note => String::new(),
        SweepOutputMode::CC => format!("<cc{}>", cc_number),
        SweepOutputMode::Both => format!("<cc{}+>", cc_number),
      };

      if row_suffix.is_empty() && output_suffix.is_empty() {
        base
      } else {
        base.replace(
          sweep_str.as_str(),
          &format!("{}{}{}", sweep_str, row_suffix, output_suffix),
        )
      }
    } else {
      base
    };

    if drone {
      let ch = self.modes.drone_channel.load(Ordering::Relaxed);
      if ch > 1 {
        let drone_str = &format!("{}", AppMode::Drone).to_uppercase();
        return base.replace(drone_str.as_str(), &format!("{}<{}>", drone_str, ch));
      }
    }

    base
  }

  pub(super) fn build_movement_status_string(&self) -> String {
    let movement = *self.movement.lock().unwrap();
    let sweep_movement = *self.modes.sweep_movement.lock().unwrap();
    let base = movement.print_movements_with_sweep(sweep_movement);
    let easing = *self.easing_mode.lock().unwrap();
    let label = easing.label();
    if label.is_empty() {
      base
    } else {
      format!("{} : {}", base, label)
    }
  }

  pub fn cycle_easing_mode(&self) {
    let mut mode = self.easing_mode.lock().unwrap();
    *mode = mode.cycle_next();
    drop(mode);
    let _ = self.ui_tx.send(UIUpdate::MovementStatus(
      self.build_movement_status_string(),
    ));
  }

  pub fn switch_movement(&self, new_movement: Movement) {
    // If sweep movement mode is active, redirect to sweep movement instead.
    if self.modes.sweep_movement.lock().unwrap().is_some() {
      self.set_sweep_movement(new_movement);
      return;
    }

    let mut movement = self.movement.lock().unwrap();
    *movement = new_movement;
    drop(movement);

    let _ = self.ui_tx.send(UIUpdate::MovementStatus(
      self.build_movement_status_string(),
    ));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn toggle_sweep_movement_mode(&self) {
    let mut sweep_movement = self.modes.sweep_movement.lock().unwrap();
    let init_x: Option<usize> = if sweep_movement.is_some() {
      *sweep_movement = None;
      None
    } else {
      // Only activate when sweep mode is on.
      if !self.modes.sweep_mode.load(Ordering::Relaxed) {
        return;
      }
      let current = *self.movement.lock().unwrap();
      *sweep_movement = Some(current);
      let abs_x = self.pos.lock().unwrap().x + self.actived_pos.lock().unwrap().x;
      self.modes.sweep_x.store(abs_x, Ordering::Relaxed);
      Some(abs_x)
    };
    let new_val = *sweep_movement;
    drop(sweep_movement);

    let _ = self.ui_tx.send(UIUpdate::CanvasSweepMovement(new_val));
    if let Some(x) = init_x {
      let _ = self.ui_tx.send(UIUpdate::SweepX(x));
    }
    let _ = self.ui_tx.send(UIUpdate::MovementStatus(
      self.build_movement_status_string(),
    ));
  }

  pub fn set_sweep_movement(&self, new_movement: Movement) {
    let mut sweep_movement = self.modes.sweep_movement.lock().unwrap();
    *sweep_movement = Some(new_movement);
    drop(sweep_movement);

    let _ = self
      .ui_tx
      .send(UIUpdate::CanvasSweepMovement(Some(new_movement)));
    let _ = self.ui_tx.send(UIUpdate::MovementStatus(
      self.build_movement_status_string(),
    ));
  }

  pub fn toggle_arpeggiator_mode(&self) {
    let is_arp = !self.modes.arpeggiator_mode.load(Ordering::Relaxed);
    self.modes.arpeggiator_mode.store(is_arp, Ordering::Relaxed);
    self
      .position_calc
      .arpeggiator_mode
      .store(is_arp, Ordering::Relaxed);

    let _ = self.ui_tx.send(UIUpdate::CanvasArpeggiatorMode(is_arp));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn toggle_event_operator_mode(&self) {
    let is_event_op = !self.modes.event_operator_mode.load(Ordering::Relaxed);
    self
      .modes
      .event_operator_mode
      .store(is_event_op, Ordering::Relaxed);

    let _ = self
      .ui_tx
      .send(UIUpdate::CanvasEventOperatorMode(is_event_op));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn toggle_drain_queue_mode(&self) {
    let is_drain = !self.queue_manager.is_drain_queue_mode();
    self.queue_manager.set_drain_queue_mode(is_drain);

    let _ = self.ui_tx.send(UIUpdate::CanvasDrainQueueMode(is_drain));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn toggle_sweep_mode(&self) {
    let is_sweep = !self.modes.sweep_mode.load(Ordering::Relaxed);
    self.modes.sweep_mode.store(is_sweep, Ordering::Relaxed);

    let _ = self.ui_tx.send(UIUpdate::CanvasSweepMode(is_sweep));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn cycle_tilt_mode(&self) {
    let mut tilt = self.tilt_mode.lock().unwrap();
    *tilt = tilt.cycle_next();
    let new_tilt = *tilt;
    drop(tilt);

    let _ = self.ui_tx.send(UIUpdate::CanvasTiltMode(new_tilt));
    let _ = self
      .ui_tx
      .send(UIUpdate::TiltStatus(new_tilt.print_tilts()));
  }

  pub fn cycle_sweep_row_mode(&self) {
    let mut mode = self.sweep_row_mode.lock().unwrap();
    *mode = mode.cycle_next();
    let new_mode = *mode;
    drop(mode);

    let _ = self.ui_tx.send(UIUpdate::CanvasSweepRowMode(new_mode));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn cycle_sweep_output_mode(&self) {
    let mut mode = self.modes.sweep_output_mode.lock().unwrap();
    *mode = mode.cycle_next();
    let new_mode = *mode;
    drop(mode);

    let _ = self.ui_tx.send(UIUpdate::CanvasSweepOutputMode(new_mode));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn adjust_sweep_cc(&self, adj: Adjustment) {
    let current = self.modes.sweep_cc_number.load(Ordering::Relaxed);
    let next = match adj {
      Adjustment::Increase => current.saturating_add(1).min(127),
      Adjustment::Decrease => current.saturating_sub(1),
    };
    self.modes.sweep_cc_number.store(next, Ordering::Relaxed);

    let _ = self.ui_tx.send(UIUpdate::CanvasSweepCcNumber(next));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn toggle_dyn_length_mode(&self) {
    let is_dyn_length = !self.modes.dyn_length_mode.load(Ordering::Relaxed);
    self
      .modes
      .dyn_length_mode
      .store(is_dyn_length, Ordering::Relaxed);

    let _ = self
      .ui_tx
      .send(UIUpdate::CanvasDynLengthMode(is_dyn_length));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn toggle_freeze_mode(&self) {
    let is_freeze = !self.modes.freeze_mode.load(Ordering::Relaxed);
    self.modes.freeze_mode.store(is_freeze, Ordering::Relaxed);

    if is_freeze {
      let current_pos = *self.actived_pos.lock().unwrap();
      *self.frozen_active_pos.lock().unwrap() = current_pos;
    }

    let _ = self.ui_tx.send(UIUpdate::CanvasFreezeMode(is_freeze));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn cycle_drone_channel(&self, dir: Adjustment) {
    if !self.modes.drone_mode.load(Ordering::Relaxed) {
      return;
    }

    let current = self.modes.drone_channel.load(Ordering::Relaxed);
    let new_ch = match dir {
      Adjustment::Increase => {
        if current >= 16 {
          1
        } else {
          current + 1
        }
      }
      Adjustment::Decrease => {
        if current <= 1 {
          16
        } else {
          current - 1
        }
      }
    };
    self.modes.drone_channel.store(new_ch, Ordering::Relaxed);

    // Retrigger so the new channel takes effect immediately.
    let area = *self.area.lock().unwrap();
    let drone_x = self.modes.drone_x.load(Ordering::Relaxed);
    self.midi_handler.trigger_drone_at_x(drone_x, &area);

    let _ = self.ui_tx.send(UIUpdate::CanvasDroneChannel(new_ch));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  pub fn toggle_drone_mode(&self) {
    let is_drone = !self.modes.drone_mode.load(Ordering::Relaxed);
    self.modes.drone_mode.store(is_drone, Ordering::Relaxed);

    let drone_x = if is_drone {
      let x = 0; // relative offset from area.left()
      self.modes.drone_x.store(x, Ordering::Relaxed);
      self.modes.drone_channel.store(1, Ordering::Relaxed);
      x
    } else {
      self.midi_handler.release_drone_notes_with_tail();
      self.modes.drone_x.load(Ordering::Relaxed)
    };

    let _ = self
      .ui_tx
      .send(UIUpdate::CanvasDroneMode(is_drone, drone_x));
    let _ = self.ui_tx.send(UIUpdate::CanvasDroneChannel(1));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }

  /// Move the drone line left or right within the playhead area bounds.
  /// Releases held notes with a tail and triggers new notes at the new position.
  pub fn move_drone(&self, dir: Direction) {
    if !self.modes.drone_mode.load(Ordering::Relaxed) {
      return;
    }

    let area = {
      let a = self.area.lock().unwrap();
      *a
    };

    let current = self.modes.drone_x.load(Ordering::Relaxed);
    let new_x = match dir {
      Direction::Left if current > 0 => current - 1,
      Direction::Right if current < area.width().saturating_sub(1) => current + 1,
      _ => return,
    };

    self.modes.drone_x.store(new_x, Ordering::Relaxed);
    self.midi_handler.trigger_drone_at_x(new_x, &area);

    let _ = self.ui_tx.send(UIUpdate::CanvasDroneX(new_x));
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

    let _ = self
      .ui_tx
      .send(UIUpdate::CanvasAccumulationMode(is_enabled));
    let _ = self.ui_tx.send(UIUpdate::InputStatus("-".to_string()));
    let _ = self
      .ui_tx
      .send(UIUpdate::ModeStatus(self.build_mode_status_string()));
  }
}

#[cfg(test)]
mod tests {
  use super::super::movement::Movement;
  use super::super::test_helpers::make_playhead;
  use crate::core::geom::Vec2;
  use crate::view::rect::Rect;
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
