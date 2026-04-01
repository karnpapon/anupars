use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, Mutex};

use cursive::Vec2;
use cursive::XY;

use crate::core::command::types;
use crate::core::consts;
use crate::core::engine::regex::Match;
use crate::core::tonal::scale;
use crate::view::rect::Rect;

use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

use super::movement::Movement;
use super::queue::QueueManager;
use super::tilt::TiltMode;

/// Sweep row filter mode: which rows the vertical crosshair draws on and triggers MIDI from.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum SweepRowMode {
  #[default]
  Normal,
  Odd,
  Even,
  Random(u64),
}

impl SweepRowMode {
  pub fn cycle_next(self) -> Self {
    match self {
      SweepRowMode::Normal => SweepRowMode::Odd,
      SweepRowMode::Odd => SweepRowMode::Even,
      SweepRowMode::Even => SweepRowMode::Random(fastrand::u64(..)),
      SweepRowMode::Random(_) => SweepRowMode::Normal,
    }
  }

  pub fn label(&self) -> &'static str {
    match self {
      SweepRowMode::Normal => "",
      SweepRowMode::Odd => "O",
      SweepRowMode::Even => "E",
      SweepRowMode::Random(_) => "R",
    }
  }

  /// Returns true if the given row `y` is active under this mode.
  /// For Random mode the seed (stored in the variant) is mixed into the hash
  /// so a fresh seed → a fresh pattern of active rows.
  pub fn is_row_active(self, y: usize) -> bool {
    match self {
      SweepRowMode::Normal => true,
      SweepRowMode::Odd => y % 2 == 1,
      SweepRowMode::Even => y.is_multiple_of(2),
      SweepRowMode::Random(seed) => {
        let mut hasher = DefaultHasher::new();
        y.hash(&mut hasher);
        seed.hash(&mut hasher);
        hasher.finish().is_multiple_of(2)
      }
    }
  }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Direction {
  Up,
  Down,
  Left,
  Right,
  Idle,
}

impl Direction {
  pub fn get_direction(&self) -> (i32, i32) {
    match self {
      Direction::Right => (1, 0),
      Direction::Up => (0, -1),
      Direction::Left => (-1, 0),
      Direction::Down => (0, 1),
      Direction::Idle => (0, 0),
    }
  }
}

#[derive(Clone, Debug)]
pub enum UIUpdate {
  ActivePos(Vec2),
  AccumulationCounter(usize, usize),
  PlayheadPosAndArea(Vec2, Rect),
  ChnStatus(String),
  GridSplits(usize, usize),
  AimedArea(Option<Rect>),
  TmpAppend(usize),
  TmpAppendSpace,
  RplCycle(Rect),
  SweepX(usize),
}

pub struct PlayheadUI {
  pub playhead_area: Rect,
  pub playhead_pos: Vec2,
  pub actived_pos: Vec2,
  pub aimed_area: Option<Rect>,
  pub text_matcher: Option<HashMap<usize, Match>>,
  pub regex_indexes: Arc<Mutex<BTreeSet<usize>>>,
  pub scale_mode_left: scale::ScaleMode,
  pub scale_mode_top: scale::ScaleMode,
  pub scale_root_top: scale::ScaleRoot,
  pub scale_root_left: scale::ScaleRoot,
  pub arpeggiator_mode: bool,
  pub accumulation_mode: bool,
  pub event_operator_mode: bool,
  pub drain_queue_mode: bool,
  pub sweep_mode: bool,
  pub sweep_row_mode: SweepRowMode,
  pub sweep_movement: Option<Movement>,
  pub sweep_x: usize,
  pub drone_mode: bool,
  pub drone_x: usize,
  pub drone_channel: usize,
  pub freeze_mode: bool,
  pub tilt_mode: TiltMode,
  pub queue_manager: Arc<QueueManager>,
  pub grid_v_splits: usize,
  pub grid_h_splits: usize,
  pub focus_mode: bool,
  pub keyboard_top_active: bool,
}

impl PlayheadUI {
  pub fn new() -> Self {
    PlayheadUI {
      playhead_area: Rect::from_point(Vec2::zero()),
      playhead_pos: Vec2::zero(),
      actived_pos: Vec2::zero(),
      aimed_area: None,
      text_matcher: None,
      regex_indexes: Arc::new(Mutex::new(BTreeSet::new())),
      scale_mode_left: scale::ScaleMode::default(),
      scale_mode_top: scale::ScaleMode::default(),
      scale_root_top: scale::ScaleRoot::default(),
      scale_root_left: scale::ScaleRoot::default(),
      arpeggiator_mode: false,
      accumulation_mode: false,
      event_operator_mode: false,
      drain_queue_mode: false,
      sweep_mode: false,
      sweep_row_mode: SweepRowMode::default(),
      sweep_movement: None,
      sweep_x: 0,
      drone_mode: false,
      drone_x: 0,
      drone_channel: 1,
      freeze_mode: false,
      tilt_mode: TiltMode::default(),
      queue_manager: Arc::new(QueueManager::new()),
      grid_v_splits: 1,
      grid_h_splits: 1,
      focus_mode: false,
      keyboard_top_active: true,
    }
  }
}

