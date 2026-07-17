use crate::runtime::ModelRuntime;

/// Configuration for audio-driven lip sync.
#[derive(Debug, Clone, PartialEq)]
pub struct LipSyncConfig {
    /// Exponential smoothing factor for amplitude changes.
    pub smoothing: f32,
    /// Blend weight 0.0-1.0.
    pub weight: f32,
}

impl Default for LipSyncConfig {
    fn default() -> Self {
        Self {
            smoothing: 0.2,
            weight: 1.0,
        }
    }
}

/// Audio-driven mouth movement.
#[derive(Debug, Clone)]
pub struct LipSync {
    config: LipSyncConfig,
    target_amplitude: f32,
    current_amplitude: f32,
}

impl LipSync {
    /// Creates a new lip sync with the given configuration.
    pub fn new(config: LipSyncConfig) -> Self {
        Self {
            config,
            target_amplitude: 0.0,
            current_amplitude: 0.0,
        }
    }

    /// Creates a new lip sync with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(LipSyncConfig::default())
    }

    /// Sets the target amplitude from external audio analysis, clamped to 0.0..=1.0.
    pub fn set_amplitude(&mut self, amplitude: f32) {
        self.target_amplitude = amplitude.clamp(0.0, 1.0);
    }

    /// Advances smoothing by `delta_seconds`.
    pub fn tick(&mut self, _delta_seconds: f32) {
        // TODO: Task 3
    }

    /// Applies current mouth values to the runtime.
    pub fn apply(&self, _runtime: &mut ModelRuntime) {
        // TODO: Task 3
    }

    /// Resets amplitude to zero.
    pub fn reset(&mut self) {
        self.target_amplitude = 0.0;
        self.current_amplitude = 0.0;
    }

    /// Sets the blend weight, clamped to 0.0..=1.0.
    pub fn set_weight(&mut self, weight: f32) {
        self.config.weight = weight.clamp(0.0, 1.0);
    }
}
