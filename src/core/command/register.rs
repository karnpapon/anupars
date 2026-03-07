use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::app::UserData;
use crate::view::common::playhead;

#[cfg(feature = "desktop")]
use crate::view::desktop::program::Program;

#[cfg(feature = "microcontroller")]
use crate::view::microcontroller::program::Program;

use super::types::Command;
use crate::core::command::binding;
use crate::core::command::types::Adjustment;
use crate::core::timing::metronome;
use crate::core::{consts, utils};

use cursive::event::Event;
use cursive::views::{LinearLayout, TextView};
use cursive::Cursive;
use log::error;
use std::cell::RefCell;

pub struct CommandManager {
  aliases: HashMap<String, String>,
  bindings: RefCell<HashMap<String, Vec<Command>>>,
  pub program: Arc<Program>,
  metronome_sender: Sender<metronome::Message>,
  cb_sink: cursive::CbSink,
  temp_tempo: Arc<Mutex<usize>>,
  temp_ratio: Arc<Mutex<(usize, usize)>>,
  pub last_key_time: Arc<Mutex<Option<Instant>>>,
  playhead_tx_cloned: Sender<playhead::Message>,
}

impl CommandManager {
  pub fn new(
    prog: Program,
    m_tx: Sender<metronome::Message>,
    cb_sink: cursive::CbSink,
    temp_tempo: Arc<Mutex<usize>>,
    last_key_time: Arc<Mutex<Option<Instant>>>,
    playhead_tx_cloned: Sender<playhead::Message>,
  ) -> Self {
    let bindings = RefCell::new(binding::get_bindings());
    Self {
      aliases: HashMap::new(), // ? maybe obsolete
      bindings,
      program: Arc::new(prog),
      metronome_sender: m_tx,
      cb_sink,
      temp_tempo,
      temp_ratio: Arc::new(Mutex::new(consts::DEFAULT_RATIO)),
      last_key_time,
      playhead_tx_cloned,
    }
  }

  pub fn register_aliases<S: Into<String>>(&mut self, name: S, aliases: Vec<S>) {
    let name = name.into();
    for a in aliases {
      self.aliases.insert(a.into(), name.clone());
    }
  }

  pub fn register_all(&mut self) {
    self.register_aliases("quit", vec!["q", "x"]);
    self.register_aliases("playpause", vec!["pause", "toggleplay", "toggleplayback"]);
    self.register_aliases("showmenubar", vec!["showmenubar"]);
  }

