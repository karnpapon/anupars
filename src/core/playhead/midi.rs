use cursive::Vec2;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::core::{consts, engine::regex::Match, io::midi, tonal::scale};

// ============================================================================
// MIDI Helper Functions
// ============================================================================

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

// ============================================================================
// MIDI Trigger Handler
// ============================================================================

pub struct MidiTriggerHandler {
  pub midi_tx: Sender<midi::Message>,
  pub grid_width: Arc<AtomicUsize>,
  pub grid_height: Arc<AtomicUsize>,
  pub grid_v_splits: Arc<AtomicUsize>,
  pub grid_h_splits: Arc<AtomicUsize>,
  pub tempo: Arc<AtomicUsize>,
  pub scale_mode_top: Arc<Mutex<scale::ScaleMode>>,
  pub scale_mode_left: Arc<Mutex<scale::ScaleMode>>,
  pub scale_root_top: Arc<Mutex<scale::ScaleRoot>>,
  pub prev_active_pos: Arc<Mutex<Vec2>>,
  pub hold_next_note: Arc<AtomicBool>,
  pub is_ratcheting: Arc<AtomicBool>,
  pub ratchet_generation: Arc<AtomicUsize>,
  pub text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,
  pub sweep_mode: Arc<AtomicBool>,
  pub dyn_length_mode: Arc<AtomicBool>,
  pub arpeggiator_mode: Arc<AtomicBool>,
}

