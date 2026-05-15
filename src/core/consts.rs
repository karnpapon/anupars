#![allow(non_upper_case_globals)]

use std::sync::atomic::{AtomicBool, AtomicU8};

use lazy_static::lazy_static;

type StaticStrInt = Vec<(&'static str, i32)>;
pub type StaticStrStr = Vec<(&'static str, &'static str)>;

lazy_static! {
  pub static ref MENU_OSC: StaticStrInt = Vec::from([
    ("Default", 9000),
    ("SuperCollider", 57120),
    ("TidalCycles", 6010),
    ("SonicPi", 4559),
  ]);
  pub static ref APP_DOCS: StaticStrStr = Vec::from([
    ("n", "add new playhead"),
    ("f", "focus only playhead(s)"),
    ("r", "[*] reverse step"),
    ("e", "[*] rename playhead"),
    ("o", "[*] set osc msg"),
    ("m", "[*] set midi msg"),
    ("c", "[*] ratcheting"),
    ("x", "[*] mute"),
    ("'", "[*] replace playhead block"),
    ("> | <", "incr/decr BPM "),
    ("[ | ]", "[*] incr/decr note-ratio (default 1/16)"),
    ("{ | }", "[*] incr/decr note-ratio for ratcheting"),
    ("?", "[*] show control informations"),
    (";", "toggle mono-step mode"),
    ("Backspace", "[*] remove current playhead"),
    ("Spacebar", "play/pause"),
    ("Cmd-Arrow", "[*] jump"),
    ("Cmd-(1..6)", "toggle regex flag respectively"),
    ("Cmd-/", "switch regex mode"),
    ("Option-Tab", "change selected playheads"),
    ("Shift-Arrow", "[*] incr/decr playhead range"),
    ("Shift-Arrow-Cmd", "[*] jump incr/decr playhead range"),
  ]);
  pub static ref APP_WELCOME_MSG: StaticStrStr = Vec::from([
    // ("", ""),
    // ("(Return)", "eval input"),
    ("(Ctrl-b)", "show menubar"),
  ]);
}

pub static APP_NAME: &str = "
░░▒▓██████▓▒░░▒▓███████▓▒░░▒▓█▓▒░░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ 
░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ 
░░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░░░▒▓██▓▒░░░░  
";

pub static DEFAULT_APP_DIRECTORY: &str = ".anupars";
pub static DEFAULT_APP_FILENAME: &str = "contents";

// workaround since `format!` cannot be calculated at build-time (eg. for `static` or `const`)
// https://users.rust-lang.org/t/how-to-avoid-recalculating-a-formatted-string-at-runtime/44895
pub fn app_empty_dir() -> &'static str {
  #[cfg(not(target_arch = "wasm32"))]
  {
    lazy_static! {
      static ref value: String = format!(
        "empty directory!\n\nto getting start, try adding any text file format (eg .txt, .md, .rtf) to {:?}\n",
        dirs::home_dir()
          .map(|p| p.join(DEFAULT_APP_DIRECTORY).join(DEFAULT_APP_FILENAME))
          .unwrap_or_default(),
      );
    }
    &value
  }
  #[cfg(target_arch = "wasm32")]
  {
    "empty directory!\n\nfile access is not available in the browser.\n"
  }
}

// View identifier constants are now defined in `view::consts`.
// Re-exported here so existing `consts::foo_unit_view` paths continue to compile.
pub use crate::view::consts::{GRID_COL_SPACING, GRID_ROW_SPACING};

// Timing constants
pub const TEMPO_CHECK_INTERVAL_MS: u64 = 100;
pub const TEMPO_RESET_DELAY_MS: u64 = 500;
pub const DEFAULT_TEMPO: usize = 120;

