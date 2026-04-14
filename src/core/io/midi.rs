#[cfg(not(target_arch = "wasm32"))]
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection, MidiOutputPort};
#[cfg(not(target_arch = "wasm32"))]
use std::error::Error;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
// use std::time::Duration;

use crate::core::consts;
use crate::core::engine::stack::{self, Stack};
use crate::core::tonal::scale;

// Constants for note length calculation
const BASE_LENGTH_FIXED: usize = 12; // 96ms at 120 BPM - for DynLength OFF
const BASE_LENGTH_DYNAMIC: usize = 32; // 256ms at 120 BPM - for DynLength ON
const MIN_AUDIBLE_LENGTH: u8 = 10; // 80ms minimum
const DYNLENGTH_DEFAULT_DISTANCE: usize = 4;

// Constants for velocity calculation
const MAX_VELOCITY: f32 = 100.0;
const MIN_VELOCITY: f32 = 10.0;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MidiStatusMsg {
  NoteOff = 0x80,
  NoteOn = 0x90,
  Stop = 0xFC,
  Start = 0xFA,
  Continue = 0xFB,
  Clock = 0xF8,
  SongPositionPointer = 0xF2,
  BitMask7 = 0x7F,
  AllNotesOff = 0x7B,
  AllSoundOff = 0x78,
  ControlChange = 0xB0,
  PitchBend = 0xE0,
}

impl TryFrom<u8> for MidiStatusMsg {
  type Error = u8;
  fn try_from(byte: u8) -> Result<Self, Self::Error> {
    match byte {
      0x80 => Ok(MidiStatusMsg::NoteOff),
      0x90 => Ok(MidiStatusMsg::NoteOn),
      0xFC => Ok(MidiStatusMsg::Stop),
      0xFA => Ok(MidiStatusMsg::Start),
      0xFB => Ok(MidiStatusMsg::Continue),
      0xF8 => Ok(MidiStatusMsg::Clock),
      0xF2 => Ok(MidiStatusMsg::SongPositionPointer),
      0x7F => Ok(MidiStatusMsg::BitMask7),
      0x7B => Ok(MidiStatusMsg::AllNotesOff),
      0x78 => Ok(MidiStatusMsg::AllSoundOff),
      0xB0 => Ok(MidiStatusMsg::ControlChange),
      other => Err(other),
    }
  }
}

impl From<MidiStatusMsg> for u8 {
  fn from(m: MidiStatusMsg) -> u8 {
    m as u8
  }
}

/// Parameters for MIDI trigger with position information
#[derive(Clone, Copy, Debug)]
pub struct TriggerParams {
  pub y_position: usize,
  pub grid_height: usize,
  pub x_position: usize,
  pub grid_width: usize,
  pub grid_v_splits: usize,
  pub grid_h_splits: usize,
  pub scale_mode: scale::ScaleMode,
  pub scale_root_offset: u8,
  pub bpm: usize,
  pub trigger_pos_y: usize,
  pub active_pos_y: usize,
  pub distance_to_next: usize,
  pub hold: bool,
  pub is_sweep: bool,
  /// ratio.1 from the playhead DIV setting (e.g. 8, 16, 32, 64).
  /// Used to cap note length so it doesn't exceed the step duration.
  pub div: usize,
}

#[derive(Clone, Debug)]
pub enum Message {
  Push(MidiMsg),
  Hold(MidiMsg),
  Release(MidiMsg),
  Trigger(MidiMsg, bool), // (msg, is_pressed)
  SetMsgConfig(MidiMsg),  // ? maybe obsolete, TBD
  ClearMsgConfig(),
  TriggerWithPosition(TriggerParams),
  ControlChange {
    channel: u8,
    cc_number: u8,
    cc_value: u8,
  },
  SwitchDevice(usize),
  Panic(),
  SetTempo(usize),
  ReleaseAll(),
  // MIDI Clock messages
  ClockStart(),
  ClockStop(),
  ClockTick(),
  ClockContinue(),
  ClockSongPosition(usize),
  EnableClock(bool),
  ExternalClock(u8),
  EnableClockInput(bool),
  ConnectInput(usize),
  DisconnectOutput(),
}

