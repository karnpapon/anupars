use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::thread;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

use cursive::views::{Canvas, TextView};
use cursive::Cursive;

use crate::core::engine::regex;
use crate::core::engine::symspell::SymSpellState;
use crate::core::{consts, utils};
use crate::view::grid::GridEditor;
use crate::view::rect::Rect;

use crate::core::playhead::{Playhead, UIUpdate};

/// Apply one typed UIUpdate to a live Cursive instance.
/// Called from spawn_ui_processor (native, inside a cb_sink closure)
/// and from wasm_step (WASM, directly on the runner).
pub fn apply_ui_update_cursive(
  siv: &mut Cursive,
  update: UIUpdate,
  sym_state: &Arc<SymSpellState>,
  _regex_tx: &Sender<regex::Message>,
) {
  match update {
    // --- existing canvas geometry updates ---
    UIUpdate::ActivePos(active_pos) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.actived_pos = active_pos;
      });
    }
    UIUpdate::AccumulationCounter(count, total) => {
      siv.call_on_name(consts::input_status_unit_view, move |view: &mut TextView| {
        view.set_content(format!("@ {}/{}", count, total));
      });
    }
    UIUpdate::PlayheadPosAndArea(pos, area) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        let editor = canvas.state_mut();
        editor.playhead_ui.playhead_pos = pos;
        editor.playhead_ui.playhead_area = area;
      });
      siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
        view.set_content(utils::build_pos_status_str(pos));
      });
      let area_size = area.size();
      siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
        view.set_content(utils::build_len_status_str((area_size.x, area_size.y)));
      });
    }
    UIUpdate::ChnStatus(s) => {
      siv.call_on_name(consts::chn_status_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::GridSplits(v, h) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        let editor = canvas.state_mut();
        editor.playhead_ui.grid_v_splits = v;
        editor.playhead_ui.grid_h_splits = h;
      });
    }
    UIUpdate::AimedArea(aimed_area) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.aimed_area = aimed_area;
      });
    }
    UIUpdate::TmpAppendSpace => sym_state.handle_buf_append_space(siv),
    UIUpdate::TmpAppend(idx) => sym_state.handle_buf_append(siv, idx),
    UIUpdate::RplCycle(old_area) => sym_state.handle_rpl_cycle(siv, old_area),
    UIUpdate::SweepX(x) => {
      siv.call_on_name(consts::canvas_editor_section_view, |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.sweep_x = x;
      });
    }

    // --- status-bar text updates ---
    UIUpdate::BpmDisplay(s) => {
      siv.call_on_name(consts::bpm_status_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::RatioStatus(s) => {
      siv.call_on_name(consts::ratio_status_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::PosStatus(s) => {
      siv.call_on_name(consts::pos_status_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::LenStatus(s) => {
      siv.call_on_name(consts::len_status_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::InputStatus(s) => {
      siv.call_on_name(consts::input_status_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::ModeStatus(s) => {
      siv.call_on_name(consts::mode_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::MovementStatus(s) => {
      siv.call_on_name(consts::movement_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::TiltStatus(s) => {
      siv.call_on_name(consts::tilt_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::RegexMatchCount(s) => {
      siv.call_on_name(consts::regex_matches_amount_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }
    UIUpdate::RegexError(s) => {
      siv.call_on_name(consts::regex_err_display_unit_view, |view: &mut TextView| {
        view.set_content(s);
      });
    }

    // --- canvas field updates ---
    UIUpdate::TextMatcher { matcher, regex_indexes } => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        let editor = canvas.state_mut();
        editor.playhead_ui.text_matcher = matcher;
        editor.playhead_ui.regex_indexes = regex_indexes;
      });
    }
    UIUpdate::QueueManagerUpdate(qm) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.queue_manager = qm;
      });
    }
    UIUpdate::CanvasPlayheadPos(pos) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.playhead_pos = pos;
      });
    }
    UIUpdate::CanvasPlayheadArea(area) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.playhead_area = area;
      });
    }
    UIUpdate::CanvasArpeggiatorMode(v) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.arpeggiator_mode = v;
      });
    }
    UIUpdate::CanvasAccumulationMode(v) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.accumulation_mode = v;
      });
    }
    UIUpdate::CanvasEventOperatorMode(v) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.event_operator_mode = v;
      });
    }
    UIUpdate::CanvasDrainQueueMode(v) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.drain_queue_mode = v;
      });
    }
    UIUpdate::CanvasSweepMode(v) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.sweep_mode = v;
      });
    }
    UIUpdate::CanvasSweepMovement(mv) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.sweep_movement = mv;
      });
    }
    UIUpdate::CanvasSweepOutputMode(m) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.sweep_output_mode = m;
      });
    }
    UIUpdate::CanvasSweepCcNumber(n) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.sweep_cc_number = n;
      });
    }
    UIUpdate::CanvasSweepRowMode(m) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.sweep_row_mode = m;
      });
    }
    UIUpdate::CanvasTiltMode(t) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.tilt_mode = t;
      });
    }
    UIUpdate::CanvasFreezeMode(v) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.freeze_mode = v;
      });
    }
    UIUpdate::CanvasDroneMode(is_drone, drone_x) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        let editor = canvas.state_mut();
        editor.playhead_ui.drone_mode = is_drone;
        editor.playhead_ui.drone_x = drone_x;
      });
    }
    UIUpdate::CanvasDroneX(x) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.drone_x = x;
      });
    }
    UIUpdate::CanvasDroneChannel(ch) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.drone_channel = ch;
      });
    }
    UIUpdate::CanvasScaleRootTop(r) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.scale_root_top = r;
      });
    }
    UIUpdate::CanvasScaleRootLeft(r) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.scale_root_left = r;
      });
    }
    UIUpdate::CanvasScaleModeTop(m) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.scale_mode_top = m;
      });
    }
    UIUpdate::CanvasScaleModeLeft(m) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.scale_mode_left = m;
      });
    }
    UIUpdate::CanvasKeyboardTopActive(v) => {
      siv.call_on_name(consts::canvas_editor_section_view, move |canvas: &mut Canvas<GridEditor>| {
        canvas.state_mut().playhead_ui.keyboard_top_active = v;
      });
    }
  }
}

