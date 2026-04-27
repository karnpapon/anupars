use crate::core::consts;
use crate::core::playhead::movement::Movement;
use crate::core::playhead::tilt::TiltMode;
use crate::core::playhead::{PlayheadUI, UIUpdate};
use crate::core::utils;
use crate::view::line_editor::LineEditor;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
  Grid,
  #[default]
  RegexInput,
  Menu,
}

/// Regex flag toggles shown in the console panel.
#[derive(Debug, Clone, Copy)]
pub struct FlagState {
  pub case_sensitive: bool,
  pub multiline: bool,
}

impl Default for FlagState {
  fn default() -> Self {
    Self {
      case_sensitive: true,
      multiline: false,
    }
  }
}

impl FlagState {
  /// Return the flag string that the regex engine expects.
  pub fn to_flag_str(self) -> &'static str {
    if self.multiline {
      "m"
    } else {
      "i"
    }
  }
}

/// Top-level menu identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuId {
  App,
  View,
  Help,
}

/// State for the custom-drawn menu bar.
#[derive(Default)]
pub struct MenuState {
  /// Whether the menu bar has keyboard focus and a dropdown is open.
  pub visible: bool,
  /// Which top-level menu is open (dropdown visible).
  pub active_menu: Option<MenuId>,
  /// Row index of the highlighted dropdown item (includes delimiter rows).
  pub active_item: usize,
  /// Index of the focused tab when no dropdown is open.
  pub focused_tab: usize,
  /// Item index of the currently open submenu (if any).
  pub open_submenu: Option<usize>,
  /// Selected row within the open submenu.
  pub submenu_item: usize,
  /// MIDI output device names, updated at startup and on device change.
  pub midi_output_devices: Vec<String>,
  /// MIDI input device names.
  pub midi_input_devices: Vec<String>,
}

/// Authoritative application state, owned by the main event loop.
/// Background threads write into this via UIUpdate messages.
pub struct AppState {
  /// All playhead-driven fields that the grid renderer reads.
  pub playhead_ui: PlayheadUI,

  /// Regex input editor - single-line editable field.
  pub line_editor: LineEditor,

  /// Regex flag toggles (case-sensitive, multiline).
  pub flags: FlagState,

  /// Menu bar state.
  pub menu: MenuState,

  /// Status bar text labels, one field per labeled slot.
  pub bpm_display: String,
  pub ratio_status: String,
  pub pos_status: String,
  pub len_status: String,
  pub chn_status: String,
  pub input_status: String,
  pub mode_status: String,
  pub movement_status: String,
  pub tilt_status: String,
  pub regex_match_count: String,
  pub regex_error: String,
  pub midi_status: String,

  /// Console / SymSpell display state.
  pub buf_status: String,
  pub sym_status: String,
  pub rpl_status: String,
  pub regex_input: String,

  /// Welcome / doc text shown in the display panel.
  pub display_text: String,

  /// Terminal dimensions, updated on Resize events.
  pub width: u16,
  pub height: u16,

  /// Keyboard focus.
  pub focus: Focus,

  /// Whether the menubar is visible (toggled with Ctrl+b, default hidden).
  pub show_menubar: bool,

  /// Whether the About dialog overlay is visible.
  pub show_about: bool,

  /// Whether the Docs "coming soon" dialog is visible.
  pub show_docs: bool,
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      playhead_ui: PlayheadUI::default(),
      line_editor: LineEditor::new(),
      flags: FlagState::default(),
      menu: MenuState::default(),
      bpm_display: utils::build_bpm_status_str(consts::DEFAULT_TEMPO),
      ratio_status: utils::build_ratio_status_str(consts::DEFAULT_RATIO),
      pos_status: "-".to_string(),
      len_status: "-".to_string(),
      chn_status: "-".to_string(),
      input_status: "-".to_string(),
      // all modes inactive at start - same format as build_mode_status_string() with no active modes
      mode_status: "anuesyzo".to_string(),
      movement_status: Movement::Forward.print_movements(),
      tilt_status: TiltMode::Vertical.print_tilts(),
      regex_match_count: "0".to_string(),
      regex_error: String::new(),
      midi_status: String::new(),
      buf_status: String::new(),
      sym_status: String::new(),
      rpl_status: String::new(),
      regex_input: String::new(),
      display_text: String::new(),
      width: 0,
      height: 0,
      focus: Focus::RegexInput,
      show_menubar: false,
      show_about: false,
      show_docs: false,
    }
  }
}

impl AppState {
  pub fn resize(&mut self, w: u16, h: u16) {
    self.width = w;
    self.height = h;
  }
}

