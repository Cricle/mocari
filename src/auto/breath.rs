use crate::runtime::ModelRuntime;

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
    pub fn tick(&mut self, _delta_seconds: f32) {
        // TODO: Task 4
    }

    /// Applies current breath values to the runtime.
    pub fn apply(&self, _runtime: &mut ModelRuntime) {
        // TODO: Task 4
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
