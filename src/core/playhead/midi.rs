use cursive::Vec2;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::core::playhead::tilt::TiltMode;
use crate::core::playhead::types::GridState;
use crate::core::playhead::types::ModeFlags;
use crate::core::playhead::types::MusicState;
use crate::core::playhead::types::SweepRowMode;
use crate::core::{consts, engine::regex::Match, io::midi, tonal::scale};
use crate::view::rect::Rect;

/// Calculate MIDI channel based on grid splits and position
pub fn calculate_channel(
  abs_x: usize,
  abs_y: usize,
  grid_width: usize,
  grid_height: usize,
  grid_v_splits: usize,
  grid_h_splits: usize,
) -> u8 {
  let v = grid_v_splits.max(1);
  let h = grid_h_splits.max(1);
  let gw = grid_width.max(1);
  let gh = grid_height.max(1);
  let col_w = if v > 1 { (gw / v).max(1) } else { gw };
  let row_h = if h > 1 { (gh / h).max(1) } else { gh };
  let col_idx = (abs_x / col_w).min(v.saturating_sub(1));
  let row_idx = (abs_y / row_h).min(h.saturating_sub(1));
  (row_idx * v + col_idx) as u8
}

/// Calculate velocity based on Y position (higher = softer)
pub fn calculate_velocity(abs_y: usize, grid_height: usize, max_vel: f32, min_vel: f32) -> u8 {
  if grid_height == 0 {
    return 64; // default mid velocity
  }
  (max_vel - (abs_y as f32 / grid_height as f32) * (max_vel - min_vel))
    .round()
    .max(min_vel) as u8
}

pub struct MidiHandlerConfig {
  pub midi_tx: Sender<midi::Message>,
  pub prev_active_pos: Arc<Mutex<Vec2>>,
  pub ratchet_generation: Arc<AtomicUsize>,
  pub text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,
  pub tilt_mode: Arc<Mutex<TiltMode>>,
  pub playhead_pos: Arc<Mutex<Vec2>>,
  pub sweep_row_mode: Arc<Mutex<SweepRowMode>>,
}

pub struct MidiTriggerHandler {
  pub midi_tx: Sender<midi::Message>,
  pub grid: GridState,
  pub music: MusicState,
  pub modes: ModeFlags,
  pub prev_active_pos: Arc<Mutex<Vec2>>,
  pub ratchet_generation: Arc<AtomicUsize>,
  pub text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,
  pub tilt_mode: Arc<Mutex<TiltMode>>,
  pub playhead_pos: Arc<Mutex<Vec2>>,
  pub sweep_row_mode: Arc<Mutex<SweepRowMode>>,
  pub drone_held_notes: Arc<Mutex<Vec<midi::MidiMsg>>>,
  /// Incremented on each drone move so stale note-off threads self-cancel.
  pub drone_release_generation: Arc<AtomicUsize>,
}

impl MidiTriggerHandler {
  pub fn new(
    config: MidiHandlerConfig,
    grid: &GridState,
    music: &MusicState,
    modes: &ModeFlags,
  ) -> Self {
    MidiTriggerHandler {
      midi_tx: config.midi_tx,
      grid: grid.clone(),
      music: music.clone(),
      modes: modes.clone(),
      prev_active_pos: config.prev_active_pos,
      ratchet_generation: config.ratchet_generation,
      text_matcher: config.text_matcher,
      tilt_mode: config.tilt_mode,
      playhead_pos: config.playhead_pos,
      sweep_row_mode: config.sweep_row_mode,
      drone_held_notes: Arc::new(Mutex::new(Vec::new())),
      drone_release_generation: Arc::new(AtomicUsize::new(0)),
    }
  }

