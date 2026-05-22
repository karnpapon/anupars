// Core submodules
pub mod easing;
pub mod midi;
pub mod movement;
pub mod position;
pub mod queue;
pub mod tilt;

// Feature submodules
pub mod accumulation;
pub mod aim;
pub mod modes;
pub mod music;
pub mod transform;
pub mod types;

// no better name than "playhead" for now
#[allow(clippy::module_inception)]
mod playhead;

#[cfg(test)]
pub mod test_helpers;

pub use playhead::Playhead;
pub use types::{Direction, Message, PlayheadUI, UIUpdate};
// pub use types::{GridState, ModeFlags, MusicState};