#[derive(Clone, Debug)]
pub enum Message {
  Move(Direction, XY<usize>),
  Leap(Direction, XY<usize>),
  SetCurrentPos(XY<usize>, XY<usize>),
  UpdateInfoStatusView(),
  SetGridArea(XY<usize>),
  SetActivePos(usize),
  Scale((i32, i32)),
  SetMatcher(Option<HashMap<usize, Match>>),
  SetGridSize(usize, usize),
  SetScaleModeLeft(scale::ScaleMode),
  SetScaleModeTop(scale::ScaleMode),
  SetScaleRootTop(scale::ScaleRoot),
  ToggleAccumulationMode(),
  ToggleForwardMode(),
  ToggleReverseMode(),
  TogglePendulumMode(),
  ToggleArpeggiatorMode(),
  ToggleRandomMode(),
  ToggleEventOperatorMode(),
  ToggleDrainQueueMode(),
  ToggleSweepMode(),
  ToggleDroneMode(),
  MoveDrone(Direction),
  CycleDroneChannel(types::Adjustment),
  ToggleDynLengthMode(),
  ToggleFreezeMode(),
  CycleScaleRootTop(types::Adjustment),
  CycleScaleMode(types::Adjustment),
  CycleScaleRootLeft(types::Adjustment),
  CycleScaleModeLeft(types::Adjustment),
  SetTempo(usize),
  SetRatio((usize, usize)),
  ClearQueue(),
  SetGridSplits(usize, usize),
  CycleTiltMode(),
  CycleSweepRowMode(),
  StartAim(),
  UpdateAim(Direction, XY<usize>, usize),
  CommitAim(),
  CancelAim(),
  ToggleSpatialKeyboard(),
  ToggleSweepMovementMode(),
  SetSweepMovement(Movement),
}

/// Grid and layout state shared across playhead subsystems.
#[derive(Clone)]
pub struct GridState {
  pub width: Arc<AtomicUsize>,
  pub height: Arc<AtomicUsize>,
  pub v_splits: Arc<AtomicUsize>,
  pub h_splits: Arc<AtomicUsize>,
}

impl GridState {
  pub fn new() -> Self {
    GridState {
      width: Arc::new(AtomicUsize::new(0)),
      height: Arc::new(AtomicUsize::new(0)),
      v_splits: Arc::new(AtomicUsize::new(1)),
      h_splits: Arc::new(AtomicUsize::new(1)),
    }
  }
}

/// Musical parameters and scale configuration shared across playhead subsystems.
#[derive(Clone)]
pub struct MusicState {
  pub tempo: Arc<AtomicUsize>,
  pub scale_mode_left: Arc<Mutex<scale::ScaleMode>>,
  pub scale_mode_top: Arc<Mutex<scale::ScaleMode>>,
  pub scale_root_top: Arc<Mutex<scale::ScaleRoot>>,
  pub scale_root_left: Arc<Mutex<scale::ScaleRoot>>,
  pub ratio: Arc<Mutex<(usize, usize)>>,
}

impl MusicState {
  pub fn new() -> Self {
    MusicState {
      tempo: Arc::new(AtomicUsize::new(consts::DEFAULT_TEMPO)),
      scale_mode_left: Arc::new(Mutex::new(scale::ScaleMode::default())),
      scale_mode_top: Arc::new(Mutex::new(scale::ScaleMode::default())),
      scale_root_top: Arc::new(Mutex::new(scale::ScaleRoot::default())),
      scale_root_left: Arc::new(Mutex::new(scale::ScaleRoot::default())),
      ratio: Arc::new(Mutex::new(consts::DEFAULT_RATIO)),
    }
  }
}

/// Boolean operation mode flags shared across playhead subsystems.
#[derive(Clone)]
pub struct ModeFlags {
  pub accumulation_mode: Arc<AtomicBool>,
  pub arpeggiator_mode: Arc<AtomicBool>,
  pub event_operator_mode: Arc<AtomicBool>,
  pub sweep_mode: Arc<AtomicBool>,
  pub sweep_movement: Arc<Mutex<Option<Movement>>>,
  pub sweep_x: Arc<AtomicUsize>,
  pub drone_mode: Arc<AtomicBool>,
  pub drone_x: Arc<AtomicUsize>,
  /// MIDI channel for drone line (1-indexed, 1-16). Channel 1 is the default.
  pub drone_channel: Arc<AtomicUsize>,
  pub dyn_length_mode: Arc<AtomicBool>,
  pub freeze_mode: Arc<AtomicBool>,
  pub hold_next_note: Arc<AtomicBool>,
  pub is_ratcheting: Arc<AtomicBool>,
  /// When true the top keyboard (scale_mode_top / scale_root_top) is active;
  /// when false the left keyboard (scale_mode_left / scale_root_left) is active.
  pub keyboard_top_active: Arc<AtomicBool>,
}

impl ModeFlags {
  pub fn new() -> Self {
    ModeFlags {
      accumulation_mode: Arc::new(AtomicBool::new(false)),
      arpeggiator_mode: Arc::new(AtomicBool::new(false)),
      event_operator_mode: Arc::new(AtomicBool::new(false)),
      sweep_mode: Arc::new(AtomicBool::new(false)),
      sweep_movement: Arc::new(Mutex::new(None)),
      sweep_x: Arc::new(AtomicUsize::new(0)),
      drone_mode: Arc::new(AtomicBool::new(false)),
      drone_x: Arc::new(AtomicUsize::new(0)),
      drone_channel: Arc::new(AtomicUsize::new(1)),
      dyn_length_mode: Arc::new(AtomicBool::new(false)),
      freeze_mode: Arc::new(AtomicBool::new(false)),
      hold_next_note: Arc::new(AtomicBool::new(false)),
      is_ratcheting: Arc::new(AtomicBool::new(false)),
      keyboard_top_active: Arc::new(AtomicBool::new(true)),
    }
  }
}