// Keyboard visualization constants
pub const KEYBOARD_MARGIN_TOP: usize = 3;
pub const KEYBOARD_MARGIN_LEFT: usize = 8;
pub const QUEUE_MARGIN_RIGHT: usize = 8;
pub const KEYBOARD_MARGIN_BOTTOM: usize = 3; // Increased to accommodate event operators row
pub const BASE_OCTAVE: u8 = 2; // Starting octave (C2 = MIDI 48)
pub const NOTE_NAMES: [&str; 12] = [
  "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub const QUEUE_OP_SPACING: usize = 2;
pub const EVENT_OP_SPACING: usize = 10;

pub const DEFAULT_RATIO: (usize, usize) = (1, 16); // Default to 1/16 notes

pub const EVENT_QUEUE_CAPACITY: usize = 12;
pub const OP_QUEUE_CAPACITY: usize = 12;

pub static CLOCK_ENABLED: AtomicBool = AtomicBool::new(false);
pub static FOCUS_MODE: AtomicBool = AtomicBool::new(false);
pub static SYNTH_ENABLED: AtomicBool = AtomicBool::new(false);
pub static SYNTH_CLEAR_MSG: AtomicBool = AtomicBool::new(false);
pub static STREAM_CC_MODE: AtomicBool = AtomicBool::new(false);
pub static STREAM_CC_NUMBER: AtomicU8 = AtomicU8::new(74);
pub static EXT_CLOCK_ACTIVE: AtomicBool = AtomicBool::new(false);

pub const QUEUE_PLACEHOLDER_SYMBOL: &str = ":";
pub const PLAYHEAD_CHAR: char = '>';
pub const TRIGGER_CHAR: char = '*';
pub const PLAYHEAD_MATCH_CHAR: char = '@';
pub const MATCH_GROUP_CHAR: char = '}';
pub const SWEEP_MOVEMENT_OPEN: char = '<';
pub const SWEEP_MOVEMENT_CLOSE: char = '>';
pub const MOVEMENT_OPEN: char = '[';
pub const MOVEMENT_CLOSE: char = ']';
pub const STREAM_CC_CHAR: char = '+';

pub const TMP_BUF_SIZE: usize = 6;

pub const MOVE_X_STEP_SIZE: usize = 8;
pub const MOVE_Y_STEP_SIZE: usize = 4;
pub const SCALE_X_STEP_SIZE: usize = 8;
pub const SCALE_Y_STEP_SIZE: usize = 2;

pub const FREQ_DICT_PATH: &str = "data/frequency_dictionary_en_10k.txt";

// permanent threads, live for the entire application lifetime
pub const THREAD_NAME_TEMPO_MONITOR: &str = "tempo-monitor";
pub const THREAD_NAME_REGEX_HANDLER: &str = "regex-handler";
pub const THREAD_NAME_METRONOME: &str = "metronome";
pub const THREAD_NAME_CLOCK: &str = "clock";
pub const THREAD_NAME_PLAYHEAD: &str = "playhead";
pub const THREAD_NAME_MIDI_PROCESSOR: &str = "midi-processor";
pub const THREAD_NAME_STACK_EVENT_LOOP: &str = "stack-event-loop";
pub const THREAD_NAME_STACK_REFRESH: &str = "stack-refresh";
pub const THREAD_NAME_UI_PROCESSOR: &str = "ui-batch-processor";

// temporary threads, spawned once at startup, exit when their task completes
pub const THREAD_NAME_SYMSPELL_LOADER: &str = "symspell-loader";

// temporary threads, spawned per event
pub const THREAD_NAME_RATCHET: &str = "ratchet";
pub const THREAD_NAME_RATCHET_NOTE_OFF: &str = "ratchet-note-off";
pub const THREAD_NAME_CHORD_NOTE_OFF: &str = "chord-note-off";
pub const THREAD_NAME_DRONE_NOTE_OFF: &str = "drone-note-off";

pub(crate) const MANIFESTO_TEXT: &str =
  include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/manifesto.txt"));
pub const INITIALIZER_TEXT: &str =
  include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/texts.txt"));

pub const PLAYHEAD_INIT_AREA_WIDTH: usize = 8;
pub const PLAYHEAD_INIT_AREA_HEIGHT: usize = 1;