impl MidiTriggerHandler {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    midi_tx: Sender<midi::Message>,
    grid_width: Arc<AtomicUsize>,
    grid_height: Arc<AtomicUsize>,
    grid_v_splits: Arc<AtomicUsize>,
    grid_h_splits: Arc<AtomicUsize>,
    tempo: Arc<AtomicUsize>,
    scale_mode_top: Arc<Mutex<scale::ScaleMode>>,
    scale_mode_left: Arc<Mutex<scale::ScaleMode>>,
    scale_root_top: Arc<Mutex<scale::ScaleRoot>>,
    prev_active_pos: Arc<Mutex<Vec2>>,
    hold_next_note: Arc<AtomicBool>,
    is_ratcheting: Arc<AtomicBool>,
    ratchet_generation: Arc<AtomicUsize>,
    text_matcher: Arc<Mutex<Option<HashMap<usize, Match>>>>,
    sweep_mode: Arc<AtomicBool>,
    dyn_length_mode: Arc<AtomicBool>,
    arpeggiator_mode: Arc<AtomicBool>,
  ) -> Self {
    MidiTriggerHandler {
      midi_tx,
      grid_width,
      grid_height,
      grid_v_splits,
      grid_h_splits,
      tempo,
      scale_mode_top,
      scale_mode_left,
      scale_root_top,
      prev_active_pos,
      hold_next_note,
      is_ratcheting,
      ratchet_generation,
      text_matcher,
      sweep_mode,
      dyn_length_mode,
      arpeggiator_mode,
    }
  }

  /// Determine note position and scale mode based on movement
  pub fn determine_note_position_and_scale(
    &self,
    active_pos: Vec2,
    abs_x: usize,
    abs_y: usize,
  ) -> (usize, scale::ScaleMode) {
    let mut prev_active = self.prev_active_pos.lock().unwrap();
    *prev_active = active_pos;
    drop(prev_active);

    let grid_height = self.grid_height.load(Ordering::Relaxed);
    let pos = if grid_height > 0 {
      abs_x % grid_height
    } else {
      abs_y
    };
    let scale = *self.scale_mode_top.lock().unwrap();
    (pos, scale)
  }

  /// Trigger MIDI for a matched position
  pub fn trigger_midi_if_matched(
    &self,
    _curr_running_playhead: usize,
    note_position: usize,
    scale_mode: scale::ScaleMode,
    playhead_pos_x: usize,
    playhead_pos_y: usize,
    distance_to_next: usize,
  ) {
    let grid_height = self.grid_height.load(Ordering::Relaxed);
    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let current_tempo = self.tempo.load(Ordering::Relaxed);
    let hold_next = self.hold_next_note.load(Ordering::Relaxed);

    let _ = self
      .midi_tx
      .send(midi::Message::TriggerWithPosition(midi::TriggerParams {
        y_position: note_position,
        grid_height,
        x_position: playhead_pos_x,
        grid_width,
        grid_v_splits: self.grid_v_splits.load(Ordering::Relaxed),
        grid_h_splits: self.grid_h_splits.load(Ordering::Relaxed),
        scale_mode,
        scale_root_offset: self.scale_root_top.lock().unwrap().to_root_offset(),
        bpm: current_tempo,
        trigger_pos_y: playhead_pos_y,
        active_pos_y: playhead_pos_y,
        distance_to_next,
        hold: hold_next,
        is_sweep: false,
      }));
  }

  /// Trigger MIDI for sweep mode (vertical crosshair)
  pub fn trigger_midi_if_matched_sweep(
    &self,
    curr_running_playhead: usize,
    abs_x: usize,
    active_pos_y: usize,
  ) {
    if !self.sweep_mode.load(Ordering::Relaxed) {
      return;
    }

    let grid_width = self.grid_width.load(Ordering::Relaxed);
    let grid_height = self.grid_height.load(Ordering::Relaxed);
    let current_tempo = self.tempo.load(Ordering::Relaxed);
    let x_scale_mode = *self.scale_mode_top.lock().unwrap();

    // Collect all matched y positions for the current x
    let matched_y_positions: Vec<usize> = {
      let matcher = self.text_matcher.lock().unwrap();
      if let Some(ref m) = *matcher {
        (0..grid_height)
          .filter(|&y| {
            let crosshair_index = y * grid_width + abs_x;
            // Skip current playhead position to avoid duplicate MIDI trigger
            crosshair_index != curr_running_playhead && m.contains_key(&crosshair_index)
          })
          .collect()
      } else {
        Vec::new()
      }
    };

    // If we have matched positions, send a single MIDI trigger with average velocity
    if !matched_y_positions.is_empty() {
      // Calculate average y position for velocity reference
      let avg_y = matched_y_positions.iter().sum::<usize>() / matched_y_positions.len();
      let default_length = 4;

      let _ = self
        .midi_tx
        .send(midi::Message::TriggerWithPosition(midi::TriggerParams {
          y_position: avg_y,
          grid_height,
          x_position: abs_x,
          grid_width,
          grid_v_splits: self.grid_v_splits.load(Ordering::Relaxed),
          grid_h_splits: self.grid_h_splits.load(Ordering::Relaxed),
          scale_mode: x_scale_mode,
          scale_root_offset: self.scale_root_top.lock().unwrap().to_root_offset(),
          bpm: current_tempo,
          trigger_pos_y: avg_y,
          active_pos_y,
          distance_to_next: default_length,
          hold: false,
          is_sweep: true,
        }));
    }
  }

  // ============================================================================
  // Event Operators (r, c, h)
  // ============================================================================

  /// Hold operation - sets flag to hold next note
  pub fn h_op(&self) {
    self.hold_next_note.store(true, Ordering::Relaxed);
  }

  /// Ratchet operation - rapid retriggering with velocity decay
  pub fn r_op(&self, abs_x: usize, abs_y: usize) {
    let scale_mode = *self.scale_mode_top.lock().unwrap();
    let scale_root_offset = self.scale_root_top.lock().unwrap().to_root_offset();
    let grid_height = self.grid_height.load(Ordering::Relaxed);
    let grid_width = self.grid_width.load(Ordering::Relaxed);

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
      self.grid_v_splits.load(Ordering::Relaxed),
      self.grid_h_splits.load(Ordering::Relaxed),
    );

    let ratchet_count = 4_u8;
    const INTERVAL_MS: u64 = 100;
    let note_duration_ms = ((INTERVAL_MS * 60) / 100).max(15);
    let note_length = ((note_duration_ms / 8) as u8).max(1);

    let my_gen = self.ratchet_generation.fetch_add(1, Ordering::SeqCst) + 1;
    self.is_ratcheting.store(true, Ordering::SeqCst);

    let is_ratcheting = Arc::clone(&self.is_ratcheting);
    let ratchet_generation = Arc::clone(&self.ratchet_generation);
    let midi_tx = self.midi_tx.clone();

    thread::Builder::new()
      .name("ratchet".to_string())
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
            thread::spawn(move || {
              thread::sleep(Duration::from_millis(note_duration_ms));
              if gen_arc.load(Ordering::SeqCst) == gen_off {
                let _ = tx_off.send(midi::Message::Trigger(midi_msg, false));
              }
            });
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
    let scale_mode = *self.scale_mode_top.lock().unwrap();
    let scale_root_offset = self.scale_root_top.lock().unwrap().to_root_offset();
    let grid_height = self.grid_height.load(Ordering::Relaxed);

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

    let current_tempo = self.tempo.load(Ordering::Relaxed);
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
      self.grid_width.load(Ordering::Relaxed),
      grid_height,
      self.grid_v_splits.load(Ordering::Relaxed),
      self.grid_h_splits.load(Ordering::Relaxed),
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
    thread::spawn(move || {
      thread::sleep(Duration::from_millis(chord_duration_ms));
      for midi_msg in chord_notes {
        let _ = midi_tx_clone.send(midi::Message::Trigger(midi_msg, false));
      }
    });
  }
}
