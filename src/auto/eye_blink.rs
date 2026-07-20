use crate::runtime::ModelRuntime;

const PARAM_EYE_L_OPEN: &str = "ParamEyeLOpen";
const PARAM_EYE_R_OPEN: &str = "ParamEyeROpen";

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
    /// Parameter indices to target. If empty, uses default ParamEyeLOpen/ParamEyeROpen.
    pub parameter_indices: Vec<usize>,
}

impl Default for EyeBlinkConfig {
    fn default() -> Self {
        Self {
            min_interval: 2.5,
            max_interval: 6.0,
            close_duration: 0.1,
            open_duration: 0.15,
            weight: 1.0,
            parameter_indices: Vec::new(),
        }
    }
}

impl EyeBlinkConfig {
    /// Creates a config targeting specific parameter indices.
    pub fn for_parameters(indices: Vec<usize>) -> Self {
        Self {
            parameter_indices: indices,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BlinkPhase {
    Open,
    Closing,
    Closed,
    Opening,
}

/// Automatic eye blinking with randomized timing.
#[derive(Debug, Clone)]
pub struct EyeBlink {
    config: EyeBlinkConfig,
    phase: BlinkPhase,
    timer: f32,
    next_interval: f32,
    blink_value: f32,
    rng_state: u64,
}

impl EyeBlink {
    /// Creates a new eye blink with the given configuration.
    pub fn new(config: EyeBlinkConfig) -> Self {
        let next_interval = Self::random_interval_from_state(config.min_interval, config.max_interval, 0x1234_5678_9abc_def0);
        Self {
            config,
            phase: BlinkPhase::Open,
            timer: 0.0,
            next_interval,
            blink_value: 1.0,
            rng_state: 0x1234_5678_9abc_def0,
        }
    }

    /// Creates a new eye blink with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(EyeBlinkConfig::default())
    }

    /// Advances the blink state machine by `delta_seconds`.
    pub fn tick(&mut self, delta_seconds: f32) {
        let dt = delta_seconds.max(0.0);
        self.timer += dt;

        static FIRST_TICK: std::sync::Once = std::sync::Once::new();
        FIRST_TICK.call_once(|| {
            eprintln!("[DEBUG] EyeBlink::tick() first call: next_interval={:.2}s", self.next_interval);
        });

        let old_phase = self.phase;
        let old_value = self.blink_value;

        match self.phase {
            BlinkPhase::Open => {
                if self.timer >= self.next_interval {
                    eprintln!("[DEBUG] EyeBlink: Starting blink! timer={:.2}, next_interval={:.2}",
                        self.timer, self.next_interval);
                    self.phase = BlinkPhase::Closing;
                    self.timer = 0.0;
                }
                self.blink_value = 1.0;
            }
            BlinkPhase::Closing => {
                let progress = (self.timer / self.config.close_duration).clamp(0.0, 1.0);
                self.blink_value = 1.0 - progress;
                if self.timer >= self.config.close_duration {
                    self.phase = BlinkPhase::Closed;
                    self.timer = 0.0;
                    self.blink_value = 0.0;
                }
            }
            BlinkPhase::Closed => {
                self.blink_value = 0.0;
                // Short closed hold (use a fraction of close duration)
                let hold = self.config.close_duration * 0.5;
                if self.timer >= hold {
                    self.phase = BlinkPhase::Opening;
                    self.timer = 0.0;
                }
            }
            BlinkPhase::Opening => {
                let progress = (self.timer / self.config.open_duration).clamp(0.0, 1.0);
                self.blink_value = progress;
                if self.timer >= self.config.open_duration {
                    self.phase = BlinkPhase::Open;
                    self.timer = 0.0;
                    self.blink_value = 1.0;
                    self.advance_rng();
                    self.next_interval = Self::random_interval_from_state(
                        self.config.min_interval,
                        self.config.max_interval,
                        self.rng_state,
                    );
                }
            }
        }
    }

    /// Applies current blink values to the runtime.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        let weight = self.config.weight;
        if weight <= 0.0 {
            return;
        }

        let value = self.blink_value;
        if self.config.parameter_indices.is_empty() {
            // Default behavior: target ParamEyeLOpen and ParamEyeROpen
            if let Some(index) = runtime.parameter_index(PARAM_EYE_L_OPEN) {
                let current = runtime.parameter_value_by_index(index).unwrap_or(1.0);
                let blended = current + (value - current) * weight;
                runtime.set_parameter_by_index(index, blended);
            }
            if let Some(index) = runtime.parameter_index(PARAM_EYE_R_OPEN) {
                let current = runtime.parameter_value_by_index(index).unwrap_or(1.0);
                let blended = current + (value - current) * weight;
                runtime.set_parameter_by_index(index, blended);
            }
        } else {
            for &index in &self.config.parameter_indices {
                let Some(current) = runtime.parameter_value_by_index(index) else {
                    continue;
                };
                let blended = current + (value - current) * weight;
                runtime.set_parameter_by_index(index, blended);
            }
        }
    }

    /// Resets the blink state machine to initial state.
    pub fn reset(&mut self) {
        self.phase = BlinkPhase::Open;
        self.timer = 0.0;
        self.blink_value = 1.0;
        self.advance_rng();
        self.next_interval = Self::random_interval_from_state(
            self.config.min_interval,
            self.config.max_interval,
            self.rng_state,
        );
    }

    /// Sets the blend weight, clamped to 0.0..=1.0.
    pub fn set_weight(&mut self, weight: f32) {
        self.config.weight = weight.clamp(0.0, 1.0);
    }

    fn advance_rng(&mut self) {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 7;
        self.rng_state ^= self.rng_state << 17;
    }

    fn random_interval_from_state(min: f32, max: f32, state: u64) -> f32 {
        let normalized = (state >> 40) as f32 / (1u64 << 24) as f32;
        min + (max - min) * normalized
    }
}
