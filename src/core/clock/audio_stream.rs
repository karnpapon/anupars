use std::error::Error;
use std::f32::consts::PI;
use std::fmt::Debug;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Stream;
use crossbeam::atomic::AtomicCell;
use crossbeam::channel::{unbounded, Receiver, Sender};
use log::{error, info};

use super::dsp::{AudioGenerator, PatchBox};

pub const TWO_PI: f32 = 2.0 * PI;

#[derive(Debug)]
pub enum OutputStreamCommand<C> {
  Play,
  Pause,
  Stop,
  Terminate,
  Generator(C),
}

#[derive(Clone, Copy)]
pub struct OutputStreamConfig {
  sample_rate: u32,
}

pub struct OutputStreamManager<G>
where
  G: AudioGenerator + 'static,
{
  command_sender: Sender<OutputStreamCommand<G::Command>>,
  _output_thread: Option<JoinHandle<()>>,
}

impl<G> OutputStreamManager<G>
where
  G: AudioGenerator + 'static,
{
  pub fn new(gen: G, patches: Arc<PatchBox>) -> Self
  where
    G::Command: Send + Debug + 'static,
  {
    let (command_sender, command_receiver) = unbounded();

    let config = OutputStreamConfig { sample_rate: 48000 };

    let state = Arc::new(OutputStreamState::new());
    let state_clone = state.clone();

    let gen = Arc::new(gen);
    let gen_clone = gen.clone();

    let output_thread = thread::spawn(move || {
      Self::output_stream_thread(command_receiver, state_clone, config, gen_clone, patches)
    });

    OutputStreamManager {
      command_sender,
      _output_thread: Some(output_thread),
    }
  }

  pub fn send_command(&self, command: OutputStreamCommand<G::Command>) {
    if let Err(e) = self.command_sender.send(command) {
      error!("Failed to send audio command: {}", e);
    }
  }

  fn output_stream_thread(
    command_receiver: Receiver<OutputStreamCommand<G::Command>>,
    state: Arc<OutputStreamState>,
    config: OutputStreamConfig,
    generator: Arc<G>,
    patches: Arc<PatchBox>,
  ) where
    G::Command: Debug,
  {
    let device = match get_device() {
      Some(device) => device,
      None => {
        panic!("No audio devices found!");
      }
    };
    let cpal_config =
      get_output_stream_config(2, config.sample_rate, cpal::BufferSize::Default).unwrap();

    info!("Creating audio stream...");
    let state_clone = state.clone();
    let generator_clone = generator.clone();
    let stream =
      create_audio_stream(&device, &cpal_config, state_clone, generator_clone, patches).unwrap();
    info!("Audio stream created successfully.");

    while let Ok(command) = command_receiver.recv() {
      match command {
        OutputStreamCommand::Play => {
          state.play();
          info!("Playing audio stream...");
        }
        OutputStreamCommand::Pause => {
          state.stop();
          info!("Pausing audio stream...");
        }
        OutputStreamCommand::Stop => {
          state.stop();
          info!("Audio stream paused.");
        }
        OutputStreamCommand::Terminate => {
          let _ = stream.pause();
          state.stop();
          info!("Audio Thread terminating...");
          break;
        }
        OutputStreamCommand::Generator(command) => {
          info!("Command passed to audio generator: {:?}", command);
          generator.process_command(command);
        }
      }
    }
  }
}

impl<G> Drop for OutputStreamManager<G>
where
  G: AudioGenerator + 'static,
{
  fn drop(&mut self) {
    self.send_command(OutputStreamCommand::Terminate);
    if let Some(output_thread) = self._output_thread.take() {
      if let Err(e) = output_thread.join() {
        error!("Failed to join audio thread: {:?}", e);
      } else {
        info!("Audio thread terminated.");
      }
    }
  }
}

pub struct OutputStreamState {
  volume: AtomicCell<f32>,
  playing: AtomicBool,
}

impl OutputStreamState {
  pub fn new() -> Self {
    Self {
      volume: AtomicCell::new(0.5),
      playing: AtomicBool::new(false),
    }
  }

  pub fn set_volume(&self, vol: f32) {
    self.volume.store(vol);
  }

  pub fn volume(&self) -> f32 {
    self.volume.load()
  }

  pub fn play(&self) {
    self.playing.store(true, Ordering::Relaxed);
  }

  pub fn stop(&self) {
    self.playing.store(false, Ordering::Relaxed);
  }

  pub fn is_playing(&self) -> bool {
    self.playing.load(Ordering::Relaxed)
  }
}

fn get_device() -> Option<cpal::Device> {
  let host = cpal::default_host();
  host.default_output_device()
}

fn get_output_stream_config(
  channels: u16,
  sample_rate: u32,
  buffer_size: cpal::BufferSize,
) -> Option<cpal::StreamConfig> {
  Some(cpal::StreamConfig {
    sample_rate: cpal::SampleRate(sample_rate),
    buffer_size,
    channels,
  })
}

fn create_audio_stream<G>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  state: Arc<OutputStreamState>,
  generator: Arc<G>,
  patches: Arc<PatchBox>,
) -> Result<Stream, Box<dyn Error>>
where
  G: AudioGenerator + 'static,
{
  let state_clone = state.clone();
  let stream = device.build_output_stream(
    config,
    move |data: &mut [f32], info: &cpal::OutputCallbackInfo| {
      if !state.is_playing() {
        for sample in data.iter_mut() {
          *sample = 0.0;
        }
        return;
      }
      println!("playing");
      let volume = state_clone.volume();
      for sample in data.iter_mut() {
        *sample = volume * generator.next_sample();
      }

      patches.process(data);
    },
    move |err| {
      error!("An error occurred on the audio stream: {}", err);
    },
    None,
  )?;
  let _ = stream.play();
  Ok(stream)
}