  fn handle_default_commands(
    &self,
    s: &mut Cursive,
    cmd: &Command,
  ) -> Result<Option<String>, String> {
    match cmd {
      Command::Quit => Ok(None),
      Command::TogglePlay => {
        let _ = self.metronome_sender.send(metronome::Message::StartStop);
        Ok(None)
      }
      Command::ShowMenubar => {
        s.select_menubar();
        Ok(None)
      }
      Command::ToggleInputRegexAndCanvas => {
        self.program.set_toggle_regex_input();

        if !self.program.toggle_regex_input() {
          let mut display_view = s.find_name::<TextView>(consts::display_view).unwrap();
          display_view.set_content(utils::build_doc_string(&consts::APP_WELCOME_MSG));

          let mut interactive_display_section_view = s
            .find_name::<LinearLayout>(consts::main_section_view)
            .unwrap();
          let _ = interactive_display_section_view.set_focus_index(2); // microcontroller=2, desktop=3
        } else {
          let mut interactive_display_section_view = s
            .find_name::<LinearLayout>(consts::main_section_view)
            .unwrap();
          let _ = interactive_display_section_view.set_focus_index(0);
        }

        Ok(None)
      }
      Command::AdjustBPM(direction) => {
        let nudge: isize = match direction {
          Adjustment::Increase => 1,
          Adjustment::Decrease => -1,
        };

        let mut last_press = self.last_key_time.lock().unwrap();
        *last_press = Some(Instant::now());

        let mut tempo = self.temp_tempo.lock().unwrap();
        *tempo = ((*tempo as isize) + nudge) as usize;
        let temp = *tempo;

        self
          .playhead_tx_cloned
          .send(playhead::Message::SetTempo(temp))
          .unwrap();

        self
          .cb_sink
          .send(Box::new(move |s| {
            s.call_on_name(consts::bpm_status_unit_view, |view: &mut TextView| {
              view.set_content(utils::build_bpm_status_str(temp));
            })
            .unwrap();
          }))
          .unwrap();

        Ok(None)
      }
      Command::AdjustRatio(direction) => {
        let ratios = [1, 2, 4, 8, 16];

        let current_ratio = *self.temp_ratio.lock().unwrap();
        let current_denom = current_ratio.1;

        let current_idx = ratios.iter().position(|&d| d == current_denom).unwrap_or(4); // Default to 16 if not found

        let new_idx = match direction {
          Adjustment::Increase => {
            if current_idx < ratios.len() - 1 {
              current_idx + 1
            } else {
              current_idx
            }
          }
          Adjustment::Decrease => {
            if current_idx > 0 {
              current_idx - 1
            } else {
              current_idx
            }
          }
        };

        let new_ratio = (1, ratios[new_idx]);

        let mut ratio = self.temp_ratio.lock().unwrap();
        *ratio = new_ratio;
        drop(ratio);

        self
          .playhead_tx_cloned
          .send(playhead::Message::SetRatio(new_ratio))
          .unwrap();

        self
          .metronome_sender
          .send(metronome::Message::Reset)
          .unwrap();
        Ok(None)
      }
      Command::ToggleForward => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleForwardMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleReverse => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleReverseMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleArpeggiator => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleArpeggiatorMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleAccumulation => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleAccumulationMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleRandom => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleRandomMode())
          .unwrap();
        Ok(None)
      }
      Command::TogglePendulum => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::TogglePendulumMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleEventOperator => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleEventOperatorMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleDrainQueue => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleDrainQueueMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleSweep => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleSweepMode())
          .unwrap();
        Ok(None)
      }
      Command::ToggleDynLength => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleDynLengthMode())
          .unwrap();
        Ok(None)
      }
      Command::ChangeRootNote(dir) => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::CycleScaleRootTop(*dir))
          .unwrap();
        Ok(None)
      }
      Command::ChangeScaleMode(dir) => {
        self
          .playhead_tx_cloned
          .send(playhead::Message::CycleScaleMode(*dir))
          .unwrap();
        Ok(None)
      }
    }
  }

  fn handle_callbacks(&self, s: &mut Cursive, cmd: &Command) -> Result<Option<String>, String> {
    self.handle_default_commands(s, cmd)
  }

  pub fn handle(&self, siv: &mut Cursive, cmd: Command) {
    let _result = self.handle_callbacks(siv, &cmd);
    siv.on_event(Event::Refresh);
  }

  pub fn register_keybinding<E: Into<cursive::event::Event>>(
    &self,
    cursive: &mut Cursive,
    event: E,
    commands: Vec<Command>,
  ) {
    cursive.add_global_callback(event, move |s| {
      let cb_sink = s.cb_sink().clone();
      let cmd_cloned = commands.clone();
      cb_sink
        .send(Box::new(move |inner_s: &mut Cursive| {
          if let Some(data) = inner_s.user_data::<UserData>().cloned() {
            for command in cmd_cloned.into_iter() {
              data.cmd.handle(inner_s, command);
            }
          };
        }))
        .unwrap();
    });
  }

  pub fn unregister_keybindings(&self, cursive: &mut Cursive) {
    let kb = self.bindings.borrow();

    for (k, _v) in kb.iter() {
      if let Some(binding) = binding::parse_keybinding(k) {
        cursive.clear_global_callbacks(binding);
      }
    }
  }

  pub fn register_keybindings(&self, cursive: &mut Cursive) {
    let kb = self.bindings.borrow();

    for (k, v) in kb.iter() {
      if let Some(binding) = binding::parse_keybinding(k) {
        self.register_keybinding(cursive, binding, v.clone());
      } else {
        error!("Could not parse keybinding: \"{}\"", k);
      }
    }
  }
}
