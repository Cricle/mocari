use std::f32::consts::TAU;

use crate::runtime::ModelRuntime;

const PARAM_BREATH: &str = "ParamBreath";

/// Configuration for breathing animation.
#[derive(Debug, Clone, PartialEq)]
pub struct BreathConfig {
    /// Breaths per second (default 0.25 = 4 second cycle).
    pub cycle_speed: f32,
    /// Blend weight 0.0-1.0.
    pub weight: f32,
}

impl Default for BreathConfig {
    fn default() -> Self {
        Self {
            cycle_speed: 0.25,
            weight: 1.0,
        }
    }
}

/// Subtle breathing animation using a sine wave.
///
/// Oscillates `ParamBreath` using a sine wave at the configured cycle speed.
#[derive(Debug, Clone)]
pub struct Breath {
    config: BreathConfig,
    phase: f32,
}

impl Breath {
    /// Creates a new breath with the given configuration.
    pub fn new(config: BreathConfig) -> Self {
        Self { config, phase: 0.0 }
    }

    /// Creates a new breath with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(BreathConfig::default())
    }

    /// Advances the breath phase by `delta_seconds`.
    pub fn tick(&mut self, delta_seconds: f32) {
        let dt = delta_seconds.max(0.0);
        self.phase += dt * self.config.cycle_speed * TAU;
        self.phase %= TAU;
    }

    /// Applies current breath values to the runtime.
    ///
    /// The sine wave output is mapped from [-1, 1] to [0, 1] so the parameter
    /// oscillates between its minimum and maximum.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        let weight = self.config.weight;
        if weight <= 0.0 {
            return;
        }

        // Map sine [-1, 1] to [0, 1]
        let breath_value = (self.phase.sin() + 1.0) * 0.5;

        if let Some(current) = runtime.parameter_value(PARAM_BREATH) {
            let blended = current + (breath_value - current) * weight;
            runtime.set_parameter(PARAM_BREATH, blended);
        }
    }

    /// Resets the breath phase to zero.
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }

    /// Sets the blend weight, clamped to 0.0..=1.0.
    pub fn set_weight(&mut self, weight: f32) {
        self.config.weight = weight.clamp(0.0, 1.0);
    }
}