#[derive(Clone, Debug)]
pub struct MidiMsg {
  pub note: f32, // Can be fractional for microtonal scales
  pub velocity: u8,
  pub octave: u8,
  pub channel: u8,
  pub length: u8,
}

impl PartialEq for MidiMsg {
  fn eq(&self, other: &Self) -> bool {
    self.note.to_bits() == other.note.to_bits()
      && self.octave == other.octave
      && self.channel == other.channel
  }
}

impl MidiMsg {
  pub fn from(note: f32, octave: u8, length: u8, velocity: u8, channel: u8) -> MidiMsg {
    Self {
      note,
      octave,
      length,
      velocity,
      channel,
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
pub struct Midi {
  pub midi: Mutex<Option<MidiOutput>>,
  pub out_device: Mutex<Option<MidiOutputConnection>>,
  pub out_device_name: Mutex<Option<String>>,
  pub msg_config_list: Arc<Mutex<Vec<MidiMsg>>>,
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  tempo: Arc<Mutex<usize>>,
  clock_enabled: Arc<Mutex<bool>>,

  /// for tracking last held note for stuck note clearing
  last_held_note: Arc<Mutex<Option<MidiMsg>>>,
  /// Keeps the MIDI input connection alive, dropping it disconnects
  pub in_connection: Mutex<Option<MidiInputConnection<()>>>,
  /// only dispatch clock bytes when true
  pub clock_input_enabled: Arc<AtomicBool>,
  #[allow(clippy::type_complexity)]
  /// calling from run() on every dispatched ExternalClock byte
  ext_clock_callback: Mutex<Option<Box<dyn Fn(u8) + Send>>>,
}

/// WASM stub: no real MIDI hardware, just keeps channels alive.
#[cfg(target_arch = "wasm32")]
pub struct Midi {
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  pub msg_config_list: Arc<Mutex<Vec<MidiMsg>>>,
  #[allow(dead_code)]
  clock_input_enabled: Arc<AtomicBool>,
}

#[cfg(not(target_arch = "wasm32"))]
impl Midi {
  fn handle_push(&self, stack_tx: std::sync::mpsc::Sender<stack::Message>, midi_msg: MidiMsg) {
    let _ = stack_tx.send(stack::Message::Push(midi_msg));
  }

  fn handle_hold(&self, stack_tx: std::sync::mpsc::Sender<stack::Message>, midi_msg: MidiMsg) {
    let _ = stack_tx.send(stack::Message::Hold(midi_msg));
  }

  fn handle_release(&self, stack_tx: std::sync::mpsc::Sender<stack::Message>, midi_msg: MidiMsg) {
    let _ = stack_tx.send(stack::Message::Release(midi_msg));
  }

  fn handle_release_all(&self, stack_tx: std::sync::mpsc::Sender<stack::Message>) {
    let _ = stack_tx.send(stack::Message::ReleaseAll());
    let mut last_held = self.last_held_note.lock().unwrap();
    *last_held = None;
  }

  fn handle_trigger(&self, midi_msg: MidiMsg, is_pressed: bool) {
    self.trigger(&midi_msg, is_pressed).unwrap();
  }

  fn handle_trigger_with_position(&self, params: TriggerParams) {
    self.trigger_w_position(params);
  }

  fn handle_set_tempo(&self, bpm: usize) {
    let mut tempo = self.tempo.lock().unwrap();
    *tempo = bpm;
  }

  fn handle_switch_device(&self, port_index: usize) {
    if let Err(e) = self.switch_device(port_index) {
      eprintln!("Error switching MIDI device: {}", e);
    }
  }

  fn handle_disconnect_output(&self) {
    self.send_all_notes_off();
    *self.out_device.lock().unwrap() = None;
    *self.out_device_name.lock().unwrap() = None;
  }

  fn handle_panic(&self) {
    self.send_all_notes_off();
    let mut last_held = self.last_held_note.lock().unwrap();
    *last_held = None;
  }

  fn handle_clock_start(&self) {
    self.send_clock_start();
  }

  fn handle_clock_stop(&self) {
    self.send_clock_stop();
  }

  fn handle_clock_tick(&self) {
    self.send_clock_tick();
  }

  fn handle_clock_continue(&self) {
    self.send_clock_continue();
  }

  fn handle_clock_song_position(&self, position: usize) {
    self.send_song_position_pointer(position);
  }

  fn handle_external_clock(&self, byte: u8) {
    let cb = self.ext_clock_callback.lock().unwrap();
    if let Some(ref f) = *cb {
      f(byte);
    }
  }

  fn handle_enable_clock_input(&self, en: bool) {
    self.clock_input_enabled.store(en, Ordering::Relaxed);
  }

  fn handle_control_change(&self, channel: u8, cc_number: u8, cc_value: u8) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        let _ = connection_out.send(&[
          u8::from(MidiStatusMsg::ControlChange) + (channel & 0x0F),
          cc_number,
          cc_value,
        ]);
      }
    }
  }

