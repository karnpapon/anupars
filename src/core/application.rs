use std::{rc::Rc, sync::mpsc::Sender};

use super::{commands::CommandManager, midi};

#[cfg(feature = "microcontroller")]
use crate::view::microcontroller::console::RegexFlag;
#[cfg(feature = "microcontroller")]
use std::sync::{Arc, RwLock};

pub type UserData = Rc<UserDataInner>;
pub struct UserDataInner {
  pub cmd: CommandManager,
  pub midi_tx: Sender<midi::Message>,
  #[cfg(feature = "microcontroller")]
  pub selected_flag: Arc<RwLock<RegexFlag>>,
}
