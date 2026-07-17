use crate::runtime::ModelRuntime;

/// Configuration for mouse/cursor tracking.
#[derive(Debug, Clone, PartialEq)]
pub struct MouseTrackerConfig {
    /// Exponential smoothing factor for position changes.
    pub smoothing: f32,
    /// Blend weight 0.0-1.0.
    pub weight: f32,
}

impl Default for MouseTrackerConfig {
    fn default() -> Self {
        Self {
            smoothing: 0.15,
            weight: 1.0,
        }
    }
}

/// Face follows cursor position with smooth interpolation.
///
/// Target coordinates are in normalized space: -1.0..=1.0 where (0,0) is center.
#[derive(Debug, Clone)]
pub struct MouseTracker {
    config: MouseTrackerConfig,
    target_x: f32,
    target_y: f32,
    current_x: f32,
    current_y: f32,
}

impl MouseTracker {
    /// Creates a new mouse tracker with the given configuration.
    pub fn new(config: MouseTrackerConfig) -> Self {
        Self {
            config,
            target_x: 0.0,
            target_y: 0.0,
            current_x: 0.0,
            current_y: 0.0,
        }
    }

    /// Creates a new mouse tracker with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(MouseTrackerConfig::default())
    }

    /// Sets the target position in normalized -1.0..=1.0 space.
    pub fn set_target(&mut self, x: f32, y: f32) {
        self.target_x = x.clamp(-1.0, 1.0);
        self.target_y = y.clamp(-1.0, 1.0);
    }

    /// Advances smoothing by `delta_seconds`.
    pub fn tick(&mut self, _delta_seconds: f32) {
        // TODO: Task 5
    }

    /// Applies current tracking values to the runtime.
    pub fn apply(&self, _runtime: &mut ModelRuntime) {
        // TODO: Task 5
    }

    /// Resets position to center.
    pub fn reset(&mut self) {
        self.target_x = 0.0;
        self.target_y = 0.0;
        self.current_x = 0.0;
        self.current_y = 0.0;
    }

    /// Sets the blend weight, clamped to 0.0..=1.0.
    pub fn set_weight(&mut self, weight: f32) {
        self.config.weight = weight.clamp(0.0, 1.0);
    }
}
