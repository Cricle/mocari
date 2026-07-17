# More Cubism Runtime Features - Design Spec

## Overview

Add the remaining core Cubism SDK runtime features to Mocari: auto-animation (eye blink, lip sync, breath, mouse tracking), a priority-based motion manager with crossfade blending, user data parsing (userdata3.json), and programmatic drawable visibility control.

## Goals

- Make models feel alive without manual parameter tweaking (auto-animation)
- Production-ready motion playback with queuing, priorities, and crossfade (MotionManager)
- Support userdata3.json sidecar files for per-element metadata
- Allow runtime show/hide of individual drawables

## Architecture

All features follow the standalone struct pattern established by `MotionPlayer` and `ExpressionPlayer`: each owns its state, exposes `tick(delta)` and `apply(runtime)` methods, and is independently usable.

### Module Structure

```
src/
  auto/               # NEW
    mod.rs
    eye_blink.rs
    lip_sync.rs
    breath.rs
    mouse_tracker.rs
  json/
    userdata3.rs       # NEW
  motion.rs            # MODIFIED - add MotionManager
  runtime.rs           # MODIFIED - add visibility, user data
  lib.rs               # MODIFIED - re-export new types
```

## Feature 1: Auto-Animation

### EyeBlink

Automatic eye blinking with randomized timing.

**State machine phases:** Open -> Closing -> Closed -> Opening -> Open

**Configuration (`EyeBlinkConfig`):**
- `min_interval: f32` - minimum seconds between blinks (default 2.5)
- `max_interval: f32` - maximum seconds between blinks (default 6.0)
- `close_duration: f32` - seconds to close eyes (default 0.1)
- `open_duration: f32` - seconds to open eyes (default 0.15)
- `weight: f32` - blend weight 0.0-1.0 (default 1.0)

**Target parameters:**
- `ParamEyeLOpen` - left eye open/close
- `ParamEyeROpen` - right eye open/close

**API:**
```rust
pub struct EyeBlink { /* state */ }
impl EyeBlink {
    pub fn new(config: EyeBlinkConfig) -> Self;
    pub fn with_defaults() -> Self;
    pub fn tick(&mut self, delta_seconds: f32);
    pub fn apply(&self, runtime: &mut ModelRuntime);
    pub fn reset(&mut self);
    pub fn set_weight(&mut self, weight: f32);
}
```

### LipSync

Audio-driven mouth movement. The user provides amplitude values from external audio analysis; LipSync smooths and applies them. Input amplitude is clamped to `0.0..=1.0`.

**Configuration (`LipSyncConfig`):**
- `smoothing: f32` - exponential smoothing factor (default 0.2)
- `weight: f32` - blend weight (default 1.0)

**Target parameters:**
- `ParamMouthOpenY` - mouth open amount

**API:**
```rust
pub struct LipSync { /* state */ }
impl LipSync {
    pub fn new(config: LipSyncConfig) -> Self;
    pub fn with_defaults() -> Self;
    pub fn set_amplitude(&mut self, amplitude: f32);
    pub fn tick(&mut self, delta_seconds: f32);
    pub fn apply(&self, runtime: &mut ModelRuntime);
    pub fn reset(&mut self);
    pub fn set_weight(&mut self, weight: f32);
}
```

### Breath

Subtle breathing animation using a sine wave.

**Configuration (`BreathConfig`):**
- `cycle_speed: f32` - breathes per second (default 0.25, i.e. 4s cycle)
- `weight: f32` - blend weight (default 1.0)

**Target parameters:**
- `ParamBreath` - dedicated breath parameter

**API:**
```rust
pub struct Breath { /* state */ }
impl Breath {
    pub fn new(config: BreathConfig) -> Self;
    pub fn with_defaults() -> Self;
    pub fn tick(&mut self, delta_seconds: f32);
    pub fn apply(&self, runtime: &mut ModelRuntime);
    pub fn reset(&mut self);
    pub fn set_weight(&mut self, weight: f32);
}
```

