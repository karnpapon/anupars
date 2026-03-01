use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
// use std::time::Duration;

use super::stack::{self, Stack};
use crate::core::consts;

// Constants for note length calculation
const BASE_LENGTH_FIXED: usize = 12; // 96ms at 120 BPM - for DynLength OFF
const BASE_LENGTH_DYNAMIC: usize = 32; // 256ms at 120 BPM - for DynLength ON
const MIN_AUDIBLE_LENGTH: u8 = 10; // 80ms minimum
const DYNLENGTH_DEFAULT_DISTANCE: usize = 4;

// Constants for velocity calculation
const MAX_VELOCITY: f32 = 100.0;
const MIN_VELOCITY: f32 = 10.0;

/// Parameters for MIDI trigger with position information
#[derive(Clone, Copy, Debug)]
pub struct TriggerParams {
  pub y_position: usize,
  pub grid_height: usize,
  pub scale_mode: crate::core::scale::ScaleMode,
  pub scale_root_offset: u8,
  pub bpm: usize,
  pub trigger_pos_y: usize,
  pub active_pos_y: usize,
  pub distance_to_next: usize,
  pub hold: bool,
  pub is_sweep: bool,
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
  SwitchDevice(usize),
  Panic(),
  SetTempo(usize),
  ReleaseAll(),
  // MIDI Clock messages
  ClockStart(),             // 0xFA - Start playback
  ClockStop(),              // 0xFC - Stop playback
  ClockTick(),              // 0xF8 - Timing clock (24 PPQN)
  ClockContinue(),          // 0xFB - Continue from pause
  ClockSongPosition(usize), // 0xF2 - Song Position Pointer (in 16th notes)
  EnableClock(bool),        // Enable/disable MIDI clock output
}

#[derive(Clone, Debug)]
pub struct MidiMsg {
  pub note: f32, // Can be fractional for microtonal scales
  pub velocity: u8,
  pub octave: u8,
  pub channel: u8,
  pub length: u8,
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

pub struct Midi {
  pub midi: Mutex<Option<MidiOutput>>,
  pub devices: Mutex<HashMap<String, String>>,
  pub out_device: Mutex<Option<MidiOutputConnection>>,
  pub out_device_name: Mutex<Option<String>>,
  pub msg_config_list: Arc<Mutex<Vec<MidiMsg>>>,
  pub tx: Sender<Message>,
  pub rx: Receiver<Message>,
  // throttler: Arc<Mutex<Throttler>>,
  tempo: Arc<Mutex<usize>>,
  clock_enabled: Arc<Mutex<bool>>,
}

impl Midi {
  pub fn new() -> Self {
    let (tx, rx) = channel();
    let tempo = Arc::new(Mutex::new(consts::DEFAULT_TEMPO));
    let Ok(midi_out) = MidiOutput::new("client-midi-output") else {
      return Self {
        midi: None.into(),
        devices: HashMap::new().into(),
        out_device: None.into(),
        out_device_name: None.into(),
        tx,
        rx,
        msg_config_list: Arc::new(Mutex::new(Vec::new())),
        tempo,
        clock_enabled: Arc::new(Mutex::new(false)),
      };
    };
    Midi {
      midi: Some(midi_out).into(),
      devices: HashMap::new().into(),
      out_device: None.into(),
      out_device_name: None.into(),
      tx,
      rx,
      msg_config_list: Arc::new(Mutex::new(Vec::new())),
      tempo,
      clock_enabled: Arc::new(Mutex::new(false)),
    }
  }
}

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

