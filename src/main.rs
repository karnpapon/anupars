mod core;

use core::anu::Anu;
use core::controller::ControllerData;

use cursive::view::Nameable;
use cursive::{Cursive, CursiveExt};

fn main() {
  let mut anu = Anu::new().with_name("anu");
  let mut siv: Cursive = Cursive::new();
  siv.set_autorefresh(true);
  siv.set_user_data(ControllerData::default());

  anu.get_mut().init_default_style(&mut siv);
  anu.get_mut().build_menubar(&mut siv);

  let main_views = anu.get_mut().build_root_view(&mut siv);

  siv.add_layer(main_views);

  siv.run();
}
