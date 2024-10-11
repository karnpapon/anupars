use midir::{MidiIO, MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Mutex;

pub struct MidiMsg {
  note: String,
  velocity: i8,
  octave: i8,
  channel: i8,
  length: i8,
  is_played: bool,
}

pub struct Midi {
  pub midi: Mutex<Option<MidiOutput>>,
  pub devices: Mutex<HashMap<String, String>>,
  pub out_device: Mutex<Option<MidiOutputConnection>>,
  pub stack: Vec<MidiMsg>,
}

impl Default for Midi {
  fn default() -> Self {
    let Ok(midi_out) = MidiOutput::new("client-midi-output") else {
      return Self {
        midi: None.into(),
        devices: HashMap::new().into(),
        out_device: None.into(),
        stack: vec![],
      };
    };
    Midi {
      midi: Some(midi_out).into(),
      devices: HashMap::new().into(),
      out_device: None.into(),
      stack: vec![],
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

    let conn_out = midi_out.connect(out_port, "midir-test")?;
    self.out_device = Mutex::new(Some(conn_out));
    Ok(())
  }

  pub fn push(&mut self) {
    let item = MidiMsg {
      channel: 0,
      octave: 4,
      note: "C".to_string(),
      velocity: 8,
      length: 8,
      is_played: false,
    };
    // Retrigger duplicates
    // for (const id in this.stack) {
    //   const dup = this.stack[id]
    //   if (dup.channel === channel && dup.octave === octave && dup.note === note) { this.release(item, id) }
    // }
    self.stack.push(item)
  }

  pub fn trigger(&self) {}

  pub fn press(&mut self, item: Option<MidiMsg>) {
    if item.is_some() {
      return;
    }
    // self.trigger(item, true);
    item.unwrap().is_played = true
  }

  pub fn release(&self, item: Option<MidiMsg>) {
    if item.is_some() {
      return;
    }
    self.trigger();
    // delete this.stack[id]
  }

  pub fn send_midi_on(&self) {
    let play_note = |note: u8, duration: u64| {
      const NOTE_ON_MSG: u8 = 0x90;
      const VELOCITY: u8 = 0x64;
      match self.out_device.lock() {
        Ok(mut conn_out) => {
          let connection_out = conn_out.as_mut().unwrap();
          connection_out.send(&[NOTE_ON_MSG, note, VELOCITY]).unwrap();
          Ok(())
        }
        _ => Err("send_midi_note_out::error"),
      }
    };
    play_note(66, 4).unwrap();
  }

  pub fn send_midi_off(&self) {
    let play_note = |note: u8, duration: u64| {
      const NOTE_OFF_MSG: u8 = 0x80;
      const VELOCITY: u8 = 0x64;
      match self.out_device.lock() {
        Ok(mut conn_out) => {
          let connection_out = conn_out.as_mut().unwrap();
          connection_out
            .send(&[NOTE_OFF_MSG, note, VELOCITY])
            .unwrap();
          Ok(())
        }
        _ => Err("send_midi_note_out::error"),
      }
    };
    play_note(66, 4).unwrap();
  }
}