  /// Determine note position and scale mode based on movement
  pub fn determine_note_position_and_scale(
    &self,
    active_pos: Vec2,
    abs_x: usize,
    _abs_y: usize,
  ) -> (usize, scale::ScaleMode) {
    let mut prev_active = self.prev_active_pos.lock().unwrap();
    *prev_active = active_pos;
    drop(prev_active);

    let grid_height = self.grid.height.load(Ordering::Relaxed);
    // Always use the top keyboard for note sending (x-axis layout).
    // The left keyboard is reserved for drone mode.
    let scale = *self.music.scale_mode_top.lock().unwrap();
    let pos = if grid_height > 0 {
      abs_x % grid_height
    } else {
      abs_x
    };
    (pos, scale)
  }

  /// Trigger MIDI for a matched position
  pub fn trigger_midi_if_matched(
    &self,
    _curr_running_playhead: usize,
    note_position: usize,
    scale_mode: scale::ScaleMode,
    abs_x: usize,
    abs_y: usize,
    distance_to_next: usize,
  ) {
    let grid_height = self.grid.height.load(Ordering::Relaxed);
    let grid_width = self.grid.width.load(Ordering::Relaxed);
    let current_tempo = self.music.tempo.load(Ordering::Relaxed);
    let hold_next = self.modes.hold_next_note.load(Ordering::Relaxed);
    let div = self.music.ratio.lock().unwrap().1;

    let _ = self
      .midi_tx
      .send(midi::Message::TriggerWithPosition(midi::TriggerParams {
        y_position: note_position,
        grid_height,
        x_position: abs_x,
        grid_width,
        grid_v_splits: self.grid.v_splits.load(Ordering::Relaxed),
        grid_h_splits: self.grid.h_splits.load(Ordering::Relaxed),
        scale_mode,
        scale_root_offset: self.music.scale_root_top.lock().unwrap().to_root_offset(),
        bpm: current_tempo,
        trigger_pos_y: abs_y,
        active_pos_y: abs_y,
        distance_to_next,
        hold: hold_next,
        is_sweep: false,
        div,
      }));
  }

  /// Return the flat grid indexes that sweep would trigger (same geometry as trigger_midi_if_matched_sweep).
  pub fn sweep_matched_indexes(
    &self,
    curr_running_playhead: usize,
    abs_x: usize,
    area_right: usize,
    exclude: Option<(usize, u8)>,
  ) -> Vec<usize> {
    if !self.modes.sweep_mode.load(Ordering::Relaxed) {
      return Vec::new();
    }
    let grid_width = self.grid.width.load(Ordering::Relaxed);
    let grid_height = self.grid.height.load(Ordering::Relaxed);
    let tilt_mode = *self.tilt_mode.lock().unwrap();
    let playhead_pos = *self.playhead_pos.lock().unwrap();
    let v = self.grid.v_splits.load(Ordering::Relaxed).max(1);
    let h = self.grid.h_splits.load(Ordering::Relaxed).max(1);
    let col_w = (grid_width / v).max(1);
    let row_h = (grid_height / h).max(1);
    let playhead_col = (playhead_pos.x / col_w).min(v.saturating_sub(1));
    let playhead_row = (playhead_pos.y / row_h).min(h.saturating_sub(1));
    let x_offset_in_band = abs_x % col_w.max(1);

    let area_tail_col = (area_right / col_w).min(v.saturating_sub(1));
    let span_cols = area_tail_col.saturating_sub(playhead_col);

    let matcher = self.text_matcher.lock().unwrap();
    let sweep_row = *self.sweep_row_mode.lock().unwrap();
    if let Some(ref m) = *matcher {
      (0..grid_height)
        .flat_map(|y| {
          if !sweep_row.is_row_active(y) {
            return vec![];
          }
          let row_idx = (y / row_h).min(h.saturating_sub(1));
          let base_col =
            match tilt_mode.sweep_col_for_row(playhead_col, playhead_row, row_idx, v, h) {
              Some(c) => c,
              None => return vec![],
            };
          (0..=span_cols)
            .filter_map(|col_off| {
              let col = (base_col + col_off).min(v.saturating_sub(1));
              let tilt_x = col * col_w + x_offset_in_band;
              let crosshair_index = y * grid_width + tilt_x;
              let note_pos = if grid_height > 0 {
                tilt_x % grid_height
              } else {
                tilt_x
              };
              let channel = (row_idx * v + col) as u8;
              if crosshair_index != curr_running_playhead
                && exclude != Some((note_pos, channel))
                && m.contains_key(&crosshair_index)
              {
                Some(crosshair_index)
              } else {
                None
              }
            })
            .collect::<Vec<_>>()
        })
        .collect()
    } else {
      Vec::new()
    }
  }

