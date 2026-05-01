#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ModSource {
  /// Playhead progress through its area, normalized to 0..1.
  /// Shape depends on movement mode: ramp (Fwd), inverse ramp (Rev),
  /// triangle (Pendulum), stepped random (Random).
  MovementPhase,
  /// Where the playhead area is parked on the grid, normalized to 0..1.
  /// Changes only when the user moves the area, not on every step.
  PlayheadAnchorX,
  /// Bar counter cycling over BAR_COUNT_PERIOD, normalized to 0..1.
  BarCount,
}

/// BAR_COUNT_PERIOD defines how many beats BarCount takes to complete one 0..1 cycle.
/// 4 = one bar (4/4), 2 = bar/2, 1 = bar/4 (one beat).
pub const BAR_COUNT_PERIOD: usize = 8;

impl ModSource {
  pub fn label(self) -> &'static str {
    match self {
      ModSource::MovementPhase => "Phs:",
      ModSource::PlayheadAnchorX => "AnX:",
      ModSource::BarCount => "Bar:",
    }
  }

  pub const ALL: [ModSource; 3] = [
    ModSource::MovementPhase,
    ModSource::PlayheadAnchorX,
    ModSource::BarCount,
  ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiceDest {
  /// Which die face is active, maps 0..1 to face 1..6.
  Face,
  /// Dot advance direction: 0..0.33 = -1, 0.33..0.67 = 0 (hold), 0.67..1.0 = +1.
  StepDir,
  /// Bars per dot advance, maps 0..1 to 1..16.
  BarsDiv,
}

impl DiceDest {
  pub fn label(self) -> &'static str {
    match self {
      DiceDest::Face => " Face ",
      DiceDest::StepDir => " Step ",
      DiceDest::BarsDiv => " Bars ",
    }
  }

  pub const ALL: [DiceDest; 3] = [DiceDest::Face, DiceDest::StepDir, DiceDest::BarsDiv];
}

#[derive(Clone, Copy, Debug)]
pub struct ModRoute {
  pub source: ModSource,
  pub dest: DiceDest,
  /// Scales source value before mapping. Clamped to -1.0..1.0.
  /// Negative inverts: high source drives destination toward its minimum.
  pub amount: f32,
}

impl ModRoute {
  pub fn new(source: ModSource, dest: DiceDest, amount: f32) -> Self {
    Self {
      source,
      dest,
      amount: amount.clamp(-1.0, 1.0),
    }
  }
}

/// Pre-normalized source values, collected once per evaluation.
pub struct SourceValues {
  pub movement_phase: f32,
  pub playhead_anchor_x: f32,
  pub bar_count: f32,
}

impl SourceValues {
  pub fn get(&self, source: ModSource) -> f32 {
    match source {
      ModSource::MovementPhase => self.movement_phase,
      ModSource::PlayheadAnchorX => self.playhead_anchor_x,
      ModSource::BarCount => self.bar_count,
    }
  }
}

/// Resolved destination overrides. None means no active route for that dest.
pub struct ModOutput {
  pub face: Option<u8>,
  pub step_dir: Option<i8>,
  pub bars_div: Option<usize>,
}

#[derive(Clone, Default)]
pub struct ModMatrix {
  pub routes: Vec<ModRoute>,
}

impl ModMatrix {
  /// Evaluate all routes. Last route per destination wins.
  pub fn evaluate(&self, sources: &SourceValues) -> ModOutput {
    let mut face: Option<f32> = None;
    let mut step_dir: Option<f32> = None;
    let mut bars_div: Option<f32> = None;

    for route in &self.routes {
      let t = sources.get(route.source);
      // negative amount: invert, then clamp to 0..1
      let v = if route.amount >= 0.0 {
        (t * route.amount).clamp(0.0, 1.0)
      } else {
        ((1.0 - t) * (-route.amount)).clamp(0.0, 1.0)
      };

      match route.dest {
        DiceDest::Face => face = Some(face.unwrap_or(0.0) + v),
        DiceDest::StepDir => step_dir = Some(step_dir.unwrap_or(0.0) + v),
        DiceDest::BarsDiv => bars_div = Some(bars_div.unwrap_or(0.0) + v),
      }
    }

    ModOutput {
      face: face.map(|v| (1 + (v.clamp(0.0, 1.0) * 5.0) as u8).clamp(1, 6)),
      step_dir: step_dir.map(|v| {
        let v = v.clamp(0.0, 1.0);
        if v < 0.33 {
          -1
        } else if v < 0.67 {
          0
        } else {
          1
        }
      }),
      bars_div: bars_div.map(|v| (1 + (v.clamp(0.0, 1.0) * 15.0) as usize).max(1)),
    }
  }

  /// Return the amount for a given (source, dest) pair, or None if no route exists.
  pub fn get_amount(&self, source: ModSource, dest: DiceDest) -> Option<f32> {
    self
      .routes
      .iter()
      .rev()
      .find(|r| r.source == source && r.dest == dest)
      .map(|r| r.amount)
  }

  /// Set or replace a route. Passing amount = 0.0 removes the route.
  pub fn set_route(&mut self, source: ModSource, dest: DiceDest, amount: f32) {
    self
      .routes
      .retain(|r| !(r.source == source && r.dest == dest));
    if amount != 0.0 {
      self.routes.push(ModRoute::new(source, dest, amount));
    }
  }
}
