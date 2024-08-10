mod core;

use core::app;

fn main() {
  let mut s = cursive::default();
  app::start(&mut s);
  s.run();
}
