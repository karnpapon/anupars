use std::{rc::Rc, sync::mpsc::Sender};

use super::{commands::CommandManager, midi};

pub type UserData = Rc<UserDataInner>;
pub struct UserDataInner {
  pub cmd: CommandManager,
  pub midi_tx: Sender<midi::Message>,
}
