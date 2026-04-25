use crossterm::event::{KeyEvent, MouseEvent};

pub enum AppEvent {
  Key(KeyEvent),
  Mouse(MouseEvent),
  Resize(u16, u16),
  /// A background thread signals that new UIUpdates are pending.
  UIFlush,
  Quit,
}