  pub fn run(self) {
    let midi_tx_1 = self.tx.clone();
    let midi_tx_2 = self.tx.clone();
    let stack = Arc::new(Stack::new());
    let stack_clone_2 = Arc::clone(&stack);
    let stack_tx = stack.run(midi_tx_1);
    stack_clone_2.refresh(midi_tx_2);

    thread::spawn(move || {
      for control_message in &self.rx {
        match control_message {
          Message::Push(midi_msg) => {
            let _ = stack_tx.send(stack::Message::Push(midi_msg));
          }
          Message::Hold(midi_msg) => {
            let _ = stack_tx.send(stack::Message::Hold(midi_msg));
          }
          Message::Release(midi_msg) => {
            let _ = stack_tx.send(stack::Message::Release(midi_msg));
          }
          Message::ReleaseAll() => {
            let _ = stack_tx.send(stack::Message::ReleaseAll());
          }
          Message::Trigger(msg, is_pressed) => {
            self.trigger(&msg, is_pressed).unwrap();
          }
          Message::SetMsgConfig(msg) => {
            self.set_msg_config_list(msg);
          }
          Message::ClearMsgConfig() => {
            self.clear_msg_config_list();
          }
          Message::TriggerWithPosition(params) => {
            self.trigger_w_position(params);
          }
          Message::SetTempo(bpm) => {
            let mut tempo = self.tempo.lock().unwrap();
            *tempo = bpm;
          }
          Message::SwitchDevice(port_index) => {
            if let Err(e) = self.switch_device(port_index) {
              eprintln!("Error switching MIDI device: {}", e);
            }
          }
          Message::Panic() => {
            self.send_all_notes_off();
          }
          Message::ClockStart() => {
            self.send_clock_start();
          }
          Message::ClockStop() => {
            self.send_clock_stop();
          }
          Message::ClockTick() => {
            self.send_clock_tick();
          }
          Message::ClockContinue() => {
            self.send_clock_continue();
          }
          Message::ClockSongPosition(position) => {
            self.send_song_position_pointer(position);
          }
          Message::EnableClock(enabled) => {
            self.enable_clock(enabled);
          }
        }
      }
    });
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

  /// Calculate note length based on BPM, distance, and DynLength mode
  fn calculate_note_length(bpm: usize, distance_to_next: usize) -> u8 {
    let base_bpm = consts::DEFAULT_TEMPO;

    if distance_to_next == DYNLENGTH_DEFAULT_DISTANCE {
      // DynLength OFF: fixed shorter duration to prevent overlap in fast playback
      let calculated_length = if bpm > 0 {
        ((BASE_LENGTH_FIXED * base_bpm) / bpm).max(1)
      } else {
        BASE_LENGTH_FIXED
      };
      (calculated_length as u8).min(127)
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

      (length_with_distance as u8).clamp(MIN_AUDIBLE_LENGTH, 127)
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
    let note_length = Self::calculate_note_length(params.bpm, params.distance_to_next);
    let midi_msg = MidiMsg::from(note_index, octave, note_length, velocity, 0);

    let _ = self.trigger(&midi_msg, true);

    if params.hold {
      self.tx.send(Message::Hold(midi_msg)).unwrap();
    } else {
      self.tx.send(Message::Release(midi_msg.clone())).unwrap();
      self.tx.send(Message::Push(midi_msg)).unwrap();
    }
  }

  fn build_midi_msg(&self, midi_msg: &MidiMsg, down: bool) -> [u8; 3] {
    let note_event = if down {
      0x90 + midi_msg.channel
    } else {
      0x80 + midi_msg.channel
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

    let lsb = (bend_value & 0x7F) as u8;
    let msb = ((bend_value >> 7) & 0x7F) as u8;

    Some([0xE0 + midi_msg.channel, lsb, msb])
  }

  pub fn trigger(&self, midi_msg: &MidiMsg, down: bool) -> Result<(), &str> {
    match self.out_device.lock() {
      Ok(mut conn_out) => {
        let connection_out = conn_out.as_mut().unwrap();

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
    // Send All Notes Off (CC 123) on all 16 MIDI channels
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        for channel in 0..16 {
          // CC 123: All Notes Off
          let all_notes_off = [0xB0 + channel, 123, 0];
          let _ = connection_out.send(&all_notes_off);
          // CC 120: All Sound Off (for good measure)
          let all_sound_off = [0xB0 + channel, 120, 0];
          let _ = connection_out.send(&all_sound_off);
        }
      }
    }
  }
  fn send_clock_start(&self) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        let _ = connection_out.send(&[0xFA]); // 0xFA = Start
      }
    }
  }

  fn send_clock_stop(&self) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        let _ = connection_out.send(&[0xFC]); // 0xFC = Stop
      }
    }
  }

  fn send_clock_tick(&self) {
    if *self.clock_enabled.lock().unwrap() {
      if let Ok(mut conn_out) = self.out_device.lock() {
        if let Some(connection_out) = conn_out.as_mut() {
          let _ = connection_out.send(&[0xF8]); // 0xF8 = Timing Clock (x24 per quarter note (PPQN))
        }
      }
    }
  }

  fn send_clock_continue(&self) {
    if let Ok(mut conn_out) = self.out_device.lock() {
      if let Some(connection_out) = conn_out.as_mut() {
        let _ = connection_out.send(&[0xFB]); // 0xFB = Continue
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
        let lsb = (spp_beats & 0x7F) as u8;
        let msb = ((spp_beats >> 7) & 0x7F) as u8;

        // 0xF2 = Song Position Pointer
        let _ = connection_out.send(&[0xF2, lsb, msb]);
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