  /// Trigger MIDI for sweep mode (vertical crosshair, tilt-adjusted per row)
  pub fn trigger_midi_if_matched_sweep(
    &self,
    curr_running_playhead: usize,
    abs_x: usize,
    active_pos_y: usize,
    area_right: usize,
    exclude: Option<(usize, u8)>,
  ) {
    if !self.modes.sweep_mode.load(Ordering::Relaxed) {
      return;
    }

    let grid_width = self.grid.width.load(Ordering::Relaxed);
    let grid_height = self.grid.height.load(Ordering::Relaxed);
    let current_tempo = self.music.tempo.load(Ordering::Relaxed);
    let x_scale_mode = *self.music.scale_mode_top.lock().unwrap();

    // Precompute tilt sweep geometry (mirrors printer.rs logic)
    let tilt_mode = *self.tilt_mode.lock().unwrap();
    let playhead_pos = *self.playhead_pos.lock().unwrap();
    let v = self.grid.v_splits.load(Ordering::Relaxed).max(1);
    let h = self.grid.h_splits.load(Ordering::Relaxed).max(1);
    let col_w = (grid_width / v).max(1);
    let row_h = (grid_height / h).max(1);
    let playhead_col = (playhead_pos.x / col_w).min(v.saturating_sub(1));
    let playhead_row = (playhead_pos.y / row_h).min(h.saturating_sub(1));
    let x_offset_in_band = abs_x % col_w.max(1);

    // Number of extra columns spanned by the playhead area beyond the head column.
    let area_tail_col = (area_right / col_w).min(v.saturating_sub(1));
    let span_cols = area_tail_col.saturating_sub(playhead_col);

    // Collect matched (y, tilt_x) pairs. For each row, check every spanned column so
    // that matches in all channels covered by the playhead area are triggered.
    let sweep_row = *self.sweep_row_mode.lock().unwrap();
    let matched: Vec<(usize, usize)> = {
      let matcher = self.text_matcher.lock().unwrap();
      if let Some(ref m) = *matcher {
        (0..grid_height)
          .flat_map(|y| {
            if !sweep_row.is_row_active(y) {
              return vec![];
            }
            let row_idx = (y / row_h).min(h.saturating_sub(1));
            let base_col =
              match tilt_mode.sweep_col_for_row(playhead_col, playhead_row, row_idx, v, h) {
                Some(c) => c,
                None => return vec![],
              };
            (0..=span_cols)
              .filter_map(|col_off| {
                let col = (base_col + col_off).min(v.saturating_sub(1));
                let tilt_x = col * col_w + x_offset_in_band;
                let crosshair_index = y * grid_width + tilt_x;
                if crosshair_index != curr_running_playhead && m.contains_key(&crosshair_index) {
                  Some((y, tilt_x))
                } else {
                  None
                }
              })
              .collect::<Vec<_>>()
          })
          .collect()
      } else {
        Vec::new()
      }
    };

    // Deduplicate matched cells by (note_pos, channel): among cells that would
    // produce the same MIDI note on the same channel, keep only the one nearest
    // to active_pos_y (highest proximity velocity). Also skip any (note_pos, channel)
    // that the playhead already triggered at higher priority.
    let default_length = 4;
    let scale_root_offset = self.music.scale_root_top.lock().unwrap().to_root_offset();
    let div = self.music.ratio.lock().unwrap().1;

    // key: (note_pos, channel) → (distance_to_active, y, tilt_x)
    let mut best: HashMap<(usize, u8), (usize, usize, usize)> = HashMap::new();
    for (y, tilt_x) in matched {
      let note_pos = if grid_height > 0 {
        tilt_x % grid_height
      } else {
        tilt_x
      };
      let col_idx = (tilt_x / col_w).min(v.saturating_sub(1));
      let row_idx = (y / row_h).min(h.saturating_sub(1));
      let channel = (row_idx * v + col_idx) as u8;
      if exclude == Some((note_pos, channel)) {
        continue;
      }
      let dist = (y as i32 - active_pos_y as i32).unsigned_abs() as usize;
      let entry = best.entry((note_pos, channel)).or_insert((dist, y, tilt_x));
      if dist < entry.0 {
        *entry = (dist, y, tilt_x);
      }
    }

    for ((note_pos, _), (_, y, tilt_x)) in best {
      let _ = self
        .midi_tx
        .send(midi::Message::TriggerWithPosition(midi::TriggerParams {
          y_position: note_pos,
          grid_height,
          x_position: tilt_x,
          grid_width,
          grid_v_splits: self.grid.v_splits.load(Ordering::Relaxed),
          grid_h_splits: self.grid.h_splits.load(Ordering::Relaxed),
          scale_mode: x_scale_mode,
          scale_root_offset,
          bpm: current_tempo,
          trigger_pos_y: y,
          active_pos_y,
          distance_to_next: default_length,
          hold: false,
          is_sweep: true,
          div,
        }));
    }
  }

