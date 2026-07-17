use crate::runtime::ModelRuntime;

/// Configuration for automatic eye blinking.
#[derive(Debug, Clone, PartialEq)]
pub struct EyeBlinkConfig {
    /// Minimum seconds between blinks.
    pub min_interval: f32,
    /// Maximum seconds between blinks.
    pub max_interval: f32,
    /// Seconds to close eyes during a blink.
    pub close_duration: f32,
    /// Seconds to open eyes after a blink.
    pub open_duration: f32,
    /// Blend weight 0.0-1.0.
    pub weight: f32,
}

impl Default for EyeBlinkConfig {
    fn default() -> Self {
        Self {
            min_interval: 2.5,
            max_interval: 6.0,
            close_duration: 0.1,
            open_duration: 0.15,
            weight: 1.0,
        }
    }
}

/// Automatic eye blinking with randomized timing.
#[derive(Debug, Clone)]
pub struct EyeBlink {
    config: EyeBlinkConfig,
}

impl EyeBlink {
    /// Creates a new eye blink with the given configuration.
    pub fn new(config: EyeBlinkConfig) -> Self {
        Self { config }
    }

    /// Creates a new eye blink with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(EyeBlinkConfig::default())
    }

    /// Advances the blink state machine by `delta_seconds`.
    pub fn tick(&mut self, _delta_seconds: f32) {
        // TODO: Task 2
    }

    /// Applies current blink values to the runtime.
    pub fn apply(&self, _runtime: &mut ModelRuntime) {
        // TODO: Task 2
    }

    /// Resets the blink state machine to initial state.
    pub fn reset(&mut self) {
        // TODO: Task 2
    }

    /// Sets the blend weight, clamped to 0.0..=1.0.
    pub fn set_weight(&mut self, weight: f32) {
        self.config.weight = weight.clamp(0.0, 1.0);
    }
}
