use cursive::{
  event::{Callback, Event, EventResult, Key},
  theme::{Color, ColorStyle},
  views::Dialog,
  Cursive, Printer, Vec2, View,
};

use super::menubar::Menubar;

pub struct Anu {
  menubar: Menubar,
  is_paused: bool,
  hit_bottom: bool,
  frame_idx: usize,
  max_frame_idx: usize,
  gameover: bool,
}

impl Default for Anu {
  fn default() -> Self {
    Self::new()
  }
}

impl Anu {
  pub fn new() -> Anu {
    let mut menubar = Menubar::new();
    Anu {
      menubar,
      is_paused: false,
      hit_bottom: false,
      frame_idx: 0,
      max_frame_idx: 10,
      gameover: false,
    }
  }

  pub fn init(&mut self, siv: &mut Cursive) -> EventResult {
    self.menubar.init(siv);
    // self.board.renew();
    // self.queue.renew();
    // self.timer.renew();
    self.is_paused = false;
    self.hit_bottom = false;
    self.frame_idx = 0;
    // self.max_frame_idx = SLOW_SPEED;
    self.gameover = false;
    EventResult::Consumed(None)
  }

  fn on_down(&mut self, is_drop: bool, is_begin: bool) -> EventResult {
    // if self.is_paused {
    //   return EventResult::Consumed(None);
    // }
    // let (gameover, hit_bottom) = self.board.on_down(is_drop, is_begin);
    // let gameover = gameover || self.score.is_gameover();
    // if gameover {
    //   self.gameover = true;
    //   self.toggle_pause();
    //   return EventResult::Consumed(Some(Callback::from_fn(move |s| {
    //     s.add_layer(Dialog::info("Game Over!"));
    //   })));
    // }
    // if hit_bottom {
    //   if is_drop {
    //     self.merge_block();
    //   } else {
    //     self.hit_bottom = hit_bottom;
    //     self.frame_idx = 0;
    //     self.max_frame_idx = NORMAL_SPEED;
    //   }
    // }
    EventResult::Consumed(None)
  }

  fn new_game(&mut self) -> EventResult {
    // self.score.renew();
    // self.board.renew();
    // self.queue.renew();
    // self.timer.renew();
    // self.is_paused = false;
    // self.hit_bottom = false;
    // self.frame_idx = 0;
    // self.max_frame_idx = SLOW_SPEED;
    // self.gameover = false;
    EventResult::Consumed(None)
  }

  fn stop_and_resume(&mut self) -> EventResult {
    // self.toggle_pause();
    // if self.is_paused {
    //   EventResult::Consumed(Some(Callback::from_fn(move |s| {
    //     s.add_layer(Pause::new());
    //   })))
    // } else {
    EventResult::Consumed(None)
    // }
  }

  fn toggle_pause(&mut self) {
    // self.is_paused = !self.is_paused;
    // self.timer.toggle_pause();
  }

  fn handle_merge_and_pass(&mut self, event: Event) -> EventResult {
    if self.gameover && event != Event::Char('n') && event != Event::Char('N') {
      return EventResult::Consumed(None);
    }
    let is_begin = self.hit_bottom;
    if self.hit_bottom {
      self.merge_block();
    }
    match event {
      Event::Key(Key::Down) => self.speed_up(),
      Event::Refresh => self.on_down(false, is_begin),
      Event::Char(' ') => self.on_down(true, is_begin),
      Event::Char('n') | Event::Char('N') => self.new_game(),
      Event::Char('m') | Event::Char('M') => self.stop_and_resume(),
      _ => EventResult::Ignored,
    }
  }

  fn merge_block(&mut self) {
    // let score = self.board.merge_block();
    // self.score.add(score);
    // let block = self.queue.pop_and_spawn_new_block();
    // self.board.insert(block);
    // self.hit_bottom = false;
    // self.max_frame_idx = SLOW_SPEED;
    // self.frame_idx = 0;
  }

  fn speed_up(&mut self) -> EventResult {
    // self.max_frame_idx = FAST_SPEED;
    // self.frame_idx = 0;
    EventResult::Consumed(None)
  }

  fn pass_event_to_board(&mut self, event: Event) -> EventResult {
    // if self.is_paused || self.gameover {
    //   return EventResult::Consumed(None);
    // }
    // let moved = self.board.handle_event(event, self.hit_bottom);
    // if self.hit_bottom && moved {
    //   self.max_frame_idx = std::cmp::min(3 + self.max_frame_idx, 2 * NORMAL_SPEED);
    // }
    EventResult::Consumed(None)
  }
}

impl View for Anu {
  fn draw(&self, printer: &Printer) {
    // let x_padding = 4;
    // let y_padding = 4;
    // let score_padding = Vec2::new(x_padding, y_padding);
    // let timer_padding = Vec2::new(x_padding, y_padding + 1 + self.score_size.y);
    // let manual_padding = Vec2::new(x_padding, y_padding + self.score_size.y + self.timer_size.y);
    // let first_column_x_padding = std::cmp::max(
    //   std::cmp::max(self.manual_size.x, self.score_size.x),
    //   self.timer_size.x,
    // );
    // let board_padding = Vec2::new(x_padding + first_column_x_padding + 2, y_padding);
    // let queue_padding = Vec2::new(
    //   x_padding + first_column_x_padding + self.board_size.x,
    //   y_padding,
    // );

    // let score_printer = printer.offset(score_padding);
    // let timer_printer = printer.offset(timer_padding);
    // let manual_printer = printer.offset(manual_padding);
    // let board_printer = printer.offset(board_padding);
    // let queue_printer = printer.offset(queue_padding);

    // self.score.draw(&score_printer);
    // self.timer.draw(&timer_printer);
    // self.manual.draw(&manual_printer);
    // self.board.draw(&board_printer);
    // self.queue.draw(&queue_printer);
  }

  fn required_size(&mut self, constraints: Vec2) -> Vec2 {
    // let score_size = self.score.required_size(constraints);
    // let timer_size = self.timer.required_size(constraints);
    // let manual_size = self.manual.required_size(constraints);
    // let board_size = self.board.required_size(constraints);
    // let queue_size = self.queue.required_size(constraints);
    // Vec2::new(
    //   std::cmp::max(std::cmp::max(manual_size.x, score_size.x), timer_size.x)
    //     + board_size.x
    //     + queue_size.x
    //     + 10,
    //   board_size.y,
    // )

    Vec2::new(0, 0)
  }

  fn on_event(&mut self, event: Event) -> EventResult {
    // if event == Event::Refresh {
    //   self.frame_idx += 1;
    //   if self.frame_idx == self.max_frame_idx {
    //     self.frame_idx = 0;
    //   } else {
    //     return EventResult::Ignored;
    //   }
    // }

    // match event {
    //   Event::Refresh
    //   | Event::Key(Key::Down)
    //   | Event::Char(' ')
    //   | Event::Char('n')
    //   | Event::Char('N')
    //   | Event::Char('m')
    //   | Event::Char('M') => self.handle_merge_and_pass(event),
    //   _ => self.pass_event_to_board(event),
    // }

    return EventResult::Ignored;
  }
}