  pub fn release_drone_notes_with_tail(&self) {
    let old_notes: Vec<midi::MidiMsg> = {
      let mut held = self.drone_held_notes.lock().unwrap();
      std::mem::take(&mut *held)
    };
    if old_notes.is_empty() {
      return;
    }
    let gen = self.drone_release_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let tx = self.midi_tx.clone();
    let gen_arc = Arc::clone(&self.drone_release_generation);
    thread::Builder::new()
      .name(consts::THREAD_NAME_DRONE_NOTE_OFF.to_string())
      .spawn(move || {
        thread::sleep(Duration::from_millis(80));
        if gen_arc.load(Ordering::SeqCst) == gen {
          for msg in old_notes {
            let _ = tx.send(midi::Message::Trigger(msg, false));
          }
        }
      })
      .expect("failed to spawn drone-note-off thread");
  }

  /// Release held drone notes with tail, then immediately trigger new notes at `drone_x_rel`
  /// (relative offset from `area.left()`) for every row in `area` that has a regex match.
  /// Notes are derived from the left keyboard (scale_mode_left / scale_root_left).
  pub fn trigger_drone_at_x(&self, drone_x_rel: usize, area: &Rect) {
    self.release_drone_notes_with_tail();

    let abs_x = area.left() + drone_x_rel;
    let grid_width = self.grid.width.load(Ordering::Relaxed);
    let grid_height = self.grid.height.load(Ordering::Relaxed);
    if grid_width == 0 || grid_height == 0 || abs_x >= grid_width {
      return;
    }

    let scale_mode = *self.music.scale_mode_left.lock().unwrap();
    let scale_root_offset = self.music.scale_root_left.lock().unwrap().to_root_offset();
    let v_splits = self.grid.v_splits.load(Ordering::Relaxed);
    let h_splits = self.grid.h_splits.load(Ordering::Relaxed);
    // drone_channel is 1-indexed; convert to 0-indexed for MIDI.
    // When channel is 1 (default), derive channel from grid splits as normal.
    let drone_channel = self.modes.drone_channel.load(Ordering::Relaxed);

    let new_notes: Vec<midi::MidiMsg> = {
      let matcher = self.text_matcher.lock().unwrap();
      if let Some(ref m) = *matcher {
        let mut notes = Vec::new();
        for y in 0..grid_height {
          if area.contains((abs_x, y).into()) {
            continue;
          }
          let cell_index = abs_x + y * grid_width;
          if m.contains_key(&cell_index) {
            let velocity = calculate_velocity(y, grid_height, 100.0, 10.0);
            let channel = if drone_channel > 1 {
              (drone_channel - 1) as u8
            } else {
              calculate_channel(abs_x, y, grid_width, grid_height, v_splits, h_splits)
            };
            let (note_index, octave) =
              scale_mode.pos_to_scale_note(y, grid_height, consts::BASE_OCTAVE, scale_root_offset);
            // length=127 is a placeholder; release is managed manually
            notes.push(midi::MidiMsg::from(
              note_index, octave, 127, velocity, channel,
            ));
          }
        }
        notes
      } else {
        Vec::new()
      }
    };

    {
      let mut held = self.drone_held_notes.lock().unwrap();
      *held = new_notes.clone();
    }
    for msg in new_notes {
      let _ = self.midi_tx.send(midi::Message::Trigger(msg, true));
    }
  }

