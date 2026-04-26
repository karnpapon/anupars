use std::io::{self, Write};

use crossterm::cursor::{Hide, MoveTo};
use crossterm::style::{
  Color as CColor, Print, ResetColor, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{
  disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::{execute, QueueableCommand};

use super::buffer::ScreenBuffer;
use super::cell::{Cell, Color};

pub struct Renderer {
  current: ScreenBuffer,
  previous: ScreenBuffer,
}

impl Renderer {
  pub fn new(width: u16, height: u16) -> Self {
    enable_raw_mode().expect("failed to enable raw mode");
    execute!(
      std::io::stdout(),
      EnterAlternateScreen,
      crossterm::terminal::Clear(crossterm::terminal::ClearType::All),
      crossterm::cursor::Hide
    )
    .expect("failed to enter alternate screen");
    Self {
      current: ScreenBuffer::new(width, height),
      previous: ScreenBuffer::new(width, height),
    }
  }

  pub fn current_mut(&mut self) -> &mut ScreenBuffer {
    &mut self.current
  }

  pub fn resize(&mut self, width: u16, height: u16) {
    self.current.resize(width, height);
    self.previous.resize(width, height);
    // Clear the physical screen so stale content from before the resize
    // doesn't persist - previous is now all-default, terminal must match
    let _ = execute!(
      std::io::stdout(),
      crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    );
  }

  /// Diff current vs previous, emit only changed cells, then swap buffers and
  /// clear current for the next frame.
  pub fn flush(&mut self, stdout: &mut impl Write) -> io::Result<()> {
    let width = self.current.width as usize;

    for y in 0..self.current.height {
      let mut x = 0u16;
      while x < self.current.width {
        let idx = y as usize * width + x as usize;
        let cur = &self.current.cells[idx];
        let prev = &self.previous.cells[idx];

        if cur != prev {
          stdout.queue(MoveTo(x, y))?;
          queue_style(stdout, cur)?;

          // Batch consecutive changed cells on the same row that share the same style.
          let mut run = String::new();
          let mut rx = x;
          while rx < self.current.width {
            let ridx = y as usize * width + rx as usize;
            let rc = &self.current.cells[ridx];
            let rp = &self.previous.cells[ridx];
            if rc != rp && rc.fg == cur.fg && rc.bg == cur.bg && rc.reverse == cur.reverse {
              run.push(if rc.ch == '\0' { ' ' } else { rc.ch });
              rx += 1;
            } else {
              break;
            }
          }
          stdout.queue(Print(&run))?;
          x = rx;
        } else {
          x += 1;
        }
      }
    }

    stdout.queue(ResetColor)?;
    stdout.queue(Hide)?;
    stdout.flush()?;

    std::mem::swap(&mut self.current, &mut self.previous);

    Ok(())
  }
}

impl Drop for Renderer {
  fn drop(&mut self) {
    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    let _ = disable_raw_mode();
  }
}

fn to_crossterm_color(c: Color) -> CColor {
  match c {
    Color::Reset => CColor::Reset,
    Color::Rgb(r, g, b) => CColor::Rgb { r, g, b },
    Color::Indexed(i) => CColor::AnsiValue(i),
  }
}

fn queue_style(stdout: &mut impl Write, cell: &Cell) -> io::Result<()> {
  if cell.reverse {
    stdout.queue(SetForegroundColor(to_crossterm_color(cell.bg)))?;
    stdout.queue(SetBackgroundColor(to_crossterm_color(cell.fg)))?;
  } else {
    stdout.queue(SetForegroundColor(to_crossterm_color(cell.fg)))?;
    stdout.queue(SetBackgroundColor(to_crossterm_color(cell.bg)))?;
  }
  Ok(())
}