  fn handle_connect_input(&self, port_index: usize) {
    *self.in_connection.lock().unwrap() = None;
    let Ok(new_inp) = MidiInput::new("MIDI Input") else {
      return;
    };
    let ports = new_inp.ports();
    let Some(port) = ports.get(port_index) else {
      return;
    };
    let inp_tx = self.tx.clone();
    let clock_enabled = Arc::clone(&self.clock_input_enabled);
    if let Ok(conn) = new_inp.connect(
      port,
      "MIDI Input",
      move |_stamp, msg, _| {
        if clock_enabled.load(Ordering::Relaxed) && !msg.is_empty() {
          if let Ok(m) = MidiStatusMsg::try_from(msg[0]) {
            #[allow(clippy::collapsible_match)]
            match m {
              MidiStatusMsg::Clock
              | MidiStatusMsg::Start
              | MidiStatusMsg::Continue
              | MidiStatusMsg::Stop => {
                let _ = inp_tx.send(Message::ExternalClock(msg[0]));
              }
              _ => {}
            }
          }
        }
      },
      (),
    ) {
      *self.in_connection.lock().unwrap() = Some(conn);
    }
  }

  pub fn get_available_devices(&self) -> Vec<(String, usize)> {
    let midi_lock = self.midi.lock().unwrap();
    if let Some(midi_out) = midi_lock.as_ref() {
      let out_ports = midi_out.ports();
      out_ports
        .iter()
        .enumerate()
        .map(|(i, p)| {
          let name = midi_out
            .port_name(p)
            .unwrap_or_else(|_| format!("Port {}", i));
          (name, i)
        })
        .collect()
    } else {
      Vec::new()
    }
  }

  // #[allow(dead_code)]
  // pub fn get_available_input_devices(&self) -> Vec<(String, usize)> {
  //   let Ok(midi_inp) = MidiInput::new("anupars-query") else {
  //     return Vec::new();
  //   };
  //   midi_inp
  //     .ports()
  //     .iter()
  //     .enumerate()
  //     .map(|(i, p)| {
  //       let name = midi_inp
  //         .port_name(p)
  //         .unwrap_or_else(|_| format!("Port {}", i));
  //       (name, i)
  //     })
  //     .collect()
  // }

  /// Register a callback that is invoked (from the run thread) for every
  /// incoming external MIDI clock byte (0xF8/0xFA/0xFB/0xFC).
  /// to forward clock events to the metronome without a circular
  /// module dependency between `midi` and `metronome`.
  pub fn set_ext_clock_handler(&self, cb: impl Fn(u8) + Send + 'static) {
    *self.ext_clock_callback.lock().unwrap() = Some(Box::new(cb));
  }

  pub fn switch_device(&self, port_index: usize) -> Result<(), Box<dyn Error>> {
    // Close existing connection
    let mut out_device = self.out_device.lock().unwrap();
    *out_device = None;
    drop(out_device);

    // Create new connection
    let new_midi_out = MidiOutput::new("MIDI Output")?;
    let new_ports = new_midi_out.ports();
    let new_port = new_ports.get(port_index).ok_or("Port not found")?;

    let port_name = new_midi_out.port_name(new_port)?;
    let conn_out = new_midi_out.connect(new_port, "midir-connection")?;

    *self.out_device.lock().unwrap() = Some(conn_out);
    *self.out_device_name.lock().unwrap() = Some(port_name.clone());

    Ok(())
  }

  pub fn out_device_name(&self) -> String {
    let out_device_name = self.out_device_name.lock().unwrap();
    out_device_name.clone().unwrap()
  }