  /// Hold operation - sets flag to hold next note
  pub fn h_op(&self) {
    self.modes.hold_next_note.store(true, Ordering::Relaxed);
  }

  /// Ratchet operation - rapid retriggering with velocity decay
  pub fn r_op(&self, abs_x: usize, abs_y: usize) {
    let scale_mode = *self.music.scale_mode_top.lock().unwrap();
    let scale_root_offset = self.music.scale_root_top.lock().unwrap().to_root_offset();
    let grid_height = self.grid.height.load(Ordering::Relaxed);
    let grid_width = self.grid.width.load(Ordering::Relaxed);

    if grid_height == 0 {
      return;
    }

    let note_position = abs_x % grid_height;
    let (note_index, octave) = scale_mode.pos_to_scale_note(
      note_position,
      grid_height,
      consts::BASE_OCTAVE,
      scale_root_offset,
    );

    let base_velocity = calculate_velocity(abs_y, grid_height, 100.0, 10.0);

    let channel = calculate_channel(
      abs_x,
      abs_y,
      grid_width,
      grid_height,
      self.grid.v_splits.load(Ordering::Relaxed),
      self.grid.h_splits.load(Ordering::Relaxed),
    );

    let ratchet_count = 4_u8;
    const INTERVAL_MS: u64 = 100;
    let note_duration_ms = ((INTERVAL_MS * 60) / 100).max(15);
    let note_length = ((note_duration_ms / 8) as u8).max(1);

    let my_gen = self.ratchet_generation.fetch_add(1, Ordering::SeqCst) + 1;
    self.modes.is_ratcheting.store(true, Ordering::SeqCst);

    let is_ratcheting = Arc::clone(&self.modes.is_ratcheting);
    let ratchet_generation = Arc::clone(&self.ratchet_generation);
    let midi_tx = self.midi_tx.clone();

    thread::Builder::new()
      .name(crate::core::consts::THREAD_NAME_RATCHET.to_string())
      .spawn(move || {
        for i in 0..ratchet_count {
          if ratchet_generation.load(Ordering::SeqCst) != my_gen {
            break;
          }

          // velocity decays from base → 30 % of base over the run
          let decay = 1.0_f32 - (i as f32 / ratchet_count as f32) * 0.70;
          let velocity = ((base_velocity as f32 * decay).round() as u8).max(10);

          let midi_msg = midi::MidiMsg::from(note_index, octave, note_length, velocity, channel);
          let _ = midi_tx.send(midi::Message::Trigger(midi_msg.clone(), true));

          {
            let tx_off = midi_tx.clone();
            let gen_off = my_gen;
            let gen_arc = Arc::clone(&ratchet_generation);
            thread::Builder::new()
              .name(crate::core::consts::THREAD_NAME_RATCHET_NOTE_OFF.to_string())
              .spawn(move || {
                thread::sleep(Duration::from_millis(note_duration_ms));
                if gen_arc.load(Ordering::SeqCst) == gen_off {
                  let _ = tx_off.send(midi::Message::Trigger(midi_msg, false));
                }
              })
              .expect("Failed to spawn ratchet-note-off thread");
          }

          thread::sleep(Duration::from_millis(INTERVAL_MS));
        }

        if ratchet_generation.load(Ordering::SeqCst) == my_gen {
          is_ratcheting.store(false, Ordering::Relaxed);
        }
      })
      .expect("failed to spawn ratchet thread");
  }

