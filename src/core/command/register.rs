use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::app_state::{AppState, Focus};
use crate::core::playhead;
use crate::core::playhead::UIUpdate;

use crate::view::grid::GridEditor;
use crate::view::layout::Program;

use super::types::Command;
use crate::core::command::binding;
use crate::core::command::types::Adjustment;
use crate::core::timing::metronome;
use crate::core::{consts, utils};

use log::error;
use std::cell::RefCell;

pub struct CommandManager {
  aliases: HashMap<String, String>,
  bindings: RefCell<HashMap<String, Vec<Command>>>,
  pub program: Arc<Program>,
  metronome_sender: Sender<metronome::Message>,
  ui_tx: Sender<UIUpdate>,
  temp_tempo: Arc<Mutex<usize>>,
  temp_ratio: Arc<Mutex<(usize, usize)>>,
  pub last_key_time: Arc<Mutex<Option<Instant>>>,
  playhead_tx_cloned: Sender<playhead::Message>,
}

impl CommandManager {
  pub fn new(
    prog: Program,
    m_tx: Sender<metronome::Message>,
    ui_tx: Sender<UIUpdate>,
    temp_tempo: Arc<Mutex<usize>>,
    last_key_time: Arc<Mutex<Option<Instant>>>,
    playhead_tx_cloned: Sender<playhead::Message>,
  ) -> Self {
    let bindings = RefCell::new(binding::get_bindings());
    Self {
      aliases: HashMap::new(),
      bindings,
      program: Arc::new(prog),
      metronome_sender: m_tx,
      ui_tx,
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

  fn execute_command(
    &self,
    cmd: &Command,
    state: &mut AppState,
    grid: &mut GridEditor,
    should_quit: &mut bool,
  ) {
    match cmd {
      Command::Quit => {
        *should_quit = true;
      }
      Command::TogglePlay => {
        if consts::EXT_CLOCK_ACTIVE.load(Ordering::Relaxed) {
          return;
        }
        let _ = self.metronome_sender.send(metronome::Message::StartStop);
      }
      Command::ShowMenubar => {
        if state.focus == Focus::Menu {
          state.focus = Focus::Grid;
          state.menu.active_menu = None;
        } else {
          state.focus = Focus::Menu;
        }
      }
      Command::ToggleInputRegexAndCanvas => {
        self.program.set_toggle_regex_input();
        let is_canvas = !self.program.toggle_regex_input();
        if is_canvas {
          state.focus = Focus::Grid;
          grid.is_canvas_focused = true;
        } else {
          state.focus = Focus::RegexInput;
          grid.is_canvas_focused = false;
        }
      }
      Command::AdjustBPM(direction) => {
        if consts::EXT_CLOCK_ACTIVE.load(Ordering::Relaxed) {
          return;
        }
        let nudge: isize = match direction {
          Adjustment::Increase => 1,
          Adjustment::Decrease => -1,
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
          let mut last_press = self.last_key_time.lock().unwrap();
          *last_press = Some(Instant::now());
        }

        let mut tempo = self.temp_tempo.lock().unwrap();
        *tempo = ((*tempo as isize) + nudge) as usize;
        let temp = *tempo;

        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::SetTempo(temp));

        let _ = self
          .ui_tx
          .send(UIUpdate::BpmDisplay(utils::build_bpm_status_str(temp)));
      }
      Command::AdjustRatio(direction) => {
        let ratios = [1, 2, 4, 8, 16, 32, 64];

        let current_ratio = *self.temp_ratio.lock().unwrap();
        let current_denom = current_ratio.1;

        let current_idx = ratios.iter().position(|&d| d == current_denom).unwrap_or(4);

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
        *self.temp_ratio.lock().unwrap() = new_ratio;

        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::SetRatio(new_ratio));
        let _ = self.metronome_sender.send(metronome::Message::Reset);
      }
      Command::ToggleForward => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleForwardMode());
      }
      Command::ToggleReverse => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleReverseMode());
      }
      Command::ToggleArpeggiator => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleArpeggiatorMode());
      }
      Command::ToggleAccumulation => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleAccumulationMode());
      }
      Command::ToggleRandom => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleRandomMode());
      }
      Command::TogglePendulum => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::TogglePendulumMode());
      }
      Command::ToggleEventOperator => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleEventOperatorMode());
      }
      Command::ToggleDrainQueue => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleDrainQueueMode());
      }
      Command::ToggleSweep => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleSweepMode());
      }
      Command::ToggleDrone => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleDroneMode());
      }
      Command::MoveDroneLeft => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::MoveDrone(playhead::Direction::Left));
      }
      Command::MoveDroneRight => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::MoveDrone(playhead::Direction::Right));
      }
      Command::CycleDroneChannel(adj) => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleDroneChannel(*adj));
      }
      Command::ToggleFreeze => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleFreezeMode());
      }
      Command::CycleTilt => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleTiltMode());
      }
      Command::ToggleDynLength => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleDynLengthMode());
      }
      Command::ChangeRootNote(dir) => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleScaleRootTop(*dir));
      }
      Command::ChangeScaleMode(dir) => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleScaleMode(*dir));
      }
      Command::ChangeScaleModeLeft(dir) => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleScaleModeLeft(*dir));
      }
      Command::ChangeRootNoteLeft(dir) => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleScaleRootLeft(*dir));
      }
      Command::CycleSweepMode => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleSweepRowMode());
      }
      Command::ToggleSweepMovement => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleSweepMovementMode());
      }
      Command::CycleSweepOutputMode => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::CycleSweepOutputMode());
      }
      Command::AdjustSweepCC(adj) => {
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::AdjustSweepCC(*adj));
      }
      Command::ToggleSpatialKeyboard => {
        grid.playhead_ui.keyboard_top_active = !grid.playhead_ui.keyboard_top_active;
        let _ = self
          .playhead_tx_cloned
          .send(playhead::Message::ToggleSpatialKeyboard());
      }
    }
  }

  /// Return a reference to the stored playhead sender.
  pub fn playhead_tx(&self) -> &Sender<playhead::Message> {
    &self.playhead_tx_cloned
  }

  /// Look up `key` in the binding table and execute any matching commands.
  /// Accepts the same key string format used in `binding.rs` (e.g. "Space", "Ctrl+f").
  /// Returns true if at least one command was executed.
  #[cfg(target_arch = "wasm32")]
  pub fn dispatch_command_str(
    &self,
    key: &str,
    state: &mut AppState,
    grid: &mut GridEditor,
    should_quit: &mut bool,
  ) -> bool {
    let kb = self.bindings.borrow();
    if let Some(commands) = kb.get(key) {
      let commands = commands.clone();
      drop(kb);
      for cmd in &commands {
        self.execute_command(cmd, state, grid, should_quit);
      }
      return true;
    }
    false
  }

  /// Dispatch a crossterm key event. Returns true if the key was consumed.
  #[cfg(not(target_arch = "wasm32"))]
  pub fn dispatch_key(
    &self,
    key: crossterm::event::KeyEvent,
    state: &mut AppState,
    grid: &mut GridEditor,
    should_quit: &mut bool,
  ) -> bool {
    use crossterm::event::KeyEventKind;
    if key.kind == KeyEventKind::Release {
      return false;
    }

    use crossterm::event::{KeyCode, KeyModifiers};

    // For printable chars the SHIFT modifier is already encoded in the character
    // itself ('+' vs '=', '>' vs '.', etc.), so strip it before comparing to
    // avoid mismatches between terminals that include SHIFT and those that don't.
    let norm = |code: &KeyCode, mods: KeyModifiers| -> KeyModifiers {
      if matches!(code, KeyCode::Char(_)) {
        mods & !KeyModifiers::SHIFT
      } else {
        mods
      }
    };
    let key_eff = norm(&key.code, key.modifiers);

    let kb = self.bindings.borrow();
    for (k, commands) in kb.iter() {
      if let Some(bound) = binding::parse_keybinding(k) {
        if bound.code == key.code && norm(&bound.code, bound.modifiers) == key_eff {
          for cmd in commands {
            self.execute_command(cmd, state, grid, should_quit);
          }
          return true;
        }
      } else {
        error!("Could not parse keybinding: \"{}\"", k);
      }
    }
    false
  }
}
