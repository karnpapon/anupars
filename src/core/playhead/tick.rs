use std::collections::HashMap;
use std::sync::atomic::Ordering;

use cursive::Vec2;

use crate::core::engine::regex;
use crate::view::rect::Rect;

use super::{Playhead, UIUpdate};

impl Playhead {
  pub fn set_actived_pos(&self, pos: usize) {
    let area = self.area.lock().unwrap();
    let playhead_w = area.width();
    let playhead_h = area.height();
    let playhead_x = area.left();
    let playhead_y = area.top();
    drop(area);

    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);
    self
      .position_calc
      .grid_height
      .store(self.grid.height.load(Ordering::Relaxed), Ordering::Relaxed);
    self.position_calc.arpeggiator_mode.store(
      self.modes.arpeggiator_mode.load(Ordering::Relaxed),
      Ordering::Relaxed,
    );

    let new_pos = self
      .position_calc
      .calculate_actived_pos(pos, playhead_x, playhead_y, playhead_w, playhead_h);

    let mut actived_pos = self.actived_pos.lock().unwrap();
    *actived_pos = new_pos;
  }

  /// Get absolute position the same way it's displayed in the console "POS: (x,y)" and flattened index.
  pub(super) fn calculate_absolute_position(&self, active_pos: Vec2) -> (usize, usize, usize) {
    let pos = self.pos.lock().unwrap();
    let playhead_pos = *pos;
    drop(pos);

    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);
    self
      .position_calc
      .calculate_absolute_position(playhead_pos, active_pos)
  }

  // TODO: revisit keyboard-left
  /// Find distance to the next closest trigger position within the playhead area.
  pub(super) fn find_distance_to_next_trigger(&self, curr_pos: usize) -> usize {
    let area = self.area.lock().unwrap();
    let playhead_x = area.left();
    let playhead_y = area.top();
    let playhead_width = area.width();
    let playhead_height = area.height();
    drop(area);

    self
      .position_calc
      .grid_width
      .store(self.grid.width.load(Ordering::Relaxed), Ordering::Relaxed);

    self.position_calc.find_distance_to_next_trigger(
      curr_pos,
      playhead_x,
      playhead_y,
      playhead_width,
      playhead_height,
    )
  }

  pub(super) fn check_contains(&self, area: &Rect, matcher: &HashMap<usize, regex::Match>) -> bool {
    for dy in 0..area.height() {
      for dx in 0..area.width() {
        let x = area.top_left.x + dx;
        let y = area.top_left.y + dy;
        let pos_index = y * self.grid.width.load(Ordering::Relaxed) + x;
        if matcher.contains_key(&pos_index) {
          return true;
        }
      }
    }
    false
  }

  pub(super) fn update_active_pos_ui(&self, active_pos: Vec2) {
    let mut queue = self.ui_update_queue.lock().unwrap();
    queue.push_back(UIUpdate::ActivePos(active_pos));
  }

  pub(super) fn handle_set_active_pos(&self, tick: usize) {
    let ratio = self.music.ratio.lock().unwrap();
    // Clock fires 16 ticks/beat (64 per 4-beat bar). Divider table:
    //   ratio.1=1  → every 64 ticks (DIV 1)    ratio.1=16 → every  4 ticks (DIV 16)
    //   ratio.1=32 → every  2 ticks (DIV 32)   ratio.1=64 → every  1 tick  (DIV 64)
    let base_divider = (64 / ratio.1).max(1);
    drop(ratio);

    if self.modes.freeze_mode.load(Ordering::Relaxed) {
      if tick.is_multiple_of(base_divider) && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        let active_pos = *self.frozen_active_pos.lock().unwrap();
        let going_forward = self.is_going_forward();
        let (abs_x, abs_y, curr_running_playhead) = self.calculate_absolute_position(active_pos);
        let (note_position, scale_mode) = self
          .midi_handler
          .determine_note_position_and_scale(active_pos, abs_x, abs_y);

        let matcher_guard = self.text_matcher.lock().unwrap();
        let match_at_pos = if going_forward {
          matcher_guard
            .as_ref()
            .and_then(|m| m.get(&curr_running_playhead))
            .cloned()
        } else {
          matcher_guard.as_ref().and_then(|m| {
            m.values()
              .find(|mv| curr_running_playhead == mv.i + mv.l.max(1) - 1)
              .cloned()
          })
        };
        drop(matcher_guard);
        let has_match = match_at_pos.is_some();
        let match_len = match_at_pos.as_ref().map(|m| m.l).unwrap_or(1).max(1);

        if has_match {
          self.midi_handler.trigger_midi_if_matched(
            curr_running_playhead,
            note_position,
            scale_mode,
            abs_x,
            abs_y,
            match_len,
          );
        }

        self.update_active_pos_ui(active_pos);
      }
      return;
    }

    let going_forward = self.is_going_forward();
    // Inside a match span: speed up when going forward (halve divider),
    // slow down when going reverse (double divider).
    let in_match_span = self.match_span_remaining.load(Ordering::Relaxed) > 0;
    let divider = if in_match_span {
      if going_forward {
        (base_divider / 2).max(1)
      } else {
        base_divider * 2
      }
    } else {
      base_divider
    };

    let should_advance =
      tick.is_multiple_of(divider) && !self.modes.is_ratcheting.load(Ordering::Relaxed);
    if should_advance {
      if in_match_span {
        self.match_span_remaining.fetch_sub(1, Ordering::Relaxed);
      }

      let mut step_idx = self.step_index.lock().unwrap();
      *step_idx += 1;
      self.set_actived_pos(*step_idx);
      drop(step_idx);

      let active_pos_mutex = self.actived_pos.lock().unwrap();
      let mut active_pos = *active_pos_mutex;
      drop(active_pos_mutex);

      let (abs_x, abs_y, curr_running_playhead) = self.calculate_absolute_position(active_pos);

      let (note_position, scale_mode) = self
        .midi_handler
        .determine_note_position_and_scale(active_pos, abs_x, abs_y);

      let mut did_jump = false;

      // Capture the full Match for the current cell.
      // Forward: trigger at the first cell (m.i == curr).
      // Reverse/Pendulum-reverse: trigger at the last cell (m.i + m.l - 1 == curr).
      let matcher_guard = self.text_matcher.lock().unwrap();
      let match_at_pos = if going_forward {
        matcher_guard
          .as_ref()
          .and_then(|m| m.get(&curr_running_playhead))
          .cloned()
      } else {
        matcher_guard.as_ref().and_then(|m| {
          m.values()
            .find(|mv| curr_running_playhead == mv.i + mv.l.max(1) - 1)
            .cloned()
        })
      };
      drop(matcher_guard);

      let has_match = match_at_pos.is_some();
      let match_len = match_at_pos.as_ref().map(|m| m.l).unwrap_or(1).max(1);

      // Retrigger on every fast step inside a span (cells 2, 3, … of a match word)
      if in_match_span && !has_match && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        self.midi_handler.trigger_midi_if_matched(
          curr_running_playhead,
          note_position,
          scale_mode,
          abs_x,
          abs_y,
          1,
        );
        self.enqueue_sym_buf_append(curr_running_playhead);
      }

      if has_match {
        if self.modes.accumulation_mode.load(Ordering::Relaxed) {
          if let Some(new_active_pos) = self.handle_accumulation_mode(abs_x) {
            active_pos = new_active_pos;
            did_jump = true;
          }
        }
        if !did_jump && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
          let distance_to_next = if self.modes.dyn_length_mode.load(Ordering::Relaxed)
            && !self.modes.arpeggiator_mode.load(Ordering::Relaxed)
          {
            self.find_distance_to_next_trigger(curr_running_playhead)
          } else {
            match_len
          };

          self.midi_handler.trigger_midi_if_matched(
            curr_running_playhead,
            note_position,
            scale_mode,
            abs_x,
            abs_y,
            distance_to_next,
          );
          self.enqueue_sym_buf_append(curr_running_playhead);
        }
        if self.modes.hold_next_note.load(Ordering::Relaxed) {
          self.modes.hold_next_note.store(false, Ordering::Relaxed);
        }
        // Start fast-stepping for remaining cells in the match span
        if match_len > 1 {
          self
            .match_span_remaining
            .store(match_len - 1, Ordering::Relaxed);
        }
      }

      let matcher_guard = self.text_matcher.lock().unwrap();
      if let Some(ref m) = *matcher_guard {
        self.handle_silent_step(m);
      }
      drop(matcher_guard);

      if !did_jump && !self.modes.is_ratcheting.load(Ordering::Relaxed) {
        let area_right = self.area.lock().unwrap().right();
        let active_pos_y = self.pos.lock().unwrap().y;
        self.midi_handler.trigger_midi_if_matched_sweep(
          curr_running_playhead,
          abs_x,
          active_pos_y,
          area_right,
        );

        for idx in self
          .midi_handler
          .sweep_matched_indexes(curr_running_playhead, abs_x, area_right)
        {
          self.enqueue_sym_buf_append(idx);
        }
      }
      self.update_active_pos_ui(active_pos);
    }
  }
}