impl Playhead {
  #[cfg(not(target_arch = "wasm32"))]
  pub fn spawn_ui_processor(
    ui_rx: Receiver<UIUpdate>,
    cb_sink: cursive::CbSink,
    sym_state: Arc<SymSpellState>,
    regex_tx: Sender<regex::Message>,
  ) {
    thread::Builder::new()
      .name(consts::THREAD_NAME_UI_PROCESSOR.to_string())
      .spawn(move || loop {
        thread::sleep(Duration::from_millis(16)); // ~60 FPS

        let anim_tick = sym_state.advance_anim_frame();

        let mut updates: Vec<UIUpdate> = Vec::new();
        while let Ok(u) = ui_rx.try_recv() {
          updates.push(u);
        }

        if updates.is_empty() && anim_tick.is_none() {
          continue;
        }

        let sym_state_cb = Arc::clone(&sym_state);
        let regex_tx_cb = regex_tx.clone();

        cb_sink
          .send(Box::new(move |siv| {
            if let Some(tick) = anim_tick {
              sym_state_cb.render_anim_tick(siv, tick, &regex_tx_cb);
            }
            for update in updates {
              apply_ui_update_cursive(siv, update, &sym_state_cb, &regex_tx_cb);
            }
          }))
          .unwrap();
      })
      .expect("Failed to spawn UI batch processor thread");
  }

  pub(crate) fn enqueue_sym_space(&self) {
    let _ = self.ui_tx.send(UIUpdate::TmpAppendSpace);
  }

  pub(crate) fn enqueue_sym_buf_append(&self, idx: usize) {
    use std::sync::atomic::Ordering;
    if !self.modes.accumulation_mode.load(Ordering::Relaxed) {
      return;
    }
    let _ = self.ui_tx.send(UIUpdate::TmpAppend(idx));
  }

  pub(crate) fn enqueue_sym_rpl_cycle(&self, area: Rect) {
    let _ = self.ui_tx.send(UIUpdate::RplCycle(area));
  }
}
