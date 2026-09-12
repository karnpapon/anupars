use std::f32::consts::PI;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum EasingMode {
  #[default]
  None,
  EaseIn,
  EaseOut,
  EaseInOut,
}

impl EasingMode {
  pub fn cycle_next(self) -> Self {
    match self {
      EasingMode::None => EasingMode::EaseIn,
      EasingMode::EaseIn => EasingMode::EaseOut,
      EasingMode::EaseOut => EasingMode::EaseInOut,
      EasingMode::EaseInOut => EasingMode::None,
    }
  }

  pub fn label(self) -> &'static str {
    match self {
      EasingMode::None => "",
      EasingMode::EaseIn => "ei",
      EasingMode::EaseOut => "eo",
      EasingMode::EaseInOut => "eio",
    }
  }

  // Apply easing to compute an effective tick divider.
  // phase: normalized position within the current playhead cycle (0.0..1.0).
  // div_denom: current DIV denominator from the ratio setting (ratio.1).
  // Returns Some(divider) when easing is active, None when it has no effect.
  //
  // Interpolation range (as DIV denominator values):
  //   fast = min(div_denom * 2, 64)  - 2x the current speed, capped at max DIV=64
  //   slow = div_denom.clamp(2, 32)  - current speed, capped when fast is already at ceiling
  //
  // eg. DIV=16 -> [slow=16, fast=32], DIV=64 -> [slow=32, fast=64]
  pub fn apply(self, phase: f32, div_denom: usize, going_forward: bool) -> Option<usize> {
    if matches!(self, EasingMode::None) || div_denom == 0 {
      return None;
    }
    let phase = match self {
      EasingMode::EaseInOut => phase,
      _ => {
        if going_forward {
          phase
        } else {
          1.0 - phase
        }
      }
    };
    let phase = phase.clamp(0.0, 1.0);
    let fast = ((div_denom * 2).min(64)) as f32;
    let slow = div_denom.clamp(2, 32) as f32;

    let effective_denom = match self {
      EasingMode::None => return None,
      EasingMode::EaseIn => {
        let t = phase * phase;
        slow + t * (fast - slow)
      }
      EasingMode::EaseOut => {
        let t = 1.0 - (1.0 - phase) * (1.0 - phase);
        fast + t * (slow - fast)
      }
      // squared sine narrows the fast span
      EasingMode::EaseInOut => {
        let t = (phase * PI).sin().powi(3);
        slow + t * (fast - slow)
      }
    };

    let denom = (effective_denom.round() as usize).max(1);
    Some((64 / denom).max(1))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn none_mode_never_applies() {
    for div_denom in [0, 1, 16, 64] {
      for phase in [0.0, 0.5, 1.0] {
        for going_forward in [true, false] {
          assert_eq!(
            EasingMode::None.apply(phase, div_denom, going_forward),
            None
          );
        }
      }
    }
  }

  #[test]
  fn zero_div_denom_never_applies() {
    for mode in [
      EasingMode::EaseIn,
      EasingMode::EaseOut,
      EasingMode::EaseInOut,
    ] {
      assert_eq!(mode.apply(0.5, 0, true), None);
    }
  }

  #[test]
  fn active_modes_stay_within_divider_bounds() {
    for mode in [
      EasingMode::EaseIn,
      EasingMode::EaseOut,
      EasingMode::EaseInOut,
    ] {
      for div_denom in [1usize, 2, 3, 8, 16, 32, 63, 64, 100] {
        for i in 0..=4 {
          let phase = i as f32 / 4.0;
          for going_forward in [true, false] {
            if let Some(divider) = mode.apply(phase, div_denom, going_forward) {
              assert!(
                (1..=64).contains(&divider),
                "{mode:?} div_denom={div_denom} phase={phase} -> {divider}"
              );
            }
          }
        }
      }
    }
  }

  #[test]
  fn ease_in_out_is_symmetric_around_midpoint() {
    for div_denom in [1usize, 4, 16, 32, 64] {
      for (a, b) in [(0.0, 1.0), (0.25, 0.75), (0.1, 0.9)] {
        assert_eq!(
          EasingMode::EaseInOut.apply(a, div_denom, true),
          EasingMode::EaseInOut.apply(b, div_denom, true),
          "div_denom={div_denom} phases=({a},{b})"
        );
      }
    }
  }

  #[test]
  fn cycle_next_returns_to_start_after_full_loop() {
    for start in [
      EasingMode::None,
      EasingMode::EaseIn,
      EasingMode::EaseOut,
      EasingMode::EaseInOut,
    ] {
      let mut mode = start;
      for _ in 0..4 {
        mode = mode.cycle_next();
      }
      assert_eq!(mode, start);
    }
  }
}
