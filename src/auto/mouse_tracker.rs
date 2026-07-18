use crate::runtime::ModelRuntime;

const PARAM_ANGLE_X: &str = "ParamAngleX";
const PARAM_ANGLE_Y: &str = "ParamAngleY";
const PARAM_BODY_ANGLE_X: &str = "ParamBodyAngleX";
const PARAM_EYE_BALL_X: &str = "ParamEyeBallX";
const PARAM_EYE_BALL_Y: &str = "ParamEyeBallY";

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
/// Target coordinates are in normalized space: `-1.0..=1.0` where `(0, 0)` is
/// center, `(-1, -1)` is bottom-left, `(1, 1)` is top-right. The caller is
/// responsible for converting from window coordinates to this normalized space.
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
    pub fn tick(&mut self, delta_seconds: f32) {
        let dt = delta_seconds.max(0.0);
        let factor = 1.0 - (-dt / self.config.smoothing.max(0.001)).exp();
        self.current_x += (self.target_x - self.current_x) * factor;
        self.current_y += (self.target_y - self.current_y) * factor;
    }

    /// Applies current tracking values to the runtime.
    ///
    /// Writes to head rotation (ParamAngleX/Y), body rotation (ParamBodyAngleX),
    /// and eye direction (ParamEyeBallX/Y). Each parameter is scaled by the
    /// configured weight and blended with the current value.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        let weight = self.config.weight;
        if weight <= 0.0 {
            return;
        }

        // Head rotation: full range
        self.apply_parameter(runtime, PARAM_ANGLE_X, self.current_x * 30.0, weight);
        self.apply_parameter(runtime, PARAM_ANGLE_Y, self.current_y * 30.0, weight);

        // Body rotation: reduced range
        self.apply_parameter(runtime, PARAM_BODY_ANGLE_X, self.current_x * 10.0, weight);

        // Eye direction: full range
        self.apply_parameter(runtime, PARAM_EYE_BALL_X, self.current_x, weight);
        self.apply_parameter(runtime, PARAM_EYE_BALL_Y, self.current_y, weight);
    }

    /// Resets position to center.
    pub fn reset(&mut self) {
        self.target_x = 0.0;
        self.target_y = 0.0;
        self.current_x = 0.0;
        self.current_y = 0.0;
    }

    /// Returns whether the tracker has non-zero offset from center.
    pub fn is_active(&self) -> bool {
        self.current_x.abs() > 0.001
            || self.current_y.abs() > 0.001
            || self.target_x.abs() > 0.001
            || self.target_y.abs() > 0.001
    }

    /// Sets the blend weight, clamped to 0.0..=1.0.
    pub fn set_weight(&mut self, weight: f32) {
        self.config.weight = weight.clamp(0.0, 1.0);
    }

    fn apply_parameter(&self, runtime: &mut ModelRuntime, id: &str, target: f32, weight: f32) {
        if let Some(current) = runtime.parameter_value(id) {
            let blended = current + (target - current) * weight;
            runtime.set_parameter(id, blended);
        }
    }
}