  /// Chord operation - trigger triad (root, third, fifth)
  pub fn c_op(&self, abs_x: usize, abs_y: usize, distance_to_next: usize) {
    let scale_mode = *self.music.scale_mode_top.lock().unwrap();
    let scale_root_offset = self.music.scale_root_top.lock().unwrap().to_root_offset();
    let grid_height = self.grid.height.load(Ordering::Relaxed);

    if grid_height == 0 {
      return;
    }

    let note_position = abs_x % grid_height;

    // Get the base note
    let (_base_note_index, base_octave) = scale_mode.pos_to_scale_note(
      note_position,
      grid_height,
      consts::BASE_OCTAVE,
      scale_root_offset,
    );

    // For a triad: root (0), third (2 scale degrees up), fifth (4 scale degrees up)
    let scale_intervals = scale_mode.intervals();
    let scale_length = scale_intervals.len();

    // Calculate base scale degree
    let inverted_y = grid_height.saturating_sub(1).saturating_sub(note_position);
    let base_scale_degree = inverted_y % scale_length;

    let velocity = calculate_velocity(abs_y, grid_height, 100.0, 10.0);

    let current_tempo = self.music.tempo.load(Ordering::Relaxed);
    let base_bpm = consts::DEFAULT_TEMPO;

    let calculated_length = if current_tempo > 0 {
      ((distance_to_next * base_bpm) / current_tempo).max(1)
    } else {
      distance_to_next
    };
    let note_length = (calculated_length as u8).min(127);

    // Chord notes: root, third, fifth
    let chord_degrees = [0, 2, 4];

    let channel = calculate_channel(
      abs_x,
      abs_y,
      self.grid.width.load(Ordering::Relaxed),
      grid_height,
      self.grid.v_splits.load(Ordering::Relaxed),
      self.grid.h_splits.load(Ordering::Relaxed),
    );

    let mut chord_notes = Vec::new();

    for &degree_offset in &chord_degrees {
      let target_scale_degree = (base_scale_degree + degree_offset) % scale_length;
      let octave_jump = (base_scale_degree + degree_offset) / scale_length;

      let interval = scale_intervals[target_scale_degree];
      let raw_pitch = interval + (scale_root_offset % 12) as f32;
      let note_index = raw_pitch % 12.0;
      let extra_octave = (raw_pitch / 12.0).floor() as u8;
      let final_octave = base_octave + octave_jump as u8 + extra_octave;

      let midi_msg = midi::MidiMsg::from(note_index, final_octave, note_length, velocity, channel);
      chord_notes.push(midi_msg);
    }

    for midi_msg in &chord_notes {
      let _ = self
        .midi_tx
        .send(midi::Message::Trigger(midi_msg.clone(), true));
    }

    // Use BPM-aware timing for chord duration (matches Stack behavior)
    let chord_duration_ms = (note_length as u64 * 8).max(50);

    let midi_tx_clone = self.midi_tx.clone();
    thread::Builder::new()
      .name(crate::core::consts::THREAD_NAME_CHORD_NOTE_OFF.to_string())
      .spawn(move || {
        thread::sleep(Duration::from_millis(chord_duration_ms));
        for midi_msg in chord_notes {
          let _ = midi_tx_clone.send(midi::Message::Trigger(midi_msg, false));
        }
      })
      .expect("Failed to spawn chord-note-off thread");
  }
}

#[cfg(test)]
mod tests {
  use super::{calculate_channel, calculate_velocity};

  #[test]
  fn channel_no_splits_is_always_zero() {
    assert_eq!(calculate_channel(0, 0, 8, 8, 1, 1), 0);
    assert_eq!(calculate_channel(7, 7, 8, 8, 1, 1), 0);
    assert_eq!(calculate_channel(3, 5, 8, 8, 1, 1), 0);
  }

