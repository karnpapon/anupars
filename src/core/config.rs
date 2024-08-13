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
  pub static ref MENU_MIDI: StaticStrInt = Vec::from([("Default", 9000)]);
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
    ("(CmdOrCtrl-i)", "toggle input"),
    ("(CmdOrCtrl-g)", "toggle Regex input"),
    ("(Return)", "eval input(target input must = ON)"),
    ("(h)", "toggle helps window"),
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
