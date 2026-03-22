use crate::core::io::midi;

use super::Playhead;

pub fn make_playhead() -> Playhead {
  let (tx, _rx) = std::sync::mpsc::channel::<midi::Message>();
  let siv = cursive::Cursive::new();
  let ph = Playhead::new(tx, siv.cb_sink().clone());
  // Forget `siv` so its cb_sink receiver is never dropped. Any code under test
  // that calls `cb_sink.send(...)` would otherwise panic on a closed channel.
  // Leaking one Cursive per test is acceptable in an ephemeral test process.
  std::mem::forget(siv);
  ph
}
