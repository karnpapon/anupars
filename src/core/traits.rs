use std::{collections::HashMap, sync::mpsc::Sender};

use cursive::{
  theme::{ColorStyle, ColorType, Style},
  utils::span::SpannedString,
  Printer, Vec2,
};

use crate::view::common::{canvas_editor::MarkerUI, marker};

use super::{config, midi, regex::Match};

#[derive(Clone, Default, Debug)]
pub struct Matrix<T> {
  pub data: Vec<T>,
  pub width: usize,
  pub height: usize,
  // pub
}

impl<T: Copy> Matrix<T> {
  pub fn new(width: usize, height: usize, default: T) -> Matrix<T> {
    let mut data: Vec<T> = Vec::with_capacity(width * height);
    for _ in 0..width * height {
      data.push(default);
    }
    Matrix {
      data,
      width,
      height,
    }
  }

  pub fn get(&self, x: usize, y: usize) -> Option<&T> {
    self.data.get(x + y * self.width)
  }

  pub fn set(&mut self, x: usize, y: usize, item: T) {
    self.data[x + y * self.height] = item;
  }

  pub fn set_rect(&mut self, width: usize, height: usize, item: T) {
    for i in 0..height {
      for j in 0..width {
        self.set(j, i, item);
      }
    }
  }

  pub fn index_to_xy(&self, index: &usize) -> Vec2 {
    let x = index % self.width;
    let y = index / self.width;
    (x, y).into()
  }
}

pub trait Printable {
  fn display_char(&self, pos: cursive::XY<usize>) -> char;
  fn should_rest(&self, _pos: cursive::XY<usize>) -> bool {
    false
  }
}

impl Printable for char {
  fn should_rest(&self, pos: cursive::XY<usize>) -> bool {
    pos.x % config::GRID_ROW_SPACING == 0 && pos.y % config::GRID_COL_SPACING == 0
  }

  fn display_char(&self, pos: cursive::XY<usize>) -> char {
    match *self {
      '\0' => match self.should_rest(pos) {
        true => ':',
        false => '.',
      },
      _ => *self,
    }
  }
}

impl<T: Printable + Copy> Matrix<T> {
  pub fn print(&self, printer: &Printer, marker_ui: &MarkerUI, marker_tx: Sender<marker::Message>) {
    let MarkerUI {
      regex_indexes,
      text_matcher,
      marker_pos,
      marker_area,
      actived_pos,
    } = &marker_ui;

    for y in 0..self.width {
      for x in 0..self.height {
        let style = if text_matcher.is_some() {
          let hl = text_matcher.as_ref().unwrap();
          if hl.get(&(y + x * self.width)).is_some() {
            Style::highlight()
          } else {
            Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100)))
          }
        } else {
          Style::from_color_style(ColorStyle::front(ColorType::rgb(100, 100, 100)))
        };

        printer.print_styled(
          (y, x),
          &SpannedString::styled(
            self
              .get(y, x)
              .unwrap()
              .display_char((x, y).into())
              .to_string(),
            style,
          ),
        );

        // draw marker
        // is_head_pos
        // if (y, x) == (marker_pos.x, marker_pos.y) {
        //   printer.print_styled(marker_pos, &SpannedString::styled('>', Style::highlight()));
        // }

        // is within marker area
        if marker_area.contains((y, x).into()) {
          if marker_pos.saturating_add(actived_pos).eq(&(y, x)) {
            printer.print_styled((y, x), &SpannedString::styled('>', Style::none()));

            if text_matcher.is_some() {
              let curr_running_marker = y + x * self.width;
              let hl = text_matcher.as_ref().unwrap();
              if hl.get(&curr_running_marker).is_some() {
                let _ = marker_tx.send(marker::Message::TriggerWithRegexPos((
                  curr_running_marker,
                  regex_indexes.clone(),
                )));

                printer.print_styled((y, x), &SpannedString::styled('@', Style::none()));
              }
            }
          } else {
            // inside marker area
            printer.print_styled(
              (y, x),
              &SpannedString::styled(
                self
                  .get(y, x)
                  .unwrap()
                  .display_char((x, y).into())
                  .to_string(),
                Style::highlight(),
              ),
            );

            if text_matcher.is_some() {
              let curr_running_marker = y + x * self.width;
              let hl = text_matcher.as_ref().unwrap();
              if hl.get(&curr_running_marker).is_some() {
                let mut regex_idx = regex_indexes.lock().unwrap();
                regex_idx.insert(curr_running_marker);
                regex_idx.retain(|re_idx: &usize| {
                  let dd = self.index_to_xy(re_idx);
                  dd.fits(marker_pos) && dd.fits_in(*marker_pos + marker_area.size())
                });

                printer.print_styled((y, x), &SpannedString::styled('*', Style::highlight()));
              }
            }
          }
        }
      }
    }
  }
}
