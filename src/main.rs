mod core;
mod view;

use core::anu::Anu;
use core::clock::metronome::{self, Metronome};
use core::regex::RegExpHandler;
use core::{config, utils};
// use crossbeam_utils::sync::Parker;
use std::borrow::Borrow;
use std::time::{Duration, Instant};
// use crossbeam_utils::thread;
use std::{
  convert::Infallible,
  sync::mpsc::{self, channel, Receiver, Sender, TryRecvError},
  thread::JoinHandle,
};

use cursive_tabs::TabPanel;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use view::canvas_editor::CanvasEditor;
use view::menubar::Menubar;

use cursive::event::{Event, Key};
use cursive::theme::{BorderStyle, Palette};
use cursive::views::{Canvas, LinearLayout, TextView};
use cursive::{Cursive, CursiveExt, With};

pub struct WorkerThread {
  thread_active: Arc<AtomicU32>,
}
const PAUSED: u32 = 0;
const ACTIVE: u32 = 1;
impl WorkerThread {
  pub fn toggle_thread(&self, should_be_active: bool) {
    let is_currently_active =
      self.thread_active.load(std::sync::atomic::Ordering::SeqCst) == ACTIVE;

    if should_be_active && !is_currently_active {
      self.thread_active.store(ACTIVE, Ordering::SeqCst);
      atomic_wait::wake_all(&(*self.thread_active));
    } else if !should_be_active && is_currently_active {
      self.thread_active.store(PAUSED, Ordering::SeqCst);
    }
  }

  pub fn spawn(cb_sink: cursive::CbSink) -> WorkerThread {
    let thread_active_flag = Arc::new(AtomicU32::new(PAUSED));

    {
      let thread_active_flag = thread_active_flag.clone();
      std::thread::spawn(move || {
        // loop {
        atomic_wait::wait(&thread_active_flag, PAUSED); // waits while the value is PAUSED (0)
                                                        // println!("thread_active_flag:{:?}", thread_active_flag);

        do_work(cb_sink);

        std::thread::sleep(std::time::Duration::from_secs(10));
        // }
      });
    }

    WorkerThread {
      thread_active: thread_active_flag,
    }
  }
}

fn do_work(cb_sink: cursive::CbSink) {
  let metronome = metronome::Metronome::new();
  metronome.run(cb_sink);
}

pub fn init_default_style(siv: &mut Cursive) {
  siv.set_theme(cursive::theme::Theme {
    shadow: false,
    borders: BorderStyle::Simple,
    palette: Palette::retro().with(|palette| {
      use cursive::style::Color::TerminalDefault;
      use cursive::style::PaletteColor::{
        Background, Highlight, HighlightInactive, HighlightText, Primary, Secondary, Shadow,
        Tertiary, TitlePrimary, TitleSecondary, View,
      };

      palette[Background] = TerminalDefault;
      palette[View] = TerminalDefault;
      palette[Primary] = TerminalDefault;
      palette[TitlePrimary] = TerminalDefault;
      palette[Highlight] = TerminalDefault;
      palette[Secondary] = TerminalDefault;
      palette[HighlightInactive] = TerminalDefault;
      palette[HighlightText] = TerminalDefault;
      palette[Shadow] = TerminalDefault;
      palette[TitleSecondary] = TerminalDefault;
      palette[Tertiary] = TerminalDefault;
    }),
  });
}

fn main() {
  let mut siv: Cursive = Cursive::new();
  let menu_app = Menubar::build_menu_app();
  let menu_help = Menubar::build_menu_help();
  let mut regex = RegExpHandler::new();
  let mut anu: Anu = Anu::new();
  let cb_sink = siv.cb_sink().clone();
  // let metronome = metronome::Metronome::new();
  // let m_tx = metronome.tx.clone();

  // let state = Arc::new((Mutex::new(false), Condvar::new()));
  // let metronome_state = Arc::clone(&state);
  // let is_playing = Arc::new(Mutex::new(false));
  // let metronome_state = Arc::clone(&is_playing);

  // let parker = Parker::new();
  // let unparker = parker.unparker().clone();

  siv.set_autohide_menu(true);
  siv.set_autorefresh(true);
  siv.set_user_data(Menubar::default());
  siv.set_user_data(Anu::new());
  let current_data = siv
    .with_user_data(|controller_data: &mut Anu| controller_data.clone())
    .unwrap();

  init_default_style(&mut siv);

  let main_views = anu.build(regex.tx.clone());

  siv
    .menubar()
    .add_subtree("Anu", menu_app)
    .add_subtree("Help", menu_help)
    .add_delimiter()
    .add_leaf("Quit", |s| s.quit());

  siv.add_layer(main_views);

  siv.add_global_callback(Key::Esc, move |s| {
    if !current_data.show_regex_display {
      let mut regex_display_unit_view = s
        .find_name::<TextView>(config::regex_display_unit_view)
        .unwrap();
      regex_display_unit_view.set_content(utils::build_doc_string(&config::APP_WELCOME_MSG));
    }

    s.select_menubar();
  });

  // DRY?
  siv.add_global_callback(Key::F1, move |s| {
    let mut interactive_display_section_view = s
      .find_name::<LinearLayout>(config::main_section_view)
      .unwrap();
    let _ = interactive_display_section_view.set_focus_index(0);
  });
  siv.add_global_callback(Key::F2, move |s| {
    let mut interactive_display_section_view = s
      .find_name::<LinearLayout>(config::main_section_view)
      .unwrap();
    let _ = interactive_display_section_view.set_focus_index(1);
  });

  // thread::spawn(move || {
  //   loop {
  //     let (lock, cvar) = &*metronome_state;
  //     let mut playing = lock.lock().unwrap();

  //     // Wait until the metronome is playing
  //     while !*playing {
  //       playing = cvar.wait(playing).unwrap();
  //     }

  //     drop(playing); // Unlock before sleeping

  //     let tick_duration = Duration::from_millis(500);
  //     let start: Instant = Instant::now();

  //     // Simulate the metronome tick
  //     println!("Tick!");

  //     // Loop to check for pause during the sleep period
  //     while start.elapsed() < tick_duration {
  //       thread::sleep(Duration::from_millis(10)); // Short sleep to prevent busy-waiting

  //       // Check if the metronome should be paused
  //       let (lock, cvar) = &*metronome_state;
  //       let playing = lock.lock().unwrap();
  //       if !*playing {
  //         break; // Exit the loop if paused
  //       }
  //     }
  //   }
  // });

  // let input_state = Arc::clone(&state);
  // siv.add_global_callback(Event::Char(' '), move |_| {
  //   let (lock, cvar) = &*input_state;
  //   let mut playing = lock.lock().unwrap();
  //   *playing = !*playing;

  //   if *playing {
  //     println!("Metronome Started");
  //     cvar.notify_one(); // Wake up the metronome thread
  //   } else {
  //     println!("Metronome Paused");
  //     // The metronome thread will park itself on the next iteration
  //   }
  // });
  let worker = WorkerThread::spawn(cb_sink.clone());

  siv.add_global_callback(Event::Char(' '), move |s| {
    s.call_on_name(
      config::canvas_editor_section_view,
      |c: &mut Canvas<CanvasEditor>| worker.toggle_thread(c.state_mut().set_playing()),
    )
    .unwrap();
  });

  thread::spawn(move || {
    regex.run(cb_sink);
  });

  siv.run();
}