  fn clear_msg_config_list(&self) {
    let mut midi_msg_config_list = self.msg_config_list.lock().unwrap();
    midi_msg_config_list.clear();
  }

  fn set_msg_config_list(&self, midi: MidiMsg) {
    let mut midi_msg_config_list = self.msg_config_list.lock().unwrap();
    midi_msg_config_list.push(midi);
  }

  /// Calculate velocity based on position and mode
  fn calculate_velocity(params: &TriggerParams) -> u8 {
    let ref_velocity = MAX_VELOCITY
      - (params.active_pos_y as f32 / params.grid_height as f32) * (MAX_VELOCITY - MIN_VELOCITY);

    let vel = if params.is_sweep {
      // Sweep mode: scale velocity by distance from active position
      let distance = (params.trigger_pos_y as i32 - params.active_pos_y as i32).abs() as f32;
      let max_distance = params.grid_height as f32;
      let proximity_ratio = (1.0 - (distance / max_distance)).max(0.0);
      (ref_velocity * proximity_ratio).round() as u8 / 2
    } else {
      ref_velocity.round() as u8
    };

    vel.max(MIN_VELOCITY as u8)
  }

  /// Calculate note length based on BPM, distance, DynLength mode, and DIV setting.
  ///
  /// At high DIV values (32, 64) steps fire very rapidly, so the note must release
  /// before the next step arrives.  The `div` parameter (ratio.1) is used to derive
  /// the maximum allowed frames:
  ///   step_frames = 30 000 / (div * bpm)   [each frame = 8 ms]
  /// The result is clamped to 85 % of that window so there is always a gap between
  /// note-off and the next note-on.
  fn calculate_note_length(bpm: usize, distance_to_next: usize, div: usize) -> u8 {
    let base_bpm = consts::DEFAULT_TEMPO;

    // Maximum frames the note may last before the next step fires.
    // step_frames = (64/div * tick_ms) / 8ms  where tick_ms = 60000/(bpm*16)
    //             = 30000 / (div * bpm)
    // Apply 85 % fill so the note-off arrives before the next note-on.
    let step_max_frames: u8 = if div > 0 && bpm > 0 {
      let full_step = 30_000usize / (div.max(1) * bpm.max(1));
      (full_step * 85 / 100).clamp(1, 127) as u8
    } else {
      127
    };

    if distance_to_next == DYNLENGTH_DEFAULT_DISTANCE {
      // DynLength OFF: fixed shorter duration to prevent overlap in fast playback
      let calculated_length = if bpm > 0 {
        ((BASE_LENGTH_FIXED * base_bpm) / bpm).max(1)
      } else {
        BASE_LENGTH_FIXED
      };
      (calculated_length as u8).min(step_max_frames)
    } else {
      // DynLength ON: dynamic duration based on distance to next trigger
      let calculated_length = if bpm > 0 {
        ((BASE_LENGTH_DYNAMIC * base_bpm) / bpm).max(1)
      } else {
        BASE_LENGTH_DYNAMIC
      };

      // Distance factor: 1→0.25x (staccato), 4→1.0x (neutral), 16→4.0x (sustained)
      let distance_factor = (distance_to_next.min(16) as f32 / 4.0).clamp(0.25, 4.0);
      let length_with_distance = (calculated_length as f32 * distance_factor).round() as usize;

      let min_len = MIN_AUDIBLE_LENGTH.min(step_max_frames);
      (length_with_distance as u8).clamp(min_len, step_max_frames)
    }
  }

