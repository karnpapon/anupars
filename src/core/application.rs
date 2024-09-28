use std::rc::Rc;

use super::commands::CommandManager;

pub type UserData = Rc<UserDataInner>;
pub struct UserDataInner {
  pub cmd: CommandManager,
}
