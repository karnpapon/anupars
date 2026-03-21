use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cursive::views::{Canvas, TextView};

use crate::core::engine::regex;
use crate::core::engine::symspell::{AnimTick, SymSpellState};
use crate::core::{consts, utils};
use crate::view::grid::GridEditor;
use crate::view::rect::Rect;

use crate::core::playhead::{Playhead, UIUpdate};

impl Playhead {
  pub fn spawn_ui_processor(
    ui_queue: Arc<Mutex<VecDeque<UIUpdate>>>,
    cb_sink: cursive::CbSink,
    sym_state: Arc<SymSpellState>,
    regex_tx: Sender<regex::Message>,
  ) {
    thread::Builder::new()
      .name("ui-batch-processor".to_string())
      .spawn(move || loop {
        thread::sleep(Duration::from_millis(16)); // ~60 FPS

        let anim_tick: Option<AnimTick> = sym_state.advance_anim_frame();

        let updates: Vec<UIUpdate> = {
          let mut queue = ui_queue.lock().unwrap();
          queue.drain(..).collect()
        };

        if updates.is_empty() && anim_tick.is_none() {
          continue;
        }

        let sym_state_cb = Arc::clone(&sym_state);
        let regex_tx_cb = regex_tx.clone();

        cb_sink
          .send(Box::new(move |siv| {
            let sym_state = sym_state_cb;
            let regex_tx = regex_tx_cb;

            if let Some(tick) = anim_tick {
              sym_state.render_anim_tick(siv, tick, &regex_tx);
            }

            for update in updates {
              match update {
                UIUpdate::ActivePos(active_pos) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.actived_pos = active_pos;
                    },
                  );
                }
                UIUpdate::AccumulationCounter(count, total) => {
                  siv.call_on_name(
                    consts::input_status_unit_view,
                    move |view: &mut TextView| {
                      view.set_content(format!("@ {}/{}", count, total));
                    },
                  );
                }
                UIUpdate::PlayheadPosAndArea(pos, area) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.playhead_pos = pos;
                      editor.playhead_ui.playhead_area = area;
                    },
                  );
                  siv.call_on_name(consts::pos_status_unit_view, move |view: &mut TextView| {
                    view.set_content(utils::build_pos_status_str(pos));
                  });
                  let area_size = area.size();
                  siv.call_on_name(consts::len_status_unit_view, move |view: &mut TextView| {
                    view.set_content(utils::build_len_status_str((area_size.x, area_size.y)));
                  });
                }
                UIUpdate::ChnStatus(chn_str) => {
                  siv.call_on_name(consts::chn_status_unit_view, |view: &mut TextView| {
                    view.set_content(chn_str);
                  });
                }
                UIUpdate::GridSplits(v, h) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.grid_v_splits = v;
                      editor.playhead_ui.grid_h_splits = h;
                    },
                  );
                }
                UIUpdate::AimedArea(aimed_area) => {
                  siv.call_on_name(
                    consts::canvas_editor_section_view,
                    move |canvas: &mut Canvas<GridEditor>| {
                      let editor = canvas.state_mut();
                      editor.playhead_ui.aimed_area = aimed_area;
                      if aimed_area.is_none() {
                        editor.is_aiming = false;
                      }
                    },
                  );
                }
                UIUpdate::TmpAppendSpace => sym_state.handle_buf_append_space(siv),
                UIUpdate::TmpAppend(idx) => sym_state.handle_buf_append(siv, idx),
                UIUpdate::RplCycle(old_area) => sym_state.handle_rpl_cycle(siv, old_area),
              }
            }
          }))
          .unwrap();
      })
      .expect("Failed to spawn UI batch processor thread");
  }

  #[inline]
  pub(crate) fn enqueue_sym_space(&self) {
    self
      .ui_update_queue
      .lock()
      .unwrap()
      .push_back(UIUpdate::TmpAppendSpace);
  }

  #[inline]
  pub(crate) fn enqueue_sym_buf_append(&self, idx: usize) {
    self
      .ui_update_queue
      .lock()
      .unwrap()
      .push_back(UIUpdate::TmpAppend(idx));
  }

  #[inline]
  pub(crate) fn enqueue_sym_rpl_cycle(&self, area: Rect) {
    self
      .ui_update_queue
      .lock()
      .unwrap()
      .push_back(UIUpdate::RplCycle(area));
  }
}
