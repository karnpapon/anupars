mod core;
mod view;

use core::application::UserDataInner;
use core::clock::metronome::{Message, Metronome};
use core::commands::CommandManager;
use core::config;
use core::midi::Midi;
use core::regex::RegExpHandler;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cursive::theme::{BorderStyle, Palette};
use cursive::views::TextView;
use cursive::{Cursive, CursiveExt};
use num::rational::Ratio;
use num::FromPrimitive;
use std::rc::Rc;
use std::thread;
use view::common::marker::Marker;
use view::common::menubar::Menubar;
// use view::desktop::anu::Anu;
use view::microcontroller::anu::Anu;

pub fn init_default_style(siv: &mut Cursive) {
  let my_palette = Palette::terminal_default();
  siv.set_theme(cursive::theme::Theme {
    shadow: false,
    borders: BorderStyle::Simple,
    palette: my_palette,
  });
}

fn main() {
  let mut siv: Cursive = Cursive::new();
  init_default_style(&mut siv);

  let menu_app = Menubar::build_menu_app();
  let menu_help = Menubar::build_menu_help();
  let mut midi = Midi::new();
  midi.init().unwrap();
  let midi_tx = midi.tx.clone();
  let regex = RegExpHandler::new(siv.cb_sink().clone());
  let last_key_time = Arc::new(Mutex::new(None));
  let last_key_time_clone = Arc::clone(&last_key_time);
  let mut anu: Anu = Anu::new();
  let marker = Marker::new(siv.cb_sink().clone(), midi_tx.clone());
  let marker_tx_cloned = marker.tx.clone();
  let marker_tx = marker.tx.clone();
  let marker_tx_2 = marker.tx.clone();
  let metronome = Metronome::new(siv.cb_sink().clone(), marker_tx_2);
  let m_tx = metronome.tx.clone();
  let m_tx_2 = metronome.tx.clone();
  let temp_tempo = Arc::new(Mutex::new(120));
  let temp_tempo_cloned = Arc::clone(&temp_tempo);
  let mut cmd_manager = CommandManager::new(
    anu.clone(),
    m_tx,
    siv.cb_sink().clone(),
    temp_tempo_cloned,
    last_key_time.clone(),
    marker_tx_cloned,
  );

  cmd_manager.register_all();
  cmd_manager.register_keybindings(&mut siv);

  siv.set_autohide_menu(true);
  siv.set_autorefresh(false); // "false" to prevent unintended events (eg. midi keep pushing into stack, etc.)
  siv.set_user_data(Rc::new(UserDataInner {
    cmd: cmd_manager,
    midi_tx: midi_tx.clone(),
  }));

  let main_views = anu.build(regex.tx.clone(), midi_tx, marker_tx);

  siv
    .menubar()
    .add_subtree("Anu", menu_app)
    .add_subtree("Help", menu_help)
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.add_layer(main_views);

  siv
    .call_on_name(config::midi_status_unit_view, |c: &mut TextView| {
      c.set_content(midi.out_device_name());
    })
    .unwrap();

  thread::spawn(move || loop {
    thread::sleep(Duration::from_millis(100));
    let mut last_press = last_key_time_clone.lock().unwrap();
    if let Some(last_time) = *last_press {
      if last_time.elapsed() > Duration::from_millis(500) {
        *last_press = None;
        let tempo = *temp_tempo.lock().unwrap();
        let _ = m_tx_2.send(Message::Tempo(Ratio::from_i64(tempo).unwrap()));
      }
    }
  });

  thread::spawn(move || {
    regex.run();
  });
  thread::spawn(move || {
    metronome.run();
  });
  marker.run();
  midi.run();
  siv.run();
}
