#![allow(non_upper_case_globals)]

/// Rows the console panel occupies at the bottom of the screen.
pub const CONSOLE_HEIGHT: u16 = 5;

/// Horizontal padding (columns) applied to the left of console and grid.
pub const PADDING_X: u16 = 2;
/// Vertical padding (rows) between the menubar and the content area,
/// and below the grid.
pub const PADDING_Y: u16 = 1;

pub static GRID_ROW_SPACING: usize = 9;
pub static GRID_COL_SPACING: usize = 9;

pub static REST_CHAR: char = ':';
pub static EMPTY_CHAR: char = '.';