  /// Trigger MIDI note with position and scale information
  fn trigger_w_position(&self, params: TriggerParams) {
    if params.grid_height == 0 {
      return;
    }

    let (note_index, octave) = params.scale_mode.pos_to_scale_note(
      params.y_position,
      params.grid_height,
      consts::BASE_OCTAVE,
      params.scale_root_offset,
    );

    let velocity = Self::calculate_velocity(&params);
    let note_length = Self::calculate_note_length(params.bpm, params.distance_to_next, params.div);

    // Derive MIDI channel from grid position when splits are active
    let channel: u8 = {
      let v = params.grid_v_splits.max(1);
      let h = params.grid_h_splits.max(1);
      let col_w = if v > 1 && params.grid_width > 0 {
        (params.grid_width / v).max(1)
      } else {
        params.grid_width.max(1)
      };
      let row_h = if h > 1 && params.grid_height > 0 {
        (params.grid_height / h).max(1)
      } else {
        params.grid_height.max(1)
      };
      let col_idx = (params.x_position / col_w).min(v.saturating_sub(1));
      let row_idx = (params.trigger_pos_y / row_h).min(h.saturating_sub(1));
      (row_idx * v + col_idx) as u8
    };

    let midi_msg = MidiMsg::from(note_index, octave, note_length, velocity, channel);

    let note_key = (
      midi_msg.note.round() as u8,
      midi_msg.octave,
      midi_msg.channel,
    );
    let mut last_held = self.last_held_note.lock().unwrap();

    if params.hold {
      let prev_key = last_held
        .as_ref()
        .map(|p| (p.note.round() as u8, p.octave, p.channel));

      match prev_key {
        Some(k) if k == note_key => {
          return;
        }
        Some(_) => {
          let prev = last_held.take().unwrap();
          drop(last_held);
          self.tx.send(Message::Release(prev)).unwrap();
          last_held = self.last_held_note.lock().unwrap();
        }
        None => {}
      }

      let _ = self.trigger(&midi_msg, true);
      *last_held = Some(midi_msg.clone());
      drop(last_held);
      self.tx.send(Message::Hold(midi_msg)).unwrap();
    } else {
      if let Some(prev) = last_held.take() {
        drop(last_held);
        self.tx.send(Message::Release(prev)).unwrap();
        last_held = self.last_held_note.lock().unwrap();
      }
      *last_held = None;
      drop(last_held);
      let _ = self.trigger(&midi_msg, true);
      self.tx.send(Message::Release(midi_msg.clone())).unwrap();
      self.tx.send(Message::Push(midi_msg)).unwrap();
    }
  }

  fn build_midi_msg(&self, midi_msg: &MidiMsg, down: bool) -> [u8; 3] {
    let note_event = if down {
      u8::from(MidiStatusMsg::NoteOn) + midi_msg.channel
    } else {
      u8::from(MidiStatusMsg::NoteOff) + midi_msg.channel
    };

    [
      note_event,
      convert_to_midi_note_num(midi_msg.octave, midi_msg.note).0,
      midi_msg.velocity,
    ]
  }

  fn build_pitch_bend_msg(&self, midi_msg: &MidiMsg) -> Option<[u8; 3]> {
    let (_, pitch_bend) = convert_to_midi_note_num(midi_msg.octave, midi_msg.note);
    if pitch_bend.abs() < 0.01 {
      return None; // No pitch bend needed
    }

    // MIDI pitch bend: 14-bit value, center = 8192, range typically ±2 semitones
    // pitch_bend is in cents, convert to 14-bit MIDI value
    let bend_range_semitones = 2.0; // Standard pitch bend range
    let bend_ratio = pitch_bend / (bend_range_semitones * 100.0);
    let bend_value = (8192.0 + (bend_ratio * 8192.0)).clamp(0.0, 16383.0) as u16;

    let lsb = (bend_value & MidiStatusMsg::BitMask7 as u16) as u8;
    let msb = ((bend_value >> 7) & MidiStatusMsg::BitMask7 as u16) as u8;

    Some([
      u8::from(MidiStatusMsg::PitchBend) + midi_msg.channel,
      lsb,
      msb,
    ])
  }

  pub fn trigger(&self, midi_msg: &MidiMsg, down: bool) -> Result<(), &str> {
    match self.out_device.lock() {
      Ok(mut conn_out) => {
        let Some(connection_out) = conn_out.as_mut() else {
          return Ok(());
        };

        // Send pitch bend first if needed (only on note-on)
        if down {
          if let Some(pitch_bend_msg) = self.build_pitch_bend_msg(midi_msg) {
            connection_out.send(&pitch_bend_msg).unwrap();
          }
        }

        // Send note-on or note-off
        let built_msg = self.build_midi_msg(midi_msg, down);
        connection_out.send(&built_msg).unwrap();
        Ok(())
      }
      _ => Err("send_midi_note_out::error"),
    }
  }