  #[test]
  fn channel_two_vertical_splits_left_right() {
    assert_eq!(calculate_channel(0, 0, 8, 8, 2, 1), 0);
    assert_eq!(calculate_channel(3, 0, 8, 8, 2, 1), 0);
    assert_eq!(calculate_channel(4, 0, 8, 8, 2, 1), 1);
    assert_eq!(calculate_channel(7, 0, 8, 8, 2, 1), 1);
  }

  #[test]
  fn channel_two_horizontal_splits_top_bottom() {
    assert_eq!(calculate_channel(0, 0, 8, 8, 1, 2), 0);
    assert_eq!(calculate_channel(0, 3, 8, 8, 1, 2), 0);
    assert_eq!(calculate_channel(0, 4, 8, 8, 1, 2), 1);
    assert_eq!(calculate_channel(0, 7, 8, 8, 1, 2), 1);
  }

  #[test]
  fn channel_two_by_two_splits_four_quadrants() {
    assert_eq!(calculate_channel(0, 0, 8, 8, 2, 2), 0);
    assert_eq!(calculate_channel(4, 0, 8, 8, 2, 2), 1);
    assert_eq!(calculate_channel(0, 4, 8, 8, 2, 2), 2);
    assert_eq!(calculate_channel(4, 4, 8, 8, 2, 2), 3);
  }

  #[test]
  fn channel_position_exactly_on_split_boundary() {
    let col_w = 10;
    let grid_w = 20;
    assert_eq!(calculate_channel(col_w, 0, grid_w, 8, 2, 1), 1);
    assert_eq!(calculate_channel(col_w - 1, 0, grid_w, 8, 2, 1), 0);
  }

  #[test]
  fn channel_out_of_bounds_position_is_clamped_not_panicked() {
    let ch = calculate_channel(999, 999, 8, 8, 2, 2);
    assert!(ch <= 3, "expected channel <= 3, got {ch}");
  }

  #[test]
  fn channel_zero_grid_dimensions_does_not_panic() {
    let ch = calculate_channel(0, 0, 0, 0, 1, 1);
    assert_eq!(ch, 0);
  }

  #[test]
  fn channel_zero_splits_treated_as_one() {
    assert_eq!(calculate_channel(5, 5, 8, 8, 0, 0), 0);
  }

  #[test]
  fn velocity_top_row_returns_max() {
    let v = calculate_velocity(0, 8, 100.0, 10.0);
    assert_eq!(v, 100);
  }

  #[test]
  fn velocity_bottom_row_approaches_min() {
    let v = calculate_velocity(7, 8, 100.0, 10.0);

    assert!(v >= 10, "velocity {v} must be >= min_vel 10");
    assert!(v <= 100, "velocity {v} must be <= max_vel 100");
  }

  #[test]
  fn velocity_zero_grid_height_returns_default_64() {
    let v = calculate_velocity(0, 0, 100.0, 10.0);
    assert_eq!(v, 64);
  }

  #[test]
  fn velocity_always_within_min_max_range() {
    for y in 0..16_usize {
      let v = calculate_velocity(y, 16, 100.0, 10.0);
      assert!(
        (10..=100).contains(&v),
        "y={y} produced velocity {v} outside [10, 100]"
      );
    }
  }

  #[test]
  fn velocity_decreases_monotonically_with_depth() {
    let grid_h = 16;
    let mut prev = calculate_velocity(0, grid_h, 100.0, 10.0);
    for y in 1..grid_h {
      let curr = calculate_velocity(y, grid_h, 100.0, 10.0);
      assert!(
        curr <= prev,
        "velocity increased from y={} (v={prev}) to y={y} (v={curr})",
        y - 1
      );
      prev = curr;
    }
  }

  #[test]
  fn velocity_equal_min_max_returns_that_value() {
    for y in 0..8 {
      let v = calculate_velocity(y, 8, 64.0, 64.0);
      assert_eq!(v, 64, "y={y} should give exactly 64 when min==max");
    }
  }
}
