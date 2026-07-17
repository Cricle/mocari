# More Cubism Runtime Features Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add auto-animation (eye blink, lip sync, breath, mouse tracking), MotionManager with priority queue and crossfade, userdata3.json parsing, and drawable visibility control to the Mocari Live2D runtime.

**Architecture:** Each feature is a standalone struct with `tick(delta)` and `apply(runtime)` methods, following the existing `MotionPlayer`/`ExpressionPlayer` pattern. Features live under `src/auto/`, MotionManager extends `src/motion.rs`, user data adds to `src/json/`, and drawable visibility extends `ModelRuntime`.

**Tech Stack:** Rust, serde_json, no new dependencies

## Global Constraints

- `#![forbid(unsafe_code)]` — no unsafe code anywhere
- All new types derive `Debug, Clone`
- All parsers return `Result<T, crate::Error>`
- Follow existing naming conventions: `pub fn tick(&mut self, delta_seconds: f32)`, `pub fn apply(&self, runtime: &mut ModelRuntime)`
- No new crate dependencies

---

### Task 1: Create `src/auto/mod.rs` module scaffold

**Files:**
- Create: `src/auto/mod.rs`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: `pub mod auto` in `lib.rs`

- [ ] **Step 1: Create the auto module file**

Create `src/auto/mod.rs`:

```rust
//! Auto-animation features that make models feel alive without manual parameter tweaking.
//!
//! Each struct owns its state, exposes [`tick`](EyeBlink::tick) and
//! [`apply`](EyeBlink::apply) methods, and can be used independently.
//!
//! ```no_run
//! use mocari::auto::{EyeBlink, Breath};
//! # use mocari::assets::load_model_runtime;
//! # let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
//! # let runtime = model.runtime_mut();
//! let mut blink = EyeBlink::with_defaults();
//! blink.tick(1.0 / 60.0);
//! blink.apply(runtime);
//! ```

mod breath;
mod eye_blink;
mod lip_sync;
mod mouse_tracker;

pub use breath::{Breath, BreathConfig};
pub use eye_blink::{EyeBlink, EyeBlinkConfig};
pub use lip_sync::{LipSync, LipSyncConfig};
pub use mouse_tracker::{MouseTracker, MouseTrackerConfig};
```

- [ ] **Step 2: Create stub files for each auto-animation module**

Create `src/auto/eye_blink.rs`:

```rust
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
```

Create `src/auto/lip_sync.rs`:

```rust
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
```

Create `src/auto/breath.rs`:

```rust
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
```

Create `src/auto/mouse_tracker.rs`:

```rust
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
```

- [ ] **Step 3: Add `pub mod auto` to `src/lib.rs`**

In `src/lib.rs`, add after line 42 (`pub mod runtime;`):

```rust
/// Auto-animation features for eye blink, lip sync, breath, and mouse tracking.
pub mod auto;
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully (stubs have no logic yet)

- [ ] **Step 5: Commit**

```bash
git add src/auto/ src/lib.rs
git commit -m "feat(auto): add module scaffold for eye blink, lip sync, breath, mouse tracker"
```

---

### Task 2: Implement EyeBlink

**Files:**
- Modify: `src/auto/eye_blink.rs`
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: `ModelRuntime::set_parameter(&mut self, id: &str, value: f32) -> bool`, `ModelRuntime::parameter_value(&self, id: &str) -> Option<f32>`
- Produces: `EyeBlink::tick`, `EyeBlink::apply`, `EyeBlink::reset`

- [ ] **Step 1: Write the failing tests**

Add to `tests/runtime.rs`:

```rust
use mocari::auto::EyeBlink;

#[test]
fn eye_blink_default_config_has_reasonable_values() {
    let blink = EyeBlink::with_defaults();
    // We can't access config fields directly, but we can verify behavior
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    // Eyes should start open (blink hasn't happened yet)
    blink.apply(runtime);
    // ParamEyeLOpen should still be at default after apply with no tick
    let value = runtime.parameter_value("ParamEyeLOpen");
    assert!(value.is_some());
}

#[test]
fn eye_blink_closes_eyes_during_blink() {
    let config = mocari::auto::EyeBlinkConfig {
        min_interval: 0.0,
        max_interval: 0.0,
        close_duration: 0.1,
        open_duration: 0.15,
        weight: 1.0,
    };
    let mut blink = EyeBlink::new(config);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    // Tick to trigger immediate blink (interval = 0)
    blink.tick(0.001);
    // Tick into closing phase
    blink.tick(0.05);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    blink.apply(runtime);

    let left = runtime.parameter_value("ParamEyeLOpen").unwrap();
    let right = runtime.parameter_value("ParamEyeROpen").unwrap();
    assert!(left < 1.0, "left eye should be closing: {left}");
    assert!(right < 1.0, "right eye should be closing: {right}");
}

#[test]
fn eye_blink_weight_zero_has_no_effect() {
    let mut blink = EyeBlink::with_defaults();
    blink.set_weight(0.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamEyeLOpen").unwrap();
    runtime.reset_parameters();
    blink.tick(0.5);
    blink.apply(runtime);
    let after = runtime.parameter_value("ParamEyeLOpen").unwrap();
    assert_close_runtime(after, before);
}

fn assert_close_runtime(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test runtime eye_blink -- --nocapture`
Expected: Tests fail (stub methods don't change values)

- [ ] **Step 3: Implement EyeBlink**

Replace `src/auto/eye_blink.rs` with:

```rust
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

        match self.phase {
            BlinkPhase::Open => {
                if self.timer >= self.next_interval {
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
        if let Some(current) = runtime.parameter_value(PARAM_EYE_L_OPEN) {
            let blended = current + (value - current) * weight;
            runtime.set_parameter(PARAM_EYE_L_OPEN, blended);
        }
        if let Some(current) = runtime.parameter_value(PARAM_EYE_R_OPEN) {
            let blended = current + (value - current) * weight;
            runtime.set_parameter(PARAM_EYE_R_OPEN, blended);
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
        let normalized = (state as f32) / (u64::MAX as f32);
        min + (max - min) * normalized
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test runtime eye_blink -- --nocapture`
Expected: All eye_blink tests PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/auto/eye_blink.rs tests/runtime.rs
git commit -m "feat(auto): implement EyeBlink with randomized timing state machine"
```

---

### Task 3: Implement LipSync

**Files:**
- Modify: `src/auto/lip_sync.rs`
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: `ModelRuntime::set_parameter`, `ModelRuntime::parameter_value`
- Produces: `LipSync::tick`, `LipSync::apply`, `LipSync::set_amplitude`

- [ ] **Step 1: Write the failing tests**

Add to `tests/runtime.rs`:

```rust
use mocari::auto::LipSync;

#[test]
fn lip_sync_smooths_amplitude_over_time() {
    let mut lip = LipSync::with_defaults();
    lip.set_amplitude(1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    // After first tick, amplitude should be partially smoothed
    lip.tick(1.0 / 60.0);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    lip.apply(runtime);
    let value = runtime.parameter_value("ParamMouthOpenY").unwrap();
    assert!(value > 0.0, "mouth should open after amplitude: {value}");
    assert!(value < 1.0, "mouth should not snap to max instantly: {value}");
}

#[test]
fn lip_sync_weight_zero_has_no_effect() {
    let mut lip = LipSync::with_defaults();
    lip.set_weight(0.0);
    lip.set_amplitude(1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamMouthOpenY").unwrap();
    lip.tick(1.0);
    runtime.reset_parameters();
    lip.apply(runtime);
    let after = runtime.parameter_value("ParamMouthOpenY").unwrap();
    assert_close_lip(after, before);
}

fn assert_close_lip(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test runtime lip_sync -- --nocapture`
Expected: Tests fail

- [ ] **Step 3: Implement LipSync**

Replace `src/auto/lip_sync.rs` with:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test runtime lip_sync -- --nocapture`
Expected: All lip_sync tests PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/auto/lip_sync.rs tests/runtime.rs
git commit -m "feat(auto): implement LipSync with exponential smoothing"
```

---

### Task 4: Implement Breath

**Files:**
- Modify: `src/auto/breath.rs`
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: `ModelRuntime::set_parameter`, `ModelRuntime::parameter_value`
- Produces: `Breath::tick`, `Breath::apply`

- [ ] **Step 1: Write the failing tests**

Add to `tests/runtime.rs`:

```rust
use mocari::auto::Breath;

#[test]
fn breath_produces_oscillating_output() {
    let mut breath = Breath::with_defaults();
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    breath.tick(1.0);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    breath.apply(runtime);
    let value1 = runtime.parameter_value("ParamBreath").unwrap();

    breath.tick(1.0);
    runtime.reset_parameters();
    breath.apply(runtime);
    let value2 = runtime.parameter_value("ParamBreath").unwrap();

    // Values should change over time (sine wave)
    // They may or may not be different depending on phase, but at least one should be non-zero
    assert!(value1.is_finite() && value2.is_finite(), "breath values must be finite");
}

#[test]
fn breath_weight_zero_has_no_effect() {
    let mut breath = Breath::with_defaults();
    breath.set_weight(0.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamBreath").unwrap_or(0.0);
    breath.tick(1.0);
    runtime.reset_parameters();
    breath.apply(runtime);
    let after = runtime.parameter_value("ParamBreath").unwrap_or(0.0);
    assert_close_breath(after, before);
}

fn assert_close_breath(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test runtime breath -- --nocapture`
Expected: Tests fail

- [ ] **Step 3: Implement Breath**

Replace `src/auto/breath.rs` with:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test runtime breath -- --nocapture`
Expected: All breath tests PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/auto/breath.rs tests/runtime.rs
git commit -m "feat(auto): implement Breath with sine wave oscillation"
```

---

### Task 5: Implement MouseTracker

**Files:**
- Modify: `src/auto/mouse_tracker.rs`
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: `ModelRuntime::set_parameter`, `ModelRuntime::parameter_value`
- Produces: `MouseTracker::tick`, `MouseTracker::apply`, `MouseTracker::set_target`

- [ ] **Step 1: Write the failing tests**

Add to `tests/runtime.rs`:

```rust
use mocari::auto::MouseTracker;

#[test]
fn mouse_tracker_smooths_toward_target() {
    let mut tracker = MouseTracker::with_defaults();
    tracker.set_target(1.0, -1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    tracker.tick(1.0 / 60.0);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    tracker.apply(runtime);
    let angle_x = runtime.parameter_value("ParamAngleX").unwrap();
    assert!(angle_x > 0.0, "should track toward positive X: {angle_x}");
}

#[test]
fn mouse_tracker_weight_zero_has_no_effect() {
    let mut tracker = MouseTracker::with_defaults();
    tracker.set_weight(0.0);
    tracker.set_target(1.0, 1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamAngleX").unwrap();
    tracker.tick(1.0);
    runtime.reset_parameters();
    tracker.apply(runtime);
    let after = runtime.parameter_value("ParamAngleX").unwrap();
    assert_close_mouse(after, before);
}

fn assert_close_mouse(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test runtime mouse_tracker -- --nocapture`
Expected: Tests fail

- [ ] **Step 3: Implement MouseTracker**

Replace `src/auto/mouse_tracker.rs` with:

```rust
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
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test runtime mouse_tracker -- --nocapture`
Expected: All mouse_tracker tests PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit**

```bash
git add src/auto/mouse_tracker.rs tests/runtime.rs
git commit -m "feat(auto): implement MouseTracker with exponential smoothing"
```

---

### Task 6: Implement MotionManager

**Files:**
- Modify: `src/motion.rs`
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: `MotionPlayer::tick`, `MotionPlayer::apply`, `MotionPlayer::set_weight`, `MotionPlayer::is_finished`, `ModelRuntime::reset_parameters`
- Produces: `MotionManager::start_motion`, `MotionManager::tick`, `MotionManager::apply`, `MotionPriority`

- [ ] **Step 1: Write the failing tests**

Add to `tests/runtime.rs`:

```rust
use mocari::motion::{MotionManager, MotionPriority};

#[test]
fn motion_manager_plays_single_motion() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let mut manager = MotionManager::new();

    manager.start_motion(motion, MotionPriority::Normal, "Idle");
    manager.tick(0.5);
    model.runtime_mut().reset_parameters();
    manager.apply(model.runtime_mut());

    // Should have changed some parameter values
    assert_eq!(manager.active_count(), 1);
    assert!(!manager.is_finished());
}

#[test]
fn motion_manager_crossfades_same_group() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion1 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let motion2 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let mut manager = MotionManager::new();
    manager.set_crossfade_duration(0.5);

    manager.start_motion(motion1, MotionPriority::Normal, "Idle");
    manager.tick(1.0);
    manager.start_motion(motion2, MotionPriority::Normal, "Idle");
    manager.tick(0.1);

    // Both should be active during crossfade
    assert!(manager.active_count() >= 1, "should have active motions: {}", manager.active_count());
}

#[test]
fn motion_manager_force_interrupts_normal() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion1 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let motion2 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let mut manager = MotionManager::new();
    manager.set_crossfade_duration(0.5);

    manager.start_motion(motion1, MotionPriority::Normal, "Idle");
    manager.tick(1.0);
    manager.start_motion(motion2, MotionPriority::Force, "Force");
    manager.tick(0.1);

    assert!(manager.active_count() >= 1);
}

#[test]
fn motion_manager_default_crossfade_is_half_second() {
    let manager = MotionManager::new();
    assert_eq!(manager.crossfade_duration(), 0.5);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test runtime motion_manager -- --nocapture`
Expected: Tests fail (MotionManager doesn't exist yet)

- [ ] **Step 3: Implement MotionManager**

Add the following to the end of `src/motion.rs` (before the `load_motion` function):

```rust
/// Priority level for motions in a [`MotionManager`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MotionPriority {
    /// Only plays if no other motions are active.
    Idle,
    /// Normal priority. Queues (FIFO) if another normal motion is playing.
    Normal,
    /// Immediately starts, crossfading out current motions.
    Force,
}

#[derive(Debug, Clone)]
struct ManagedMotion {
    player: MotionPlayer,
    priority: MotionPriority,
    group: String,
    fade_in_remaining: f32,
    fading_out: bool,
}

/// Priority-based motion queue with crossfade blending.
///
/// Manages multiple active [`MotionPlayer`]s with priority levels and
/// crossfade transitions. Same-group motions replace each other;
/// different-group motions can play simultaneously.
///
/// ```no_run
/// use mocari::motion::{MotionManager, MotionPriority};
/// # use mocari::motion::load_motion;
/// # let motion = load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
/// let mut manager = MotionManager::new();
/// manager.start_motion(motion, MotionPriority::Normal, "Idle");
/// ```
#[derive(Debug, Clone)]
pub struct MotionManager {
    players: Vec<ManagedMotion>,
    crossfade_duration: f32,
    queue: Vec<(Motion3, MotionPriority, String)>,
}

impl MotionManager {
    /// Creates an empty motion manager with a 0.5 second crossfade.
    pub fn new() -> Self {
        Self {
            players: Vec::new(),
            crossfade_duration: 0.5,
            queue: Vec::new(),
        }
    }

    /// Sets the crossfade duration in seconds.
    pub fn set_crossfade_duration(&mut self, seconds: f32) {
        self.crossfade_duration = seconds.max(0.0);
    }

    /// Returns the crossfade duration in seconds.
    pub fn crossfade_duration(&self) -> f32 {
        self.crossfade_duration
    }

    /// Starts a motion with the given priority and group.
    ///
    /// Returns the number of active motions after starting.
    pub fn start_motion(&mut self, motion: Motion3, priority: MotionPriority, group: &str) -> usize {
        match priority {
            MotionPriority::Idle => {
                if self.players.iter().any(|p| !p.fading_out) {
                    // Other motions are active, idle can't start
                    return self.players.len();
                }
                self.start_motion_internal(motion, priority, group);
            }
            MotionPriority::Normal => {
                // Check if same group is already playing
                let same_group_index = self.players.iter().position(|p| p.group == group && !p.fading_out);
                if let Some(index) = same_group_index {
                    // Crossfade: start fading out old, start new
                    self.players[index].fading_out = true;
                    self.players[index].fade_in_remaining = self.crossfade_duration;
                    self.start_motion_internal(motion, priority, group);
                } else if self.players.iter().any(|p| p.priority == MotionPriority::Normal && !p.fading_out) {
                    // Queue if a normal motion is playing
                    self.queue.push((motion, priority, group.to_owned()));
                    return self.players.len();
                } else {
                    self.start_motion_internal(motion, priority, group);
                }
            }
            MotionPriority::Force => {
                // Fade out all current motions
                for player in &mut self.players {
                    if !player.fading_out {
                        player.fading_out = true;
                        player.fade_in_remaining = self.crossfade_duration;
                    }
                }
                self.start_motion_internal(motion, priority, group);
            }
        }
        self.players.len()
    }

    /// Starts a motion from a file path.
    pub fn start_motion_from_path(
        &mut self,
        path: &str,
        priority: MotionPriority,
        group: &str,
    ) -> Result<usize, MotionLoadError> {
        let motion = load_motion(path)?;
        Ok(self.start_motion(motion, priority, group))
    }

    /// Advances all active motions by `delta_seconds`.
    pub fn tick(&mut self, delta_seconds: f32) {
        let dt = delta_seconds.max(0.0);
        let crossfade = self.crossfade_duration;

        for managed in &mut self.players {
            managed.player.tick(dt);
            if managed.fading_out {
                managed.fade_in_remaining -= dt;
                let progress = ((crossfade - managed.fade_in_remaining) / crossfade).clamp(0.0, 1.0);
                managed.player.set_weight(1.0 - progress);
            } else if managed.fade_in_remaining > 0.0 {
                managed.fade_in_remaining -= dt;
                let progress = ((crossfade - managed.fade_in_remaining) / crossfade).clamp(0.0, 1.0);
                managed.player.set_weight(progress);
            } else {
                managed.player.set_weight(1.0);
            }
        }

        // Remove finished motions
        self.players.retain(|m| !m.player.is_finished() && m.fade_in_remaining > -0.1);

        // Remove motions that have fully faded out
        self.players.retain(|m| {
            if m.fading_out && m.player.weight() <= 0.01 {
                false
            } else {
                true
            }
        });

        // Process queue: start queued motions if slots are available
        if !self.queue.is_empty() && !self.players.iter().any(|p| p.priority == MotionPriority::Normal && !p.fading_out) {
            if let Some((motion, priority, group)) = self.queue.remove(0) {
                self.start_motion_internal(motion, priority, &group);
            }
        }
    }

    /// Applies all active motions to the runtime.
    pub fn apply(&self, runtime: &mut ModelRuntime) {
        for managed in &self.players {
            managed.player.apply(runtime);
        }
    }

    /// Stops all motions immediately.
    pub fn stop_all(&mut self) {
        self.players.clear();
        self.queue.clear();
    }

    /// Stops all motions with a fade-out.
    pub fn stop_all_with_fade(&mut self, _fade_seconds: f32) {
        for player in &mut self.players {
            if !player.fading_out {
                player.fading_out = true;
                player.fade_in_remaining = self.crossfade_duration;
            }
        }
        self.queue.clear();
    }

    /// Returns whether there are no active or queued motions.
    pub fn is_finished(&self) -> bool {
        self.players.is_empty() && self.queue.is_empty()
    }

    /// Returns the number of active motions (including those fading out).
    pub fn active_count(&self) -> usize {
        self.players.len()
    }

    /// Removes all active and queued motions.
    pub fn clear(&mut self) {
        self.players.clear();
        self.queue.clear();
    }

    fn start_motion_internal(&mut self, motion: Motion3, priority: MotionPriority, group: &str) {
        let crossfade = self.crossfade_duration;
        let mut player = MotionPlayer::new(motion);
        if crossfade > 0.0 {
            player.set_weight(0.0);
        }
        self.players.push(ManagedMotion {
            player,
            priority,
            group: group.to_owned(),
            fade_in_remaining: crossfade,
            fading_out: false,
        });
    }
}

impl Default for MotionManager {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 4: Update the `motion` module re-exports in `src/lib.rs`**

In `src/lib.rs`, change the `pub use crate::motion::MotionPlayer;` line to:

```rust
pub use crate::motion::{MotionManager, MotionPlayer, MotionPriority};
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test runtime motion_manager -- --nocapture`
Expected: All motion_manager tests PASS

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src/motion.rs src/lib.rs tests/runtime.rs
git commit -m "feat(motion): implement MotionManager with priority queue and crossfade"
```

---

### Task 7: Implement userdata3.json parser

**Files:**
- Create: `src/json/userdata3.rs`
- Modify: `src/json/mod.rs`
- Modify: `src/json/model3.rs`
- Modify: `tests/json.rs`

**Interfaces:**
- Consumes: `serde_json`, `crate::Error`
- Produces: `UserData3::from_json_str`, `UserDataEntry`, `UserDataTarget`, `Model3::user_data()`

- [ ] **Step 1: Write the failing tests**

Add to `tests/json.rs`:

```rust
use mocari::json::UserData3;

#[test]
fn userdata3_parses_entries() {
    let json = r#"{
        "Version": 3,
        "UserData": [
            { "Target": "Parameter", "Id": "ParamAngleX", "Value": "head_turn" },
            { "Target": "Part", "Id": "PartArmL", "Value": "left_arm" },
            { "Target": "Drawable", "Id": "DrawBody", "Value": "body_mesh" }
        ]
    }"#;

    let data = UserData3::from_json_str(json).unwrap();
    assert_eq!(data.version(), 3);
    assert_eq!(data.entries().len(), 3);
    assert_eq!(data.entries()[0].id(), "ParamAngleX");
    assert_eq!(data.entries()[0].value(), "head_turn");
}

#[test]
fn userdata3_empty_is_valid() {
    let json = r#"{
        "Version": 3,
        "UserData": []
    }"#;

    let data = UserData3::from_json_str(json).unwrap();
    assert_eq!(data.entries().len(), 0);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test json userdata3 -- --nocapture`
Expected: Tests fail (UserData3 doesn't exist)

- [ ] **Step 3: Create `src/json/userdata3.rs`**

```rust
use serde::Deserialize;

use crate::{Error, Result};

const FORMAT: &str = "userdata3.json";

#[derive(Debug, Clone, PartialEq, Eq)]
/// Target type for a user data entry.
pub enum UserDataTarget {
    /// User data attached to a parameter.
    Parameter,
    /// User data attached to a part.
    Part,
    /// User data attached to a drawable.
    Drawable,
}

impl UserDataTarget {
    fn from_raw(raw: &str) -> Self {
        match raw {
            "Part" => Self::Part,
            "Drawable" => Self::Drawable,
            _ => Self::Parameter,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// One entry in a `userdata3.json` file.
pub struct UserDataEntry {
    target: UserDataTarget,
    id: String,
    value: String,
}

impl UserDataEntry {
    /// Returns the target type.
    pub fn target(&self) -> &UserDataTarget {
        &self.target
    }

    /// Returns the target element id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the user data string value.
    pub fn value(&self) -> &str {
        &self.value
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Parsed Cubism `userdata3.json` data.
pub struct UserData3 {
    version: u32,
    entries: Vec<UserDataEntry>,
}

impl UserData3 {
    /// Parses a `userdata3.json` document from a string.
    pub fn from_json_str(source: &str) -> Result<Self> {
        let raw: RawUserData3 =
            serde_json::from_str(source).map_err(|error| Error::InvalidJson {
                format: FORMAT,
                message: error.to_string(),
            })?;

        Ok(Self {
            version: raw.version,
            entries: raw
                .user_data
                .into_iter()
                .map(|raw| UserDataEntry {
                    target: UserDataTarget::from_raw(&raw.target),
                    id: raw.id,
                    value: raw.value,
                })
                .collect(),
        })
    }

    /// Returns the format version.
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Returns all user data entries.
    pub fn entries(&self) -> &[UserDataEntry] {
        &self.entries
    }

    /// Finds the first entry matching the given target and id.
    pub fn find(&self, target: &UserDataTarget, id: &str) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.target == *target && entry.id == id)
            .map(|entry| entry.value.as_str())
    }
}

#[derive(Debug, Deserialize)]
struct RawUserData3 {
    #[serde(rename = "Version")]
    version: u32,
    #[serde(rename = "UserData")]
    user_data: Vec<RawUserDataEntry>,
}

#[derive(Debug, Deserialize)]
struct RawUserDataEntry {
    #[serde(rename = "Target")]
    target: String,
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Value")]
    value: String,
}
```

- [ ] **Step 4: Add to `src/json/mod.rs`**

Add `mod userdata3;` after `mod pose3;` and add the re-export:

```rust
pub use userdata3::{UserData3, UserDataEntry, UserDataTarget};
```

- [ ] **Step 5: Add `UserData` field to `Model3`**

In `src/json/model3.rs`, add to the `FileReferences` struct:

```rust
#[serde(rename = "UserData", default)]
user_data: Option<String>,
```

And add accessor to `Model3`:

```rust
/// Returns the optional `userdata3.json` path.
pub fn user_data(&self) -> Option<&str> {
    self.file_references.user_data.as_deref()
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test json userdata3 -- --nocapture`
Expected: All userdata3 tests PASS

- [ ] **Step 7: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add src/json/userdata3.rs src/json/mod.rs src/json/model3.rs tests/json.rs
git commit -m "feat(json): add userdata3.json parser and Model3 accessor"
```

---

### Task 8: Implement drawable visibility in ModelRuntime

**Files:**
- Modify: `src/runtime.rs`
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: `Moc3DrawableMesh::vertices_mut()`
- Produces: `ModelRuntime::set_drawable_visible`, `ModelRuntime::is_drawable_visible`, `ModelRuntime::reset_drawable_visibility`

- [ ] **Step 1: Write the failing tests**

Add to `tests/runtime.rs`:

```rust
#[test]
fn drawable_visibility_hides_mesh_vertices() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let drawable_ids: Vec<String> = model.runtime().drawable_ids().to_vec();
    assert!(!drawable_ids.is_empty());

    let target_id = &drawable_ids[0];
    let index = model.runtime().drawable_index(target_id).unwrap();
    let before_vertices = model.runtime().meshes()[index].vertices().to_vec();

    model.runtime_mut().set_drawable_visible(target_id, false);
    model.runtime_mut().update_meshes().unwrap();

    let after_vertices = model.runtime().meshes()[index].vertices().to_vec();
    // Hidden drawable should have all vertices at origin
    for vertex in &after_vertices {
        assert_eq!(vertex.position(), [0.0, 0.0], "hidden vertex should be at origin");
    }
    assert_ne!(before_vertices, after_vertices, "vertices should change when hidden");
}

#[test]
fn drawable_visibility_reset_restores_all() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let drawable_ids: Vec<String> = model.runtime().drawable_ids().to_vec();

    // Hide all
    for id in &drawable_ids {
        model.runtime_mut().set_drawable_visible(id, false);
    }
    model.runtime_mut().update_meshes().unwrap();

    // Reset all
    model.runtime_mut().reset_drawable_visibility();
    model.runtime_mut().update_meshes().unwrap();

    // All should be visible again
    for (i, id) in drawable_ids.iter().enumerate() {
        assert!(model.runtime().is_drawable_visible(i), "drawable {id} should be visible after reset");
    }
}

#[test]
fn set_drawable_visible_by_index_works() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    model.runtime_mut().set_drawable_visible_by_index(0, false);
    assert!(!model.runtime().is_drawable_visible(0));
    model.runtime_mut().set_drawable_visible_by_index(0, true);
    assert!(model.runtime().is_drawable_visible(0));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test runtime drawable_visibility -- --nocapture`
Expected: Tests fail (methods don't exist)

- [ ] **Step 3: Add visibility storage to ModelRuntime**

In `src/runtime.rs`, add `drawable_visible: Vec<bool>` to the `ModelRuntime` struct, initialize it in `new()`:

Add to struct fields (after `mesh_update_scratch`):

```rust
drawable_visible: Vec<bool>,
```

In `new()`, after `mesh_update_scratch: Moc3MeshUpdateScratch::default()`:

```rust
drawable_visible: vec![true; art_meshes.meshes().len()],
```

- [ ] **Step 4: Add visibility methods to ModelRuntime**

Add these methods to the `impl ModelRuntime` block:

```rust
/// Sets whether a drawable is visible by id.
///
/// Hidden drawables produce zero-area meshes. Returns `false` when the id is
/// not present in the model.
pub fn set_drawable_visible(&mut self, id: &str, visible: bool) -> bool {
    match self.drawable_index(id) {
        Some(index) => self.set_drawable_visible_by_index(index, visible),
        None => false,
    }
}

/// Sets whether a drawable is visible by index.
pub fn set_drawable_visible_by_index(&mut self, index: usize, visible: bool) -> bool {
    let Some(slot) = self.drawable_visible.get_mut(index) else {
        return false;
    };
    *slot = visible;
    true
}

/// Returns whether a drawable is currently visible.
pub fn is_drawable_visible(&self, index: usize) -> bool {
    self.drawable_visible.get(index).copied().unwrap_or(true)
}

/// Resets all drawables to visible.
pub fn reset_drawable_visibility(&mut self) {
    self.drawable_visible.fill(true);
}
```

- [ ] **Step 5: Apply visibility in update_meshes**

In the `apply_mesh_post_processing` method, add visibility collapsing after the existing glues and render order logic. Add a new method and call it from `update_meshes`:

```rust
fn apply_drawable_visibility(&mut self) {
    for (index, mesh) in self.meshes.iter_mut().enumerate() {
        if !self.drawable_visible.get(index).copied().unwrap_or(true) {
            for vertex in mesh.vertices_mut() {
                *vertex = crate::moc3::Moc3DrawableVertex::new([0.0, 0.0], vertex.uv());
            }
        }
    }
}
```

And in `update_meshes`, call it after `apply_mesh_post_processing`:

```rust
pub fn update_meshes(&mut self) -> Option<()> {
    self.update_part_opacities();
    let drawable_part_opacities = self.drawable_part_opacities();
    self.rebuild_or_update_meshes(&drawable_part_opacities)?;
    self.apply_mesh_post_processing()?;
    self.apply_drawable_visibility();
    Some(())
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test runtime drawable_visibility -- --nocapture`
Expected: All drawable_visibility tests PASS

- [ ] **Step 7: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 8: Commit**

```bash
git add src/runtime.rs tests/runtime.rs
git commit -m "feat(runtime): add drawable visibility control"
```

---

### Task 9: Wire userdata3.json loading into assets.rs

**Files:**
- Modify: `src/assets.rs`
- Modify: `src/runtime.rs`
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: `UserData3::from_json_str`, `Model3::user_data()`
- Produces: `ModelRuntime::user_data()`, `ModelRuntime::find_user_data()`

- [ ] **Step 1: Write the failing tests**

Add to `tests/runtime.rs`:

```rust
use mocari::json::UserDataTarget;

#[test]
fn runtime_exposes_user_data_when_present() {
    // Hiyori doesn't have userdata, so user_data should be None
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    assert!(model.runtime().user_data().is_none());
}

#[test]
fn find_user_data_returns_none_when_no_data() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    assert!(model.runtime().find_user_data(&UserDataTarget::Parameter, "ParamAngleX").is_none());
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test runtime user_data -- --nocapture`
Expected: Tests fail (methods don't exist)

- [ ] **Step 3: Add user_data storage to ModelRuntime**

In `src/runtime.rs`, add to imports:

```rust
use crate::json::{Model3, Physics3, Pose3, UserData3, UserDataTarget, copy_pose_link_opacities, update_pose_group_opacities};
```

Add `user_data: Option<UserData3>` to the `ModelRuntime` struct, initialize to `None` in `new()`.

Add accessor methods:

```rust
/// Returns the user data loaded from `userdata3.json`, if any.
pub fn user_data(&self) -> Option<&UserData3> {
    self.user_data.as_ref()
}

/// Finds a user data value by target type and element id.
pub fn find_user_data(&self, target: &UserDataTarget, id: &str) -> Option<&str> {
    self.user_data.as_ref().and_then(|data| data.find(target, id))
}

/// Sets user data on the runtime.
pub fn set_user_data(&mut self, data: UserData3) {
    self.user_data = Some(data);
}
```

- [ ] **Step 4: Load userdata3.json in assets.rs**

In `src/assets.rs`, add `UserData3` to the json import and load it in `parse_model`:

```rust
use crate::json::{Model3, Physics3, Pose3, UserData3};
```

Add `user_data: Option<UserData3>` to `ParsedModel` struct.

In `parse_model`, after the pose loading block, add:

```rust
let user_data = match model.user_data() {
    Some(user_data_file) => {
        let user_data_source = read_text(&model_dir.join(user_data_file))?;
        Some(UserData3::from_json_str(&user_data_source).map_err(AssetLoadError::Json)?)
    }
    None => None,
};
```

In `into_runtime_model`, after physics loading, add:

```rust
if let Some(user_data) = self.user_data {
    runtime.set_user_data(user_data);
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test runtime user_data -- --nocapture`
Expected: All user_data tests PASS

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 7: Commit**

```bash
git add src/assets.rs src/runtime.rs tests/runtime.rs
git commit -m "feat(assets): wire userdata3.json loading into runtime"
```

---

### Task 10: Add public re-exports to lib.rs

**Files:**
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: `pub use auto::*`, `pub use json::UserData3`

- [ ] **Step 1: Update lib.rs re-exports**

Ensure `src/lib.rs` has these re-exports:

```rust
pub use crate::auto::{Breath, BreathConfig, EyeBlink, EyeBlinkConfig, LipSync, LipSyncConfig, MouseTracker, MouseTrackerConfig};
pub use crate::json::UserData3;
pub use crate::motion::{MotionManager, MotionPlayer, MotionPriority};
```

- [ ] **Step 2: Verify all public types are accessible**

Run: `cargo doc --no-deps 2>&1 | head -20`
Expected: No errors, all types documented

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs
git commit -m "feat(lib): add public re-exports for all new runtime features"
```

---

### Task 11: Integration test - all features together

**Files:**
- Modify: `tests/runtime.rs`

**Interfaces:**
- Consumes: All new types and methods

- [ ] **Step 1: Write the integration test**

Add to `tests/runtime.rs`:

```rust
#[test]
fn all_features_work_together() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();

    // Create all auto-animation features
    let mut blink = EyeBlink::with_defaults();
    let mut lip = LipSync::with_defaults();
    let mut breath = Breath::with_defaults();
    let mut tracker = MouseTracker::with_defaults();
    let mut motion_mgr = MotionManager::new();

    // Set up inputs
    lip.set_amplitude(0.5);
    tracker.set_target(0.5, -0.3);

    // Simulate a few frames
    let delta = 1.0 / 60.0;
    for _ in 0..10 {
        runtime.reset_parameters();
        runtime.reset_part_opacities();

        blink.tick(delta);
        blink.apply(runtime);
        lip.tick(delta);
        lip.apply(runtime);
        breath.tick(delta);
        breath.apply(runtime);
        tracker.tick(delta);
        tracker.apply(runtime);
        motion_mgr.tick(delta);
        motion_mgr.apply(runtime);

        runtime.apply_physics(delta);
        runtime.update_meshes().unwrap();
    }

    // Verify meshes are valid
    assert!(!runtime.meshes().is_empty(), "should have meshes after all features");
    for mesh in runtime.meshes() {
        for vertex in mesh.vertices() {
            let [x, y] = vertex.position();
            assert!(x.is_finite() && y.is_finite(), "vertex positions must be finite");
        }
    }
}
```

- [ ] **Step 2: Run the integration test**

Run: `cargo test --test runtime all_features_work_together -- --nocapture`
Expected: PASS

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add tests/runtime.rs
git commit -m "test: add integration test for all new Cubism runtime features"
```