### MouseTracker

Face follows cursor position with smooth interpolation.

**Configuration (`MouseTrackerConfig`):**
- `smoothing: f32` - exponential smoothing factor (default 0.15)
- `weight: f32` - blend weight (default 1.0)

**Target parameters:**
- `ParamAngleX`, `ParamAngleY` - head rotation
- `ParamBodyAngleX` - body rotation
- `ParamEyeBallX`, `ParamEyeBallY` - eye direction

**Coordinate space:** `set_target(x, y)` takes normalized values in `-1.0..=1.0` where `(0, 0)` is center, `(-1, -1)` is bottom-left, `(1, 1)` is top-right. The caller is responsible for converting from window coordinates to this normalized space.

**API:**
```rust
pub struct MouseTracker { /* state */ }
impl MouseTracker {
    pub fn new(config: MouseTrackerConfig) -> Self;
    pub fn with_defaults() -> Self;
    pub fn set_target(&mut self, x: f32, y: f32);
    pub fn tick(&mut self, delta_seconds: f32);
    pub fn apply(&self, runtime: &mut ModelRuntime);
    pub fn reset(&mut self);
    pub fn set_weight(&mut self, weight: f32);
}
```

## Feature 2: MotionManager

Priority-based motion queue with crossfade blending.

### Design

`MotionManager` manages multiple active `MotionPlayer`s with:
- **Priority levels:** `Idle(0)`, `Normal(1)`, `Force(2)`
- **Crossfade:** configurable fade duration when switching motions
- **Group semantics:** same-group motions replace each other; different-group motions can play simultaneously
- **Queue:** Normal-priority motions queue (FIFO) and play when the current motion finishes

### API

```rust
pub struct MotionManager {
    players: Vec<ManagedMotion>,
    crossfade_duration: f32,
}

struct ManagedMotion {
    player: MotionPlayer,
    priority: MotionPriority,
    group: String,
    fade_in_remaining: f32,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MotionPriority { Idle, Normal, Force }

impl MotionManager {
    pub fn new() -> Self;
    pub fn set_crossfade_duration(&mut self, seconds: f32);
    pub fn crossfade_duration(&self) -> f32;

    pub fn start_motion(&mut self, motion: Motion3, priority: MotionPriority, group: &str) -> usize;
    pub fn start_motion_from_path(&mut self, path: &str, priority: MotionPriority, group: &str) -> Result<usize, MotionLoadError>;

    pub fn tick(&mut self, delta_seconds: f32);
    pub fn apply(&self, runtime: &mut ModelRuntime);

    pub fn stop_all(&mut self);
    pub fn stop_all_with_fade(&mut self, fade_seconds: f32);

    pub fn is_finished(&self) -> bool;
    pub fn active_count(&self) -> usize;
    pub fn clear(&mut self);
}
```

### Priority Behavior

- **Idle:** Only plays if no other motions are active.
- **Normal:** Plays after current normal motion finishes. If same group as an active motion, crossfades into it.
- **Force:** Immediately starts, crossfading out any currently playing motions.

### Crossfade Behavior

When a new motion replaces an old one:
1. Old motion's weight ramps from current to 0.0 over `crossfade_duration`
2. New motion's weight ramps from 0.0 to 1.0 over `crossfade_duration`
3. Both motions are applied during the crossfade, blending their parameter outputs
4. Finished motions (weight reached 0.0) are removed

### Multiple Simultaneous Motions

Different groups can play at the same time. Each motion's `apply()` writes to the parameters it controls. Since motions from different groups typically target different parameter sets, they compose naturally. When two active motions target the same parameter, the last-applied motion wins (same as current MotionPlayer behavior).

## Feature 3: User Data (userdata3.json)

### Parser

