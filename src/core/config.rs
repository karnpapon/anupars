use lazy_static::lazy_static;
use std::collections::HashMap;

lazy_static! {
  pub static ref MENU_OSC: HashMap<&'static str, i32> = HashMap::from([
    ("Default", 9000),
    ("SuperCollider", 57120),
    ("TidalCycles", 6010),
    ("SonicPi", 4559),
  ]);
  pub static ref MENU_MIDI: HashMap<&'static str, i32> = HashMap::from([("Default", 9000),]);
}
