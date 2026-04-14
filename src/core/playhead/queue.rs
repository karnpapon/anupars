use arrayvec::ArrayVec;
use ringbuffer::{ConstGenericRingBuffer, RingBuffer};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::core::consts;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum QueueOperator {
  Push,      // P
  Swap,      // S
  Pop,       // O
  Duplicate, // D
}

impl fmt::Display for QueueOperator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      QueueOperator::Push => write!(f, "P"),
      QueueOperator::Swap => write!(f, "S"),
      QueueOperator::Pop => write!(f, "O"),
      QueueOperator::Duplicate => write!(f, "D"),
    }
  }
}

pub const QUEUE_OPERATORS: [QueueOperator; 4] = [
  QueueOperator::Push,
  QueueOperator::Swap,
  QueueOperator::Pop,
  QueueOperator::Duplicate,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EventOperator {
  R,
  C,
  H,
}

impl fmt::Display for EventOperator {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EventOperator::R => write!(f, "r"),
      EventOperator::C => write!(f, "c"),
      EventOperator::H => write!(f, "h"),
    }
  }
}

impl EventOperator {
  pub fn get_event_name(&self) -> &'static str {
    match self {
      EventOperator::R => ">RTCHT",
      EventOperator::C => ">CHORD",
      EventOperator::H => ">HOLDN",
    }
  }
}

pub const EVENT_OPERATORS: [EventOperator; 3] =
  [EventOperator::R, EventOperator::C, EventOperator::H];

/// Two-stage pending jump position:
/// - `Waiting`, position was just popped; the *current* jump will still be random,
///   and it will be promoted to `Armed` afterwards.
/// - `Armed`  , the *next* jump will land here.
#[derive(Clone, Debug)]
pub enum PendingJumpPosition {
  Empty,
  Waiting(usize, usize),
  Armed(usize, usize),
}

/// Queue item that can be either a position or an event
#[derive(Clone, Debug, PartialEq)]
pub enum QueueItem {
  Position(usize, usize),
  Event(EventOperator),
}

impl fmt::Display for QueueItem {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      QueueItem::Position(x, y) => write!(f, "{},{}", x, y),
      QueueItem::Event(op) => write!(f, "{}", op.get_event_name()),
    }
  }
}
/// Manages operator queue, event queue, and pushed positions
pub struct QueueManager {
  pub operator_queue: Arc<Mutex<ArrayVec<QueueItem, { consts::OP_QUEUE_CAPACITY }>>>,
  pub event_queue:
    Arc<Mutex<ConstGenericRingBuffer<EventOperator, { consts::EVENT_QUEUE_CAPACITY }>>>,
  pub pushed_positions: Arc<Mutex<HashMap<(usize, usize), bool>>>,
  pub pending_jump_position: Arc<Mutex<PendingJumpPosition>>,
  drain_queue_mode: AtomicBool,
}

impl Default for QueueManager {
  fn default() -> Self {
    Self::new()
  }
}

impl QueueManager {
  pub fn new() -> Self {
    QueueManager {
      operator_queue: Arc::new(Mutex::new(ArrayVec::new())),
      event_queue: Arc::new(Mutex::new(ConstGenericRingBuffer::new())),
      pushed_positions: Arc::new(Mutex::new(HashMap::new())),
      pending_jump_position: Arc::new(Mutex::new(PendingJumpPosition::Empty)),
      drain_queue_mode: AtomicBool::new(false),
    }
  }

  pub fn set_drain_queue_mode(&self, enabled: bool) {
    self.drain_queue_mode.store(enabled, Ordering::Relaxed);
  }

  pub fn is_drain_queue_mode(&self) -> bool {
    self.drain_queue_mode.load(Ordering::Relaxed)
  }

  pub fn clear_all(&self) {
    self.operator_queue.lock().unwrap().clear();
    self.event_queue.lock().unwrap().clear();
    self.pushed_positions.lock().unwrap().clear();
    *self.pending_jump_position.lock().unwrap() = PendingJumpPosition::Empty;
  }

  pub fn check_and_execute_operators(&self, abs_x: usize, event_operator_mode: bool) {
    let is_space = abs_x.is_multiple_of(consts::EVENT_OP_SPACING);

    if abs_x.is_multiple_of(consts::QUEUE_OP_SPACING) && !is_space {
      let position_index = abs_x / consts::QUEUE_OP_SPACING;
      self.execute_queue_operator(position_index);
    }

    if is_space && event_operator_mode {
      let position_index = abs_x / consts::EVENT_OP_SPACING;
      self.execute_event_operator(position_index);
    }
  }