```rust
// src/json/userdata3.rs

pub struct UserData3 {
    version: u32,
    entries: Vec<UserDataEntry>,
}

pub struct UserDataEntry {
    target: UserDataTarget,
    id: String,
    value: String,
}

pub enum UserDataTarget { Parameter, Part, Drawable }
```

- Parsed from `userdata3.json` referenced in `model3.json` `FileReferences.UserData`
- Standard serde_json parsing, same pattern as other JSON parsers

### Runtime Integration

```rust
// additions to ModelRuntime
pub fn user_data(&self) -> Option<&UserData3>;
pub fn find_user_data(&self, target: &UserDataTarget, id: &str) -> Option<&str>;
```

- Stored as `Option<UserData3>` in `ModelRuntime`
- Loaded automatically by `assets::load_model_runtime` when the model references a userdata file
- `Model3` gets a `user_data() -> Option<&str>` accessor for the file path

### Model3 Changes

Add `UserData` field to `FileReferences` in `model3.json` parser:

```rust
#[serde(rename = "UserData", default)]
user_data: Option<String>,
```

## Feature 4: Drawable Visibility

### Runtime Storage

```rust
// in ModelRuntime
drawable_visible: Vec<bool>,
```

Initialized to `vec![true; drawable_count]` in `ModelRuntime::new()`.

### API

```rust
// new methods on ModelRuntime
pub fn set_drawable_visible(&mut self, id: &str, visible: bool) -> bool;
pub fn set_drawable_visible_by_index(&mut self, index: usize, visible: bool) -> bool;
pub fn is_drawable_visible(&self, index: usize) -> bool;
pub fn reset_drawable_visibility(&mut self);
```

### Mesh Update Behavior

In `update_meshes()`, hidden drawables produce zero-area meshes (all vertices collapsed to origin). This approach:
- Preserves mesh array indexing (no index shifts)
- Keeps clipping/mask references valid
- Requires no changes to the renderer backend
- Zero-cost for visible drawables (just a branch per drawable)

## Typical Frame Update

```rust
// 1. Reset
runtime.reset_parameters();
runtime.reset_part_opacities();

// 2. Auto-animation
mouse_tracker.tick(delta);
mouse_tracker.apply(&mut runtime);
eye_blink.tick(delta);
eye_blink.apply(&mut runtime);
breath.tick(delta);
breath.apply(&mut runtime);
lip_sync.tick(delta);
lip_sync.apply(&mut runtime);

// 3. Motions and expressions
motion_manager.tick(delta);
motion_manager.apply(&mut runtime);
expression_manager.tick(delta);
expression_manager.apply(&mut runtime);

// 4. Overrides and physics
runtime.apply_parameter_overrides();
runtime.apply_physics(delta);
runtime.apply_pose(delta);

// 5. Rebuild meshes
runtime.update_meshes();
```

## Public API Exports

```rust
// lib.rs additions
pub mod auto;
pub use auto::{EyeBlink, EyeBlinkConfig, LipSync, LipSyncConfig, Breath, BreathConfig, MouseTracker, MouseTrackerConfig};
pub use motion::{MotionManager, MotionPriority};
pub use json::UserData3;
```

## Error Handling

- All new parsers follow existing pattern: `Result<T, crate::Error>`
- `MotionManager::start_motion_from_path` uses existing `MotionLoadError`
- Auto-animation structs are infallible (pure math, no I/O)
- Drawable visibility is infallible (boolean state)

## Testing

- **EyeBlink:** State machine transitions, timing accuracy, parameter output values
- **LipSync:** Smoothing behavior, amplitude clamping
- **Breath:** Sine wave output, cycle timing
- **MouseTracker:** Smoothing interpolation, parameter range mapping
- **MotionManager:** Priority ordering, crossfade blending, group replacement, queue behavior
- **UserData3:** JSON parsing, accessor lookups
- **Drawable visibility:** Hidden drawables produce zero-area meshes, visibility reset
- **Integration:** Load model, apply all features, verify meshes update without errors
