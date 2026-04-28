pub mod buffer;
pub mod cell;

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
    use crossterm::{
      cursor::Hide,
      execute,
      terminal::{enable_raw_mode, EnterAlternateScreen},
    };
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen, Hide)?;
    Ok(RawMode)
  }
}

#[cfg(not(target_arch = "wasm32"))]
impl Drop for RawMode {
  fn drop(&mut self) {
    use crossterm::{
      cursor::Show,
      execute,
      terminal::{disable_raw_mode, LeaveAlternateScreen},
    };
    disable_raw_mode().ok();
    execute!(stdout(), LeaveAlternateScreen, Show).ok();
  }
}
