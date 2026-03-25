//! View identifier constants — names used with `call_on_name` / `find_name`.
//!
//! Naming convention: `<identifier>_<category>_view`
//! where `category` is `unit` (leaf widget) or `section` (container).
#![allow(non_upper_case_globals)]

// --- unit views ---
pub static regex_input_unit_view: &str = "regex_input_unit_view";
pub static regex_matches_amount_unit_view: &str = "regex_matches_amount_unit_view";
pub static input_status_unit_view: &str = "input_status_unit_view";
pub static chn_status_unit_view: &str = "chn_status_unit_view";
pub static bpm_status_unit_view: &str = "bpm_status_unit_view";
pub static ratio_status_unit_view: &str = "ratio_status_unit_view";
pub static len_status_unit_view: &str = "len_status_unit_view";
pub static pos_status_unit_view: &str = "pos_status_unit_view";
pub static mode_unit_view: &str = "mode_unit_view";
pub static movement_unit_view: &str = "movement_unit_view";
pub static tilt_unit_view: &str = "tilt_unit_view";
pub static luck_unit_view: &str = "luck_unit_view";
pub static midi_status_unit_view: &str = "midi_status_unit_view";
pub static buf_status_unit_view: &str = "buf_status_unit_view";
pub static sym_status_unit_view: &str = "sym_status_unit_view";
pub static rpl_status_unit_view: &str = "rpl_status_unit_view";
pub static regex_err_display_unit_view: &str = "regex_err_display_unit_view";
pub static doc_unit_view: &str = "doc_unit_view";
pub static file_explorer_unit_view: &str = "file_explorer_unit_view";
pub static file_contents_unit_view: &str = "file_contents_unit_view";

// alias for `regex_display_unit_view`
pub static display_view: &str = "display_view";

// --- section views ---
pub static input_controller_section_view: &str = "input_controller_section_view";
pub static status_controller_section_view: &str = "status_controller_section_view";
pub static playhead_controller_section_view: &str = "playhead_controller_section_view";
pub static buf_controller_section_view: &str = "buf_controller_section_view";
pub static control_section_view: &str = "control_section_view";
pub static canvas_editor_section_view: &str = "canvas_editor_section_view";
pub static main_section_view: &str = "main_section_view";

// --- canvas grid spacing ---
pub static GRID_ROW_SPACING: usize = 9;
pub static GRID_COL_SPACING: usize = 9;
