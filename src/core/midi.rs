use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::stack::{self, Stack};
use super::utils::Throttler;
use crate::core::consts::BASE_OCTAVE;

#[derive(Clone, Debug)]
pub enum Message {
  Push(MidiMsg),
  Trigger(MidiMsg, bool),
  SetMsgConfig(MidiMsg), // ? maybe obsolete, TBD
  ClearMsgConfig(),
  TriggerWithPosition(
    (
      usize,
      usize,
      usize,
      usize,
      crate::core::scale::ScaleMode,
      usize,
    ),
  ), // (grid_index, y_position, grid_width, grid_height, scale_mode, bpm)
  SwitchDevice(usize),
  Panic(),
  SetTempo(usize),
}

#[derive(Clone, Debug)]
pub struct MidiMsg {
  note: u8,
  velocity: u8,
  octave: u8,
  channel: u8,
  pub length: u8,
  pub is_played: bool,
}

impl MidiMsg {
  pub fn from(
    note: u8,
    octave: u8,
    length: u8,
    velocity: u8,
    channel: u8,
    is_played: bool,
  ) -> MidiMsg {
    Self {
      note,
      octave,
      length,
      velocity,
      channel,
      is_played,
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
  throttler: Arc<Mutex<Throttler>>,
  tempo: Arc<Mutex<usize>>,
}

impl Midi {
  pub fn new() -> Self {
    let (tx, rx) = channel();
    let throttler = Arc::new(Mutex::new(Throttler::new(Duration::from_millis(100))));
    let tempo = Arc::new(Mutex::new(120));
    let Ok(midi_out) = MidiOutput::new("client-midi-output") else {
      return Self {
        midi: None.into(),
        devices: HashMap::new().into(),
        out_device: None.into(),
        out_device_name: None.into(),
        tx,
        rx,
        msg_config_list: Arc::new(Mutex::new(Vec::new())),
        throttler,
        tempo,
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
      throttler,
      tempo,
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
        // print!("Please select output port: ");
        // stdout().flush()?;
        let input = String::from("0");
        // stdin().read_line(&mut input)?;
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
          Message::Trigger(msg, is_pressed) => {
            self.trigger(&msg, is_pressed).unwrap();
          }
          Message::SetMsgConfig(msg) => {
            self.set_msg_config_list(msg);
          }
          Message::ClearMsgConfig() => {
            self.clear_msg_config_list();
          }
          Message::TriggerWithPosition((
            grid_index,
            y_position,
            grid_width,
            grid_height,
            scale_mode,
            bpm,
          )) => {
            self.trigger_w_position(
              grid_index,
              y_position,
              grid_width,
              grid_height,
              scale_mode,
              bpm,
            );
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

  fn trigger_w_position(
    &self,
    _grid_index: usize,
    y_position: usize,
    _grid_width: usize,
    grid_height: usize,
    scale_mode: crate::core::scale::ScaleMode,
    bpm: usize,
  ) {
    // Use the actual grid height passed as parameter
    if grid_height == 0 {
      return; // Avoid division by zero
    }

    // Use scale mode to map position to note
    let (note_index, octave) = scale_mode.y_to_scale_note(y_position, grid_height, BASE_OCTAVE);

    // Calculate dynamic note length based on BPM
    // Higher BPM = shorter notes, minimum length is 1
    // Formula: length = max(1, base_length * (base_bpm / current_bpm))
    let base_bpm = 120;
    let base_length = 4;
    let calculated_length = if bpm > 0 {
      ((base_length * base_bpm) / bpm).max(1)
    } else {
      base_length
    };
    let note_length = (calculated_length as u8).min(127);
    let midi_msg = MidiMsg::from(note_index, octave, note_length, 100, 0, false);
    let _ = self.trigger(&midi_msg, true);
    self.tx.send(Message::Push(midi_msg)).unwrap();
  }

  fn build_midi_msg(&self, midi_msg: &MidiMsg, down: bool) -> [u8; 3] {
    let note_event = if down {
      0x90 + midi_msg.channel
    } else {
      0x80 + midi_msg.channel
    };

    [
      note_event,
      convert_to_midi_note_num(midi_msg.octave, midi_msg.note),
      midi_msg.velocity,
    ]
  }

  pub fn trigger(&self, midi_msg: &MidiMsg, down: bool) -> Result<(), &str> {
    let built_msg = self.build_midi_msg(midi_msg, down);
    match self.out_device.lock() {
      Ok(mut conn_out) => {
        let connection_out = conn_out.as_mut().unwrap();
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
}

pub fn convert_to_midi_note_num(octave: u8, note: u8) -> u8 {
  24 + (octave * 12) + note // 60 = C3
}