  fn send_all_notes_off(&self) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        for channel in 0..16 {
          let all_notes_off = [
            u8::from(MidiStatusMsg::ControlChange) + channel,
            u8::from(MidiStatusMsg::AllNotesOff),
            0,
          ];
          let _ = connection_out.send(&all_notes_off);
          let all_sound_off = [
            u8::from(MidiStatusMsg::ControlChange) + channel,
            u8::from(MidiStatusMsg::AllSoundOff),
            0,
          ];
          let _ = connection_out.send(&all_sound_off);
        }
      }
    }
  }
  fn send_clock_start(&self) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        let _ = connection_out.send(&[u8::from(MidiStatusMsg::Start)]);
      }
    }
  }

  fn send_clock_stop(&self) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        let _ = connection_out.send(&[u8::from(MidiStatusMsg::Stop)]);
      }
    }
  }

  fn send_clock_tick(&self) {
    if *self.clock_enabled.lock().unwrap() {
      if let Ok(mut conn_out) = self.out_device.lock() {
        if let Some(connection_out) = conn_out.as_mut() {
          let _ = connection_out.send(&[u8::from(MidiStatusMsg::Clock)]);
        }
      }
    }
  }

  fn send_clock_continue(&self) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        let _ = connection_out.send(&[u8::from(MidiStatusMsg::Continue)]);
      }
    }
  }

  fn send_song_position_pointer(&self, position_in_ticks: usize) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        // MIDI SPP uses "beats" where 1 MIDI beat = 6 MIDI clocks
        // Internal: 4 ticks = 1 quarter note = 24 clocks = 4 MIDI beats
        // So: 1 tick = 1 MIDI beat (1:1 ratio)
        let spp_beats = position_in_ticks;

        // SPP is 14-bit value sent as two 7-bit bytes
        let lsb = (spp_beats & MidiStatusMsg::BitMask7 as usize) as u8;
        let msb = ((spp_beats >> 7) & MidiStatusMsg::BitMask7 as usize) as u8;

        let _ = connection_out.send(&[u8::from(MidiStatusMsg::SongPositionPointer), lsb, msb]);
      }
    }
  }

  pub fn enable_clock(&self, enabled: bool) {
    let mut clock_enabled = self.clock_enabled.lock().unwrap();
    *clock_enabled = enabled;
  }
}