/// Apply one UIUpdate directly to AppState
pub fn apply_ui_update(update: UIUpdate, state: &mut AppState) {
  match update {
    UIUpdate::ActivePos(pos) => {
      state.playhead_ui.actived_pos = pos;
    }
    UIUpdate::AccumulationCounter(count, total) => {
      state.input_status = format!("@ {}/{}", count, total);
    }
    UIUpdate::PlayheadPosAndArea(pos, area) => {
      state.playhead_ui.playhead_pos = pos;
      state.playhead_ui.playhead_area = area;
      state.pos_status = utils::build_pos_status_str(pos);
      let size = area.size();
      state.len_status = utils::build_len_status_str((size.x, size.y));
    }
    UIUpdate::ChnStatus(s) => state.chn_status = s,
    UIUpdate::GridSplits(v, h) => {
      state.playhead_ui.grid_v_splits = v;
      state.playhead_ui.grid_h_splits = h;
    }
    UIUpdate::AimedArea(aimed_area) => {
      state.playhead_ui.aimed_area = aimed_area;
    }
    UIUpdate::TmpAppendSpace => {
      state.buf_status.push(' ');
    }
    UIUpdate::TmpAppend(idx) => {
      state
        .buf_status
        .push(char::from_u32(idx as u32).unwrap_or('?'));
    }
    UIUpdate::RplCycle(_) => {}
    UIUpdate::SweepX(x) => {
      state.playhead_ui.sweep_x = x;
    }
    UIUpdate::BpmDisplay(s) => state.bpm_display = s,
    UIUpdate::RatioStatus(s) => state.ratio_status = s,
    UIUpdate::PosStatus(s) => state.pos_status = s,
    UIUpdate::LenStatus(s) => state.len_status = s,
    UIUpdate::InputStatus(s) => state.input_status = s,
    UIUpdate::ModeStatus(s) => state.mode_status = s,
    UIUpdate::MovementStatus(s) => state.movement_status = s,
    UIUpdate::TiltStatus(s) => state.tilt_status = s,
    UIUpdate::RegexMatchCount(s) => state.regex_match_count = s,
    UIUpdate::RegexError(s) => state.regex_error = s,
    UIUpdate::TextMatcher {
      matcher,
      regex_indexes,
    } => {
      state.playhead_ui.text_matcher = matcher;
      // Wire the shared Arc so the printer's regex_indexes writes are
      // visible to position_calc.regex_indexes (same underlying Mutex).
      state.playhead_ui.regex_indexes = regex_indexes;
    }
    UIUpdate::QueueManagerUpdate(qm) => {
      state.playhead_ui.queue_manager = qm;
    }
    UIUpdate::CanvasPlayheadPos(pos) => {
      state.playhead_ui.playhead_pos = pos;
    }
    UIUpdate::CanvasPlayheadArea(area) => {
      state.playhead_ui.playhead_area = area;
    }
    UIUpdate::CanvasArpeggiatorMode(v) => state.playhead_ui.arpeggiator_mode = v,
    UIUpdate::CanvasAccumulationMode(v) => state.playhead_ui.accumulation_mode = v,
    UIUpdate::CanvasEventOperatorMode(v) => state.playhead_ui.event_operator_mode = v,
    UIUpdate::CanvasDrainQueueMode(v) => state.playhead_ui.drain_queue_mode = v,
    UIUpdate::CanvasSweepMode(v) => state.playhead_ui.sweep_mode = v,
    UIUpdate::CanvasSweepMovement(mv) => state.playhead_ui.sweep_movement = mv,
    UIUpdate::CanvasSweepOutputMode(m) => state.playhead_ui.sweep_output_mode = m,
    UIUpdate::CanvasSweepCcNumber(n) => state.playhead_ui.sweep_cc_number = n,
    UIUpdate::CanvasSweepRowMode(m) => state.playhead_ui.sweep_row_mode = m,
    UIUpdate::CanvasTiltMode(t) => state.playhead_ui.tilt_mode = t,
    UIUpdate::CanvasFreezeMode(v) => state.playhead_ui.freeze_mode = v,
    UIUpdate::CanvasDroneMode(is_drone, drone_x) => {
      state.playhead_ui.drone_mode = is_drone;
      state.playhead_ui.drone_x = drone_x;
    }
    UIUpdate::CanvasDroneX(x) => state.playhead_ui.drone_x = x,
    UIUpdate::CanvasDroneChannel(ch) => state.playhead_ui.drone_channel = ch,
    UIUpdate::CanvasScaleRootTop(r) => state.playhead_ui.scale_root_top = r,
    UIUpdate::CanvasScaleRootLeft(r) => state.playhead_ui.scale_root_left = r,
    UIUpdate::CanvasScaleModeTop(m) => state.playhead_ui.scale_mode_top = m,
    UIUpdate::CanvasScaleModeLeft(m) => state.playhead_ui.scale_mode_left = m,
    UIUpdate::CanvasKeyboardTopActive(v) => state.playhead_ui.keyboard_top_active = v,
  }
}
