use super::canvas::Canvas;
use super::menu::setup_app_menu;
use cursive::{Cursive, CursiveExt};

use cursive::style::{BorderStyle, Palette};
use cursive::traits::*;
use std::sync::mpsc;

#[derive(Debug)]
pub struct Account {
  pub name: String,
  pub user: String,
  pub password: String,
  pub url: String,
  pub notes: String,
}

#[derive(Debug)]
pub enum UiMessage {
  UpdateStatus,
  Refresh,
}
pub struct Ui {
  cursive: Cursive,
  ui_rx: mpsc::Receiver<UiMessage>,
  ui_tx: mpsc::Sender<UiMessage>,
  controller_tx: mpsc::Sender<Message>,
}

#[derive(Debug)]
pub enum Message {
  Sync,
  Quit,
}

impl Ui {
  pub fn new(s: Cursive, controller_tx: mpsc::Sender<Message>) -> Ui {
    let (ui_tx, ui_rx) = mpsc::channel::<UiMessage>();
    Ui {
      cursive: s,
      ui_tx,
      ui_rx,
      controller_tx,
    }
  }

  pub fn run(&mut self) -> bool {
    if !self.cursive.is_running() {
      return false;
    }

    self.cursive.run();

    while let Some(message) = self.next_ui_message() {
      match message {
        UiMessage::UpdateStatus => {}
        UiMessage::Refresh => {}
      }
    }
    true
  }

  pub fn init_app_with_default_style(&mut self) {
    self.cursive.set_theme(cursive::theme::Theme {
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

  pub fn add_canvas(&mut self) {
    let canvas = Canvas::new().full_width().full_height();

    self.cursive.add_layer(canvas);
  }

  pub fn start(&mut self) {
    self.init_app_with_default_style();
    setup_app_menu(&mut self.cursive);
    self.add_canvas();
  }

  /// Retrieve the next available UiMessage to process.
  pub fn next_ui_message(&self) -> Option<UiMessage> {
    self.ui_rx.try_iter().next()
  }

  pub fn quit(&mut self) {
    self.cursive.quit();
  }
}
