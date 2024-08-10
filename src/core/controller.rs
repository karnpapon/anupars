use std::{error::Error, sync::mpsc};

use cursive::Cursive;

use super::ui::{Message, Ui};

pub struct Controller {
  rx: mpsc::Receiver<Message>,
  ui: Ui,
}

impl Controller {
  pub fn new() -> Result<Controller, Box<dyn Error>> {
    let s = Cursive::new();
    let (tx, rx) = mpsc::channel::<Message>();
    let ui = Ui::new(s, tx.clone());

    Ok(Controller { rx, ui })
  }

  pub fn init(&mut self) {
    self.ui.init()
  }

  pub fn run(&mut self) {
    while self.ui.run() {
      while let Some(message) = self.next_message() {
        match message {
          Message::Sync => {}
          Message::Quit => {
            self.ui.quit();
          }
        };
      }
    }
  }

  fn next_message(&self) -> Option<Message> {
    self.rx.try_iter().next()
  }
}
