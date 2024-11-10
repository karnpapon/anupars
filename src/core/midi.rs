use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::{BTreeSet, HashMap};
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use super::stack::{self, Stack};

#[derive(Clone, Debug)]
pub enum Message {
  Push(MidiMsg),
  Trigger(MidiMsg, bool),
  SetMsgConfig(MidiMsg),
  ClearMsgConfig(),
  TriggerWithRegexPos((usize, Arc<Mutex<BTreeSet<usize>>>)),
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
}

impl Midi {
  pub fn new() -> Self {
    let (tx, rx) = channel();

    let Ok(midi_out) = MidiOutput::new("client-midi-output") else {
      return Self {
        midi: None.into(),
        devices: HashMap::new().into(),
        out_device: None.into(),
        out_device_name: None.into(),
        tx,
        rx,
        msg_config_list: Arc::new(Mutex::new(Vec::new())),
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
    }
  }
}

impl Midi {
  pub fn init(&mut self) -> Result<(), Box<dyn Error>> {
    let midi_out = MidiOutput::new("My Test Output")?;

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
          Message::TriggerWithRegexPos(msg) => {
            self.trigger_w_regex_pos(msg.0, msg.1);
          }
        }
      }
    });
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

  fn trigger_w_regex_pos(
    &self,
    curr_running_marker: usize,
    regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  ) {
    let regex_indexes = regex_indexes.lock().unwrap();
    let triggered_index = regex_indexes
      .iter()
      .position(|v| v == &curr_running_marker)
      .unwrap_or(0); //TODO: properly handle moving marker while is_playing=true
    let midi_msg_config_list = self.msg_config_list.lock().unwrap();

    if midi_msg_config_list.len() > 0 {
      let _ = self.trigger(
        &midi_msg_config_list[triggered_index % midi_msg_config_list.len()],
        true,
      );
      self
        .tx
        .send(Message::Push(
          midi_msg_config_list[triggered_index % midi_msg_config_list.len()].clone(),
        ))
        .unwrap();
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
}

pub fn convert_to_midi_note_num(octave: u8, note: u8) -> u8 {
  24 + (octave * 12) + note // 60 = C3
}
