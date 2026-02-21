use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(debug_assertions)]
pub struct TimingStats {
  min_micros: AtomicU64,
  max_micros: AtomicU64,
  total_micros: AtomicU64,
  count: AtomicU64,
  over_threshold_count: AtomicU64, // Count of calls > 500μs
}

#[cfg(debug_assertions)]
impl TimingStats {
  pub fn new() -> Self {
    Self {
      min_micros: AtomicU64::new(u64::MAX),
      max_micros: AtomicU64::new(0),
      total_micros: AtomicU64::new(0),
      count: AtomicU64::new(0),
      over_threshold_count: AtomicU64::new(0),
    }
  }

  pub fn record(&self, micros: u64) {
    // Update min (compare-and-swap loop)
    let mut current_min = self.min_micros.load(Ordering::Relaxed);
    while micros < current_min {
      match self.min_micros.compare_exchange(
        current_min,
        micros,
        Ordering::Relaxed,
        Ordering::Relaxed,
      ) {
        Ok(_) => break,
        Err(x) => current_min = x,
      }
    }

    // Update max
    let mut current_max = self.max_micros.load(Ordering::Relaxed);
    while micros > current_max {
      match self.max_micros.compare_exchange(
        current_max,
        micros,
        Ordering::Relaxed,
        Ordering::Relaxed,
      ) {
        Ok(_) => break,
        Err(x) => current_max = x,
      }
    }

    self.total_micros.fetch_add(micros, Ordering::Relaxed);
    self.count.fetch_add(1, Ordering::Relaxed);

    if micros > 500 {
      // Threshold: 500μs = 0.5ms
      self.over_threshold_count.fetch_add(1, Ordering::Relaxed);
    }
  }

  #[cfg(debug_assertions)]
  fn log_to_file_block(msg: &str) {
    use std::fs::OpenOptions;
    use std::io::Write;
    let mut file = OpenOptions::new()
      .create(true)
      .append(true)
      .open("./timing_debug.log")
      .unwrap();
    write!(file, "{}", msg).unwrap();
  }

  pub fn print_stats(&self) {
    let count = self.count.load(Ordering::Relaxed);
    if count == 0 {
      Self::log_to_file_block("No timing data collected\n");
      return;
    }

    let min = self.min_micros.load(Ordering::Relaxed);
    let max = self.max_micros.load(Ordering::Relaxed);
    let total = self.total_micros.load(Ordering::Relaxed);
    let avg = total / count;
    let over = self.over_threshold_count.load(Ordering::Relaxed);
    let over_pct = (over * 100) / count;

    let block = format!(
      "========== SetActivePos Timing Statistics ==========\n\
    Calls:        {}\n\
    Min:          {}μs ({:.2}ms)\n\
    Max:          {}μs ({:.2}ms)    ← This is your jitter!\n\
    Avg:          {}μs ({:.2}ms)\n\
    Over 500μs:   {} ({}%)           ← {}% of calls are too slow\n\
    ============================================\n\n",
      count,
      min,
      min as f64 / 1000.0,
      max,
      max as f64 / 1000.0,
      avg,
      avg as f64 / 1000.0,
      over,
      over_pct,
      over_pct
    );
    Self::log_to_file_block(&block);
  }

  pub fn reset(&self) {
    self.min_micros.store(u64::MAX, Ordering::Relaxed);
    self.max_micros.store(0, Ordering::Relaxed);
    self.total_micros.store(0, Ordering::Relaxed);
    self.count.store(0, Ordering::Relaxed);
    self.over_threshold_count.store(0, Ordering::Relaxed);
  }
}
