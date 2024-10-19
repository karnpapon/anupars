use midir::{MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use super::stack::{self, Stack};

#[derive(Clone, Debug)]
pub enum Message {
  Push(MidiMsg),
  Trigger(MidiMsg, bool),
  MsgConfig(String),
}

#[derive(Clone, Debug)]
pub struct MidiMsg {
  note: String,
  velocity: i8,
  octave: i8,
  channel: i8,
  pub length: i8,
  pub is_played: bool,
}

impl MidiMsg {
  pub fn from(
    note: String,
    velocity: i8,
    octave: i8,
    channel: i8,
    length: i8,
    is_played: bool,
  ) -> MidiMsg {
    Self {
      note,
      velocity,
      octave,
      channel,
      length,
      is_played,
    }
  }
}

pub struct Midi {
  pub midi: Mutex<Option<MidiOutput>>,
  pub devices: Mutex<HashMap<String, String>>,
  pub out_device: Mutex<Option<MidiOutputConnection>>,
  pub out_device_name: Mutex<Option<String>>,
  // pub stack: Arc<Mutex<Vec<MidiMsg>>>,
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
        // stack: Arc::new(Mutex::new(vec![])),
        tx,
        rx,
      };
    };
    Midi {
      midi: Some(midi_out).into(),
      devices: HashMap::new().into(),
      out_device: None.into(),
      out_device_name: None.into(),
      // stack: Arc::new(Mutex::new(vec![])),
      tx,
      rx,
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
            self.trigger(&msg, is_pressed);
          }
          Message::MsgConfig(msg) => {
            // self.trigger(&msg, is_pressed);
          }
        }
      }
    });
  }

  pub fn out_device_name(&self) -> String {
    let out_device_name = self.out_device_name.lock().unwrap();
    out_device_name.clone().unwrap()
  }

  pub fn trigger(&self, item: &MidiMsg, down: bool) {
    let play_note = |note: u8, duration: u64| {
      let note_event = if down { 0x90 } else { 0x80 };
      const VELOCITY: u8 = 0x64;
      match self.out_device.lock() {
        Ok(mut conn_out) => {
          let connection_out = conn_out.as_mut().unwrap();
          connection_out.send(&[note_event, note, VELOCITY]).unwrap();
          Ok(())
        }
        _ => Err("send_midi_note_out::error"),
      }
    };
    play_note(66, 4).unwrap();
  }
}
