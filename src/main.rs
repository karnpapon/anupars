mod core;

use core::{config::EXIT_FAILURE, controller::Controller};
use std::process;

fn main() {
  let c = Controller::new();
  match c {
    Ok(mut controller) => controller.run(),
    Err(e) => {
      println!("Error: {}", e);
      process::exit(EXIT_FAILURE);
    }
  }
}
