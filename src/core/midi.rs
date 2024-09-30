use midir::{MidiIO, MidiOutput, MidiOutputConnection, MidiOutputPort};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Mutex;

pub struct Midi {
  pub midi: Mutex<Option<MidiOutput>>,
  pub devices: Mutex<HashMap<String, String>>,
  pub out_device: Mutex<Option<MidiOutputConnection>>,
}

impl Default for Midi {
  fn default() -> Self {
    let Ok(midi_out) = MidiOutput::new("client-midi-output") else {
      return Self {
        midi: None.into(),
        devices: HashMap::new().into(),
        out_device: None.into(),
      };
    };
    Midi {
      midi: Some(midi_out).into(),
      devices: HashMap::new().into(),
      out_device: None.into(),
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
        print!("Please select output port: ");
        // stdout().flush()?;
        let mut input = String::from("0");
        // stdin().read_line(&mut input)?;
        out_ports
          .get(input.trim().parse::<usize>()?)
          .ok_or("invalid output port selected")?
      }
    };

    // println!("\nOpening connection");
    let mut conn_out = midi_out.connect(out_port, "midir-test")?;
    self.out_device = Mutex::new(Some(conn_out));
    // println!("Connection open. Listen!");
    // {
    //   // Define a new scope in which the closure `play_note` borrows conn_out, so it can be called easily
    //   let mut play_note = |note: u8, duration: u64| {
    //     const NOTE_ON_MSG: u8 = 0x90;
    //     const NOTE_OFF_MSG: u8 = 0x80;
    //     const VELOCITY: u8 = 0x64;
    //     let _ = conn_out.send(&[NOTE_ON_MSG, note, VELOCITY]);
    //   };

    //   play_note(66, 4);
    //   play_note(65, 3);
    //   play_note(63, 1);
    //   play_note(61, 6);
    //   play_note(59, 2);
    //   play_note(58, 4);
    //   play_note(56, 4);
    //   play_note(54, 4);
    // }
    // // This is optional, the connection would automatically be closed as soon as it goes out of scope
    // conn_out.close();
    Ok(())
  }
}
