pub mod app;
pub mod app_state;
#[cfg(not(target_arch = "wasm32"))]
pub mod app_event;
pub mod core;
pub mod terminal;
pub mod view;

#[cfg(target_arch = "wasm32")]
pub mod wasm;
