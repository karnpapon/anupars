use std::f32::consts::PI;
use std::sync::{Arc, RwLock};

use crossbeam::atomic::AtomicCell;

const TWO_PI: f32 = 2.0 * PI;

pub trait AudioGenerator: Send + Sync {
  type Command;

  fn next_sample(&self) -> f32;

  fn process_command(&self, command: Self::Command);
}

pub trait DspProcessor: Send + Sync {
  fn process(&self, data: &mut [f32]);
}

pub struct PatchBox {
  processors: RwLock<Vec<Arc<dyn DspProcessor>>>,
}

impl PatchBox {
  pub fn new() -> Self {
    PatchBox {
      processors: RwLock::new(Vec::new()),
    }
  }

  pub fn add_processor(&self, processor: Arc<dyn DspProcessor>) {
    let mut procs = self.processors.write().unwrap();
    procs.push(processor);
  }

  pub fn remove_processor(&self, index: usize) {
    let mut procs = self.processors.write().unwrap();
    if index < procs.len() {
      procs.remove(index);
    }
  }

  pub fn process(&self, data: &mut [f32]) {
    let procs = self.processors.read().unwrap();
    for processor in procs.iter() {
      processor.process(data);
    }
  }
}

pub struct LowPassFilter {
  cutoff_frequency: AtomicCell<f32>,
  sample_rate: f32,
  previous_output: AtomicCell<f32>,
}

impl LowPassFilter {
  pub fn new(cutoff_frequency: f32, sample_rate: f32) -> Self {
    LowPassFilter {
      cutoff_frequency: AtomicCell::new(cutoff_frequency),
      sample_rate,
      previous_output: AtomicCell::new(0.0),
    }
  }
}

impl DspProcessor for LowPassFilter {
  fn process(&self, data: &mut [f32]) {
    let cut_off = self.cutoff_frequency.load();
    let prev = self.previous_output.load();
    let alpha = 1.0 / (1.0 + (self.sample_rate / (TWO_PI * cut_off)));
    for sample in data.iter_mut() {
      let output = alpha * *sample + (1.0 - alpha) * prev;
      self.previous_output.store(output);
      *sample = output;
    }
  }
}
