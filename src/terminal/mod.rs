pub mod buffer;
pub mod cell;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor::Hide, cursor::Show, execute};
#[cfg(not(target_arch = "wasm32"))]
pub mod renderer;

#[cfg(not(target_arch = "wasm32"))]
pub use renderer::Renderer;

#[cfg(not(target_arch = "wasm32"))]
use std::io::{self, stdout};

/// RAII guard that enters raw mode + alternate screen on construction and
/// restores the terminal on drop.
#[cfg(not(target_arch = "wasm32"))]
pub struct RawMode;

#[cfg(not(target_arch = "wasm32"))]
impl RawMode {
  pub fn enter() -> io::Result<Self> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    Ok(RawMode)
  }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for RawMode {
  fn drop(&mut self) {
    disable_raw_mode().ok();
    execute!(stdout(), LeaveAlternateScreen, Show).ok();
  }
}
