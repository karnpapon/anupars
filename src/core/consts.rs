#![allow(non_upper_case_globals)]

use lazy_static::lazy_static;

pub type StaticStrInt = Vec<(&'static str, i32)>;
pub type StaticStrStr = Vec<(&'static str, &'static str)>;

lazy_static! {
  pub static ref MENU_OSC: StaticStrInt = Vec::from([
    ("Default", 9000),
    ("SuperCollider", 57120),
    ("TidalCycles", 6010),
    ("SonicPi", 4559),
  ]);
  pub static ref APP_DOCS: StaticStrStr = Vec::from([
    ("n", "add new marker"),
    ("f", "focus only marker(s)"),
    ("r", "[*] reverse step"),
    ("e", "[*] rename marker"),
    ("o", "[*] set osc msg"),
    ("m", "[*] set midi msg"),
    ("c", "[*] ratcheting"),
    ("x", "[*] mute"),
    ("'", "[*] replace marker block"),
    ("> | <", "incr/decr BPM "),
    ("[ | ]", "[*] incr/decr note-ratio (default 1/16)"),
    ("{ | }", "[*] incr/decr note-ratio for ratcheting"),
    ("?", "[*] show control informations"),
    (";", "toggle mono-step mode"),
    ("Backspace", "[*] remove current marker"),
    ("Spacebar", "play/pause"),
    ("Cmd-Arrow", "[*] jump"),
    ("Cmd-(1..6)", "toggle regex flag respectively"),
    ("Cmd-/", "switch regex mode"),
    ("Option-Tab", "change selected markers"),
    ("Shift-Arrow", "[*] incr/decr marker range"),
    ("Shift-Arrow-Cmd", "[*] jump incr/decr marker range"),
  ]);
  pub static ref APP_WELCOME_MSG: StaticStrStr = Vec::from([
    // ("", ""),
    // ("(Return)", "eval input"),
    ("(Ctrl-h)", "show menubar"),
  ]);
}

pub static APP_NAME: &str = "
░░▒▓██████▓▒░░▒▓███████▓▒░░▒▓█▓▒░░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ 
░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ 
░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░░▒▓█▓▒░ 
░░▒▓██████▓▒░░▒▓█▓▒░░▒▓█▓▒░░░░▒▓██▓▒░░░░  
";

pub static DEFAULT_APP_DIRECTORY: &str = ".anu_rs";
pub static DEFAULT_APP_FILENAME: &str = "contents";

// workaround since `format!` cannot be calculated at build-time (eg. for `static` or `const`)
// https://users.rust-lang.org/t/how-to-avoid-recalculating-a-formatted-string-at-runtime/44895
pub fn app_empty_dir() -> &'static str {
  lazy_static! {
    static ref value: String = format!(
      "empty directory!\n\nto getting start, try adding any text file format (eg .txt, .md, .rtf) to {:?}\n",
      dirs::home_dir()
        .map(|p| p.join(DEFAULT_APP_DIRECTORY).join(DEFAULT_APP_FILENAME))
        .unwrap(),
    );
  }
  &value
}

// *** ID NAMING SPECIFICATIONS ***
// IT SHOULD BE COMPRISED WITH THESE FOLLOWING 3 PARTS
// format: <IDENTIFIER>_<CATEGORY>_view
// where:
//      CATEGORY = "unit" | "section"
//      IDENTIFIER = short and concise meaningful words
pub static regex_input_unit_view: &str = "regex_input_unit_view";
pub static input_status_unit_view: &str = "input_status_unit_view";
pub static bpm_status_unit_view: &str = "bpm_status_unit_view";
pub static ratio_status_unit_view: &str = "ratio_status_unit_view";
pub static len_status_unit_view: &str = "len_status_unit_view";
pub static pos_status_unit_view: &str = "pos_status_unit_view";
pub static osc_status_unit_view: &str = "osc_status_unit_view";
pub static midi_status_unit_view: &str = "midi_status_unit_view";

pub static input_controller_section_view: &str = "input_controller_section_view";
pub static status_controller_section_view: &str = "status_controller_section_view";
pub static protocol_controller_section_view: &str = "protocol_controller_section_view";

pub static regex_display_unit_view: &str = "regex_display_unit_view";
pub static control_section_view: &str = "control_section_view";
pub static interactive_display_section_view: &str = "interactive_display_section_view";

pub static canvas_base_section_view: &str = "canvas_base_section_view";
pub static canvas_highlight_section_view: &str = "canvas_highlight_section_view";
pub static canvas_editor_section_view: &str = "canvas_editor_section_view";

pub static doc_unit_view: &str = "doc_unit_view";
pub static file_explorer_unit_view: &str = "file_explorer_unit_view";
pub static file_contents_unit_view: &str = "file_contents_unit_view";

pub static main_section_view: &str = "main_section_view";

// alias for `regex_display_unit_view`
pub static display_view: &str = "display_view";

// canvas
pub static GRID_ROW_SPACING: usize = 9;
pub static GRID_COL_SPACING: usize = 9;

// Timing constants
pub const TEMPO_CHECK_INTERVAL_MS: u64 = 100;
pub const TEMPO_RESET_DELAY_MS: u64 = 500;
pub const DEFAULT_TEMPO: i64 = 120;

// Keyboard visualization constants
pub const KEYBOARD_MARGIN_TOP: usize = 3;
pub const KEYBOARD_MARGIN_LEFT: usize = 4;
pub const KEYBOARD_MARGIN_BOTTOM: usize = 2;
pub const BASE_OCTAVE: u8 = 2; // Starting octave (C2 = MIDI 48)
pub const NOTE_NAMES: [&str; 12] = [
  "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];
