use cursive::{
  event::{Callback, Event, EventResult, Key},
  theme::{BorderStyle, Palette},
  views::{LinearLayout, TextView},
  Cursive, Printer, View, With,
};

use super::{
  config,
  controller::{Controller, ControllerData},
  menubar::Menubar,
  utils,
};

pub struct Anu {
  pub menubar: Menubar,
  pub controller: Controller,
}

impl Default for Anu {
  fn default() -> Self {
    Self::new()
  }
}

impl Anu {
  pub fn new() -> Anu {
    let menubar = Menubar::new();
    let controller = Controller::new();
    Anu {
      menubar,
      controller,
    }
  }

  pub fn init_default_style(&mut self, siv: &mut Cursive) {
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

  pub fn build_menubar(&mut self, siv: &mut Cursive) {
    self.menubar.init(siv);
  }

  pub fn build_root_view(&mut self, siv: &mut Cursive) -> LinearLayout {
    let mut current_data = siv
      .with_user_data(|controller_data: &mut ControllerData| controller_data.clone())
      .unwrap();

    siv.add_global_callback(Key::Esc, move |s| {
      if !current_data.show_regex_display {
        let mut text_view = s.find_name::<TextView>("interactive_display_view").unwrap();
        text_view
          .get_shared_content()
          .set_content(utils::build_doc_string(&config::APP_WELCOME_MSG));
      }
      s.select_menubar()
    });

    self.controller.init(&mut current_data)
  }

  // fn on_down(&mut self, is_drop: bool, is_begin: bool) -> EventResult {
  //   // if self.is_paused {
  //   //   return EventResult::Consumed(None);
  //   // }
  //   // let (gameover, hit_bottom) = self.board.on_down(is_drop, is_begin);
  //   // let gameover = gameover || self.score.is_gameover();
  //   // if gameover {
  //   //   self.gameover = true;
  //   //   self.toggle_pause();
  //   //   return EventResult::Consumed(Some(Callback::from_fn(move |s| {
  //   //     s.add_layer(Dialog::info("Game Over!"));
  //   //   })));
  //   // }
  //   // if hit_bottom {
  //   //   if is_drop {
  //   //     self.merge_block();
  //   //   } else {
  //   //     self.hit_bottom = hit_bottom;
  //   //     self.frame_idx = 0;
  //   //     self.max_frame_idx = NORMAL_SPEED;
  //   //   }
  //   // }
  //   EventResult::Consumed(None)
  // }

  // fn new_game(&mut self) -> EventResult {
  //   // self.score.renew();
  //   // self.board.renew();
  //   // self.queue.renew();
  //   // self.timer.renew();
  //   // self.is_paused = false;
  //   // self.hit_bottom = false;
  //   // self.frame_idx = 0;
  //   // self.max_frame_idx = SLOW_SPEED;
  //   // self.gameover = false;
  //   EventResult::Consumed(None)
  // }

  // fn stop_and_resume(&mut self) -> EventResult {
  //   // self.toggle_pause();
  //   // if self.is_paused {
  //   //   EventResult::Consumed(Some(Callback::from_fn(move |s| {
  //   //     s.add_layer(Pause::new());
  //   //   })))
  //   // } else {
  //   EventResult::Consumed(None)
  //   // }
  // }

  // fn toggle_pause(&mut self) {
  //   // self.is_paused = !self.is_paused;
  //   // self.timer.toggle_pause();
  // }

  // fn handle_merge_and_pass(&mut self, event: Event) -> EventResult {
  //   // if self.gameover && event != Event::Char('n') && event != Event::Char('N') {
  //   //   return EventResult::Consumed(None);
  //   // }
  //   // let is_begin = self.hit_bottom;
  //   // if self.hit_bottom {
  //   //   self.merge_block();
  //   // }
  //   // match event {
  //   //   Event::Key(Key::Down) => self.speed_up(),
  //   //   Event::Refresh => self.on_down(false, is_begin),
  //   //   Event::Char(' ') => self.on_down(true, is_begin),
  //   //   Event::Char('n') | Event::Char('N') => self.new_game(),
  //   //   Event::Char('m') | Event::Char('M') => self.stop_and_resume(),
  //   //   _ => EventResult::Ignored,
  //   // }
  //   EventResult::Ignored
  // }

  // fn merge_block(&mut self) {
  //   // let score = self.board.merge_block();
  //   // self.score.add(score);
  //   // let block = self.queue.pop_and_spawn_new_block();
  //   // self.board.insert(block);
  //   // self.hit_bottom = false;
  //   // self.max_frame_idx = SLOW_SPEED;
  //   // self.frame_idx = 0;
  // }

  // fn speed_up(&mut self) -> EventResult {
  //   // self.max_frame_idx = FAST_SPEED;
  //   // self.frame_idx = 0;
  //   EventResult::Consumed(None)
  // }

  fn pass_event_to_board(&mut self, event: Event) -> EventResult {
    // let moved = self.controller.handle_event(event);
    EventResult::Ignored
  }
}

impl View for Anu {
  fn draw(&self, printer: &Printer) {
    self.menubar.draw(printer);
    self.controller.draw(printer);
  }

  // fn required_size(&mut self, constraints: Vec2) -> Vec2 {
  //   // let score_size = self.score.required_size(constraints);
  //   // let timer_size = self.timer.required_size(constraints);
  //   // let manual_size = self.manual.required_size(constraints);
  //   // let board_size = self.board.required_size(constraints);
  //   // let queue_size = self.queue.required_size(constraints);
  //   // Vec2::new(
  //   //   std::cmp::max(std::cmp::max(manual_size.x, score_size.x), timer_size.x)
  //   //     + board_size.x
  //   //     + queue_size.x
  //   //     + 10,
  //   //   board_size.y,
  //   // )
  //   Vec2::new(0, 0)
  // }

  fn on_event(&mut self, event: Event) -> EventResult {
    match event {
      Event::Char('h') => {
        let cb = Callback::from_fn(Menubar::add_doc_view);
        EventResult::Consumed(Some(cb))
      }
      _ => self.pass_event_to_board(event),
    }
  }
}