/// Convert octave and note (with fractional semitones) to MIDI note number and pitch bend in cents
/// Returns (midi_note, pitch_bend_cents)
pub fn convert_to_midi_note_num(octave: u8, note: f32) -> (u8, f32) {
  let base_note = 24 + (octave * 12);
  let total_note = base_note as f32 + note;
  let midi_note = total_note.round() as u8;
  let pitch_bend_cents = (total_note - midi_note as f32) * 100.0;

  (midi_note, pitch_bend_cents)
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for Midi {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(not(target_arch = "wasm32"))]
impl Midi {
  pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
    let midi_out = MidiOutput::new("MIDI Output")?;

    let out_ports = midi_out.ports();
    let out_port: &MidiOutputPort = match out_ports.len() {
      0 => return Err("no output port found".into()),
      1 => {
        println!(
          "Choosing the only available output port: {}",
          midi_out.port_name(&out_ports[0]).unwrap()
        );
        &out_ports[0]
      }
      _ => {
        println!("\nAvailable output ports:");
        for (i, p) in out_ports.iter().enumerate() {
          println!("{}: {}", i, midi_out.port_name(p).unwrap());
        }
        let input = String::from("0");
        out_ports
          .get(input.trim().parse::<usize>()?)
          .ok_or("invalid output port selected")?
      }
    };

    let conn_out_name = &midi_out.port_name(out_port).unwrap();
    let conn_out = midi_out.connect(out_port, "midir-test")?;
    self.out_device = Mutex::new(Some(conn_out));
    self.out_device_name = Mutex::new(Some(conn_out_name.to_string()));
    Ok(())
  }

  pub fn new() -> Self {
    let (tx, rx) = channel();
    let tempo = Arc::new(Mutex::new(consts::DEFAULT_TEMPO));
    let Ok(midi_out) = MidiOutput::new("client-midi-output") else {
      return Self {
        midi: None.into(),
        out_device: None.into(),
        out_device_name: None.into(),
        tx,
        rx,
        msg_config_list: Arc::new(Mutex::new(Vec::new())),
        tempo,
        clock_enabled: Arc::new(Mutex::new(false)),
        last_held_note: Arc::new(Mutex::new(None)),
        in_connection: Mutex::new(None),
        clock_input_enabled: Arc::new(AtomicBool::new(false)),
        ext_clock_callback: Mutex::new(None),
      };
    };
    Midi {
      midi: Some(midi_out).into(),
      out_device: None.into(),
      out_device_name: None.into(),
      tx,
      rx,
      msg_config_list: Arc::new(Mutex::new(Vec::new())),
      tempo,
      clock_enabled: Arc::new(Mutex::new(false)),
      last_held_note: Arc::new(Mutex::new(None)),
      in_connection: Mutex::new(None),
      clock_input_enabled: Arc::new(AtomicBool::new(false)),
      ext_clock_callback: Mutex::new(None),
    }
  }

  pub fn run(self) {
    let stack = Arc::new(Stack::new());
    let stack_tx = Arc::clone(&stack).run(self.tx.clone());
    Arc::clone(&stack).refresh(self.tx.clone());

    thread::Builder::new()
      .name(crate::core::consts::THREAD_NAME_MIDI_PROCESSOR.to_string())
      .spawn(move || {
        for control_message in &self.rx {
          match control_message {
            Message::Push(midi_msg) => {
              self.handle_push(stack_tx.clone(), midi_msg);
            }
            Message::Hold(midi_msg) => {
              self.handle_hold(stack_tx.clone(), midi_msg);
            }
            Message::Release(midi_msg) => {
              self.handle_release(stack_tx.clone(), midi_msg);
            }
            Message::ReleaseAll() => {
              self.handle_release_all(stack_tx.clone());
            }
            Message::Trigger(msg, is_pressed) => {
              self.handle_trigger(msg, is_pressed);
            }
            Message::SetMsgConfig(msg) => {
              self.set_msg_config_list(msg);
            }
            Message::ClearMsgConfig() => {
              self.clear_msg_config_list();
            }
            Message::TriggerWithPosition(params) => {
              self.handle_trigger_with_position(params);
            }
            Message::ControlChange {
              channel,
              cc_number,
              cc_value,
            } => {
              self.handle_control_change(channel, cc_number, cc_value);
            }
            Message::SetTempo(bpm) => {
              self.handle_set_tempo(bpm);
            }
            Message::SwitchDevice(port_index) => {
              self.handle_switch_device(port_index);
            }
            Message::DisconnectOutput() => {
              self.handle_disconnect_output();
            }
            Message::Panic() => {
              self.handle_panic();
            }
            Message::ClockStart() => {
              self.handle_clock_start();
            }
            Message::ClockStop() => {
              self.handle_clock_stop();
            }
            Message::ClockTick() => {
              self.handle_clock_tick();
            }
            Message::ClockContinue() => {
              self.handle_clock_continue();
            }
            Message::ClockSongPosition(position) => {
              self.handle_clock_song_position(position);
            }
            Message::EnableClock(enabled) => {
              self.enable_clock(enabled);
            }
            Message::ExternalClock(byte) => {
              self.handle_external_clock(byte);
            }
            Message::EnableClockInput(en) => {
              self.handle_enable_clock_input(en);
            }
            Message::ConnectInput(port_index) => {
              self.handle_connect_input(port_index);
            }
          }
        }
      })
      .expect("Failed to spawn midi-processor thread");
  }
}

// ── WASM stub implementation ──────────────────────────────────────────────────
#[cfg(target_arch = "wasm32")]
impl Midi {
  pub fn new() -> Self {
    let (tx, rx) = channel();
    Self {
      tx,
      rx,
      msg_config_list: Arc::new(Mutex::new(Vec::new())),
      clock_input_enabled: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn init(&mut self) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
  }

  pub fn run(self) {
    // No-op in WASM - MIDI output is silenced.
  }

  pub fn get_available_devices(&self) -> Vec<(String, usize)> {
    vec![]
  }

  pub fn out_device_name(&self) -> String {
    "none (WASM)".to_string()
  }

  pub fn enable_clock(&self, _enabled: bool) {}

  pub fn set_ext_clock_handler(&self, _cb: impl Fn(u8) + Send + 'static) {}

  pub fn trigger(&self, _midi_msg: &MidiMsg, _down: bool) -> Result<(), &str> {
    Ok(())
  }
}