  fn execute_queue_operator(&self, position_index: usize) {
    let operator_index = position_index % QUEUE_OPERATORS.len();
    let operator = QUEUE_OPERATORS[operator_index];

    let is_drain = self.is_drain_queue_mode();

    match operator {
      QueueOperator::Push => {
        if !is_drain {
          // Push is handled externally with current playhead position
        }
      }
      QueueOperator::Swap => {
        if !is_drain {
          self.handle_swap();
        }
      }
      QueueOperator::Pop => {
        // Pop is always allowed, even when drain mode is active
        self.handle_pop();
      }
      QueueOperator::Duplicate => {
        if !is_drain {
          self.handle_duplicate();
        }
      }
    }
  }

  fn execute_event_operator(&self, position_index: usize) {
    let operator_index = position_index % EVENT_OPERATORS.len();
    let operator = EVENT_OPERATORS[operator_index];

    match operator {
      EventOperator::R => self.handle_r(),
      EventOperator::C => self.handle_c(),
      EventOperator::H => self.handle_h(),
    }
  }

  pub fn handle_push(&self, current_pos: (usize, usize)) {
    // Check event queue first (FIFO)
    let mut event_queue = self.event_queue.lock().unwrap();
    let event_op = event_queue.dequeue();

    if let Some(event_op) = event_op {
      drop(event_queue);

      // Push the event to the main queue
      let mut queue = self.operator_queue.lock().unwrap();
      if queue.len() < queue.capacity() {
        queue.push(QueueItem::Event(event_op));
      }
      drop(queue);
    } else {
      drop(event_queue);

      // No events in queue, push current playhead position
      let mut pushed = self.pushed_positions.lock().unwrap();
      if let Entry::Vacant(e) = pushed.entry(current_pos) {
        e.insert(true);
        drop(pushed);

        let mut queue = self.operator_queue.lock().unwrap();
        if queue.len() < queue.capacity() {
          queue.push(QueueItem::Position(current_pos.0, current_pos.1));
        }
        drop(queue);
      }
    }
  }

  fn handle_swap(&self) {
    let mut queue = self.operator_queue.lock().unwrap();
    let len = queue.len();
    if len >= 2 {
      queue.swap(len - 1, len - 2);
    }
    drop(queue);
  }

  fn handle_pop(&self) {
    let mut queue = self.operator_queue.lock().unwrap();
    if !queue.is_empty() {
      let item = queue.remove(0);
      drop(queue);

      // Only remove from pushed_positions if it was a position
      if let QueueItem::Position(x, y) = item {
        let mut pushed = self.pushed_positions.lock().unwrap();
        pushed.remove(&(x, y));
        drop(pushed);
      }
    }
  }

  fn handle_duplicate(&self) {
    let mut queue = self.operator_queue.lock().unwrap();
    if let Some(item) = queue.last().cloned() {
      if queue.len() < queue.capacity() {
        queue.push(item);
      }
    }
    drop(queue);
  }

  fn handle_r(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.enqueue(EventOperator::R);
    drop(event_queue);
  }

  fn handle_h(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.enqueue(EventOperator::H);
    drop(event_queue);
  }

  fn handle_c(&self) {
    let mut event_queue = self.event_queue.lock().unwrap();
    event_queue.enqueue(EventOperator::C);
    drop(event_queue);
  }

  pub fn get_front_item(&self) -> Option<QueueItem> {
    self.operator_queue.lock().unwrap().first().cloned()
  }

  pub fn remove_front_item(&self) -> Option<QueueItem> {
    let mut queue = self.operator_queue.lock().unwrap();
    if queue.is_empty() {
      None
    } else {
      Some(queue.remove(0))
    }
  }

  pub fn peek_pending_jump(&self) -> PendingJumpPosition {
    self.pending_jump_position.lock().unwrap().clone()
  }

  pub fn set_pending_jump(&self, position: PendingJumpPosition) {
    *self.pending_jump_position.lock().unwrap() = position;
  }

  pub fn promote_waiting_to_armed(&self) {
    let mut pending = self.pending_jump_position.lock().unwrap();
    if let PendingJumpPosition::Waiting(x, y) = *pending {
      *pending = PendingJumpPosition::Armed(x, y);
    }
  }
}
