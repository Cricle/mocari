use crate::runtime::ModelRuntime;

const PARAM_MOUTH_OPEN_Y: &str = "ParamMouthOpenY";

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
///
/// The caller provides amplitude values from external audio analysis via
/// [`set_amplitude`](Self::set_amplitude). LipSync applies exponential
/// smoothing and writes to `ParamMouthOpenY`.
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
    pub fn tick(&mut self, delta_seconds: f32) {
        let dt = delta_seconds.max(0.0);
        let factor = 1.0 - (-dt / self.config.smoothing.max(0.001)).exp();
        self.current_amplitude += (self.target_amplitude - self.current_amplitude) * factor;
    }

    /// Applies current mouth values to the runtime.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        let weight = self.config.weight;
        if weight <= 0.0 {
            return;
        }

        if let Some(current) = runtime.parameter_value(PARAM_MOUTH_OPEN_Y) {
            let value = self.current_amplitude;
            let blended = current + (value - current) * weight;
            runtime.set_parameter(PARAM_MOUTH_OPEN_Y, blended);
        }
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
