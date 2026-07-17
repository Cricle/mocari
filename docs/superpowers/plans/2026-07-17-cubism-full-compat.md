# Cubism Full SDK Compatibility Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fill the remaining gaps between Mocari and the official Cubism SDK runtime (Core + Framework layers).

**Architecture:** Each feature follows existing patterns — color overrides mirror parameter overrides, motion drawable curves extend the curve target system, expression targets add a `Target` field to the JSON parser. All changes are additive; no existing APIs break.

**Tech Stack:** Rust, serde_json, wgpu (existing dependencies)

## Global Constraints

- `#![forbid(unsafe_code)]` is enforced globally
- All new public types derive `Debug, Clone`
- All new enum variants must be `Copy`
- Existing tests must continue to pass (47 tests + 3 doctests)
- Each task ends with `cargo test` passing

---

### Task 1: Culling Flag

**Files:**
- Modify: `src/moc3/drawable.rs:5-7` — add `DRAWABLE_DOUBLE_SIDED` constant
- Modify: `src/moc3/drawable.rs` — add `is_double_sided()` to `Moc3DrawableMesh`
- Modify: `src/render/common/clipping.rs` — add `is_double_sided()` to `DrawableInfo`
- Modify: `src/runtime.rs` — add `is_drawable_double_sided()` to `ModelRuntime`
- Test: `tests/runtime.rs`

**Interfaces:**
- Consumes: `Moc3DrawableMesh.drawable_flags()`, `DrawableInfo::from_mesh()`
- Produces: `Moc3DrawableMesh::is_double_sided() -> bool`, `DrawableInfo::is_double_sided() -> bool`, `ModelRuntime::is_drawable_double_sided(index: usize) -> bool`

- [ ] **Step 1: Add constant and method to Moc3DrawableMesh**

In `src/moc3/drawable.rs`, add after line 7 (`const DRAWABLE_MASK_INVERTED`):

```rust
const DRAWABLE_DOUBLE_SIDED: u8 = 1 << 2;
```

Add method to `Moc3DrawableMesh` impl block, after `is_inverted_mask()`:

```rust
/// Returns whether this drawable disables back-face culling.
pub fn is_double_sided(&self) -> bool {
    self.drawable_flags & DRAWABLE_DOUBLE_SIDED != 0
}
```

- [ ] **Step 2: Add to DrawableInfo**

In `src/render/common/clipping.rs`, add field to `DrawableInfo` struct:

```rust
double_sided: bool,
```

In `DrawableInfo::from_mesh()`, initialize it:

```rust
double_sided: mesh.is_double_sided(),
```

Add method to `DrawableInfo` impl:

```rust
/// Returns whether this drawable disables back-face culling.
pub fn is_double_sided(&self) -> bool {
    self.double_sided
}
```

- [ ] **Step 3: Add to ModelRuntime**

In `src/runtime.rs`, add method to `ModelRuntime` impl:

```rust
/// Returns whether a drawable disables back-face culling.
pub fn is_drawable_double_sided(&self, index: usize) -> bool {
    self.meshes
        .get(index)
        .map(|m| m.is_double_sided())
        .unwrap_or(false)
}
```

- [ ] **Step 4: Write test**

In `tests/runtime.rs`, add:

```rust
#[test]
fn drawable_culling_flag_from_model() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime();
    // All drawables should report a culling flag (default false for most models)
    for index in 0..runtime.drawable_ids().len() {
        let _ = runtime.is_drawable_double_sided(index);
    }
}
```

- [ ] **Step 5: Run tests and commit**

Run: `cargo test`
Expected: all 48+ tests pass

```bash
git add src/moc3/drawable.rs src/render/common/clipping.rs src/runtime.rs tests/runtime.rs
git commit -m "feat: add drawable culling flag (double-sided)"
```

---

### Task 2: Drawable Color Overrides

**Files:**
- Modify: `src/runtime.rs` — add color override storage, API, and application logic
- Test: `tests/runtime.rs`

**Interfaces:**
- Consumes: `Moc3DrawableMesh::set_multiply_color()`, `Moc3DrawableMesh::set_screen_color()`, `Moc3DrawableMesh::multiply_color()`, `Moc3DrawableMesh::screen_color()`
- Produces: `ModelRuntime::set_drawable_multiply_color(id, color)`, `set_drawable_multiply_color_by_index(index, color)`, `set_drawable_screen_color(id, color)`, `set_drawable_screen_color_by_index(index, color)`, `clear_drawable_color_overrides()`, `mesh_multiply_color(index)`, `mesh_screen_color(index)`

- [ ] **Step 1: Add storage fields to ModelRuntime**

In `src/runtime.rs`, add two fields to the `ModelRuntime` struct after `drawable_visible`:

```rust
drawable_multiply_overrides: Vec<Option<[f32; 3]>>,
drawable_screen_overrides: Vec<Option<[f32; 3]>>,
```

In `ModelRuntime::new()`, initialize them after `drawable_visible`:

```rust
drawable_multiply_overrides: vec![None; drawable_count],
drawable_screen_overrides: vec![None; drawable_count],
```

- [ ] **Step 2: Add API methods**

Add these methods to `ModelRuntime` impl block, after `reset_drawable_visibility()`:

```rust
/// Returns the pending multiply color override for a drawable index.
pub fn drawable_multiply_color_override(&self, index: usize) -> Option<[f32; 3]> {
    self.drawable_multiply_overrides.get(index).copied().flatten()
}

/// Returns the pending screen color override for a drawable index.
pub fn drawable_screen_color_override(&self, index: usize) -> Option<[f32; 3]> {
    self.drawable_screen_overrides.get(index).copied().flatten()
}

/// Sets a multiply color override by drawable id.
pub fn set_drawable_multiply_color(&mut self, id: &str, color: [f32; 3]) -> bool {
    match self.drawable_index(id) {
        Some(index) => self.set_drawable_multiply_color_by_index(index, color),
        None => false,
    }
}

/// Sets a multiply color override by drawable index.
pub fn set_drawable_multiply_color_by_index(&mut self, index: usize, color: [f32; 3]) -> bool {
    let Some(slot) = self.drawable_multiply_overrides.get_mut(index) else {
        return false;
    };
    *slot = Some(color);
    true
}

/// Sets a screen color override by drawable id.
pub fn set_drawable_screen_color(&mut self, id: &str, color: [f32; 3]) -> bool {
    match self.drawable_index(id) {
        Some(index) => self.set_drawable_screen_color_by_index(index, color),
        None => false,
    }
}

/// Sets a screen color override by drawable index.
pub fn set_drawable_screen_color_by_index(&mut self, index: usize, color: [f32; 3]) -> bool {
    let Some(slot) = self.drawable_screen_overrides.get_mut(index) else {
        return false;
    };
    *slot = Some(color);
    true
}

/// Clears all drawable color overrides.
pub fn clear_drawable_color_overrides(&mut self) {
    self.drawable_multiply_overrides.fill(None);
    self.drawable_screen_overrides.fill(None);
}

/// Returns the current multiply color on a mesh (after update_meshes).
pub fn mesh_multiply_color(&self, index: usize) -> Option<[f32; 3]> {
    self.meshes.get(index).map(|m| m.multiply_color())
}

/// Returns the current screen color on a mesh (after update_meshes).
pub fn mesh_screen_color(&self, index: usize) -> Option<[f32; 3]> {
    self.meshes.get(index).map(|m| m.screen_color())
}
```

- [ ] **Step 3: Add application method**

Add private method to `ModelRuntime` impl:

```rust
fn apply_drawable_color_overrides(&mut self) {
    for (index, mesh) in self.meshes.iter_mut().enumerate() {
        if let Some(color) = self.drawable_multiply_overrides.get(index).and_then(|c| *c) {
            mesh.set_multiply_color(color);
        }
        if let Some(color) = self.drawable_screen_overrides.get(index).and_then(|c| *c) {
            mesh.set_screen_color(color);
        }
    }
}
```

- [ ] **Step 4: Wire into update_meshes**

In `update_meshes()`, add `self.apply_drawable_color_overrides();` after `self.apply_mesh_post_processing()?;` and before `self.apply_drawable_visibility();`.

- [ ] **Step 5: Write tests**

In `tests/runtime.rs`, add:

```rust
#[test]
fn drawable_multiply_color_override() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();

    // Default is no override
    assert!(runtime.drawable_multiply_color_override(0).is_none());

    // Set override
    runtime.set_drawable_multiply_color_by_index(0, [0.5, 0.5, 0.5]);
    assert_eq!(runtime.drawable_multiply_color_override(0), Some([0.5, 0.5, 0.5]));

    // After update_meshes, mesh color reflects override
    runtime.update_meshes();
    assert_eq!(runtime.mesh_multiply_color(0), Some([0.5, 0.5, 0.5]));

    // Clear overrides reverts to keyform default
    runtime.clear_drawable_color_overrides();
    runtime.update_meshes();
    assert_eq!(runtime.mesh_multiply_color(0), Some([1.0, 1.0, 1.0]));
}

#[test]
fn drawable_screen_color_override() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();

    runtime.set_drawable_screen_color_by_index(0, [0.1, 0.2, 0.3]);
    runtime.update_meshes();
    assert_eq!(runtime.mesh_screen_color(0), Some([0.1, 0.2, 0.3]));

    runtime.clear_drawable_color_overrides();
    runtime.update_meshes();
    assert_eq!(runtime.mesh_screen_color(0), Some([0.0, 0.0, 0.0]));
}

#[test]
fn set_drawable_color_by_id() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let id = runtime.drawable_ids()[0].clone();

    assert!(runtime.set_drawable_multiply_color(&id, [0.8, 0.8, 0.8]));
    assert!(!runtime.set_drawable_multiply_color("nonexistent", [0.0, 0.0, 0.0]));
}
```

- [ ] **Step 6: Run tests and commit**

Run: `cargo test`
Expected: all tests pass

```bash
git add src/runtime.rs tests/runtime.rs
git commit -m "feat: add drawable color overrides (multiply/screen)"
```

---

### Task 3: Motion User Data Events

**Files:**
- Modify: `src/json/motion3.rs` — add `MotionUserData` struct, parse `UserData` array
- Modify: `src/json/mod.rs` — re-export `MotionUserData`
- Modify: `src/motion.rs` — add event tracking to `MotionPlayer`
- Modify: `src/lib.rs` — re-export `MotionUserData`
- Test: `tests/json.rs`, `tests/runtime.rs`

**Interfaces:**
- Consumes: `Motion3::user_data()`, `MotionPlayer::tick()`
- Produces: `MotionUserData { time: f32, value: String }`, `Motion3::user_data() -> &[MotionUserData]`, `MotionPlayer::drain_events() -> Vec<&str>`

- [ ] **Step 1: Add MotionUserData parser**

In `src/json/motion3.rs`, add after `MotionMeta` struct:

```rust
/// A user data event in a motion file.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MotionUserData {
    #[serde(rename = "Time")]
    time: f32,
    #[serde(rename = "Value")]
    value: String,
}

impl MotionUserData {
    /// Returns the time in seconds when this event fires.
    pub fn time(&self) -> f32 {
        self.time
    }

    /// Returns the event value string.
    pub fn value(&self) -> &str {
        &self.value
    }
}
```

- [ ] **Step 2: Add UserData to RawMotion3**

In `src/json/motion3.rs`, add field to `RawMotion3`:

```rust
#[serde(rename = "UserData", default)]
user_data: Vec<MotionUserData>,
```

Add field to `Motion3`:

```rust
user_data: Vec<MotionUserData>,
```

In `Motion3::from_json_str()`, populate it:

```rust
user_data: raw.user_data,
```

Add accessor to `Motion3`:

```rust
/// Returns user data events in this motion.
pub fn user_data(&self) -> &[MotionUserData] {
    &self.user_data
}
```

- [ ] **Step 3: Add event tracking to MotionPlayer**

In `src/motion.rs`, add fields to `MotionPlayer`:

```rust
last_event_time: f32,
event_cursor: usize,
```

Initialize in all constructors (`new`, `new_once`, `with_looping`):

```rust
last_event_time: 0.0,
event_cursor: 0,
```

Add method to `MotionPlayer`:

```rust
/// Returns user data events whose time falls within the last tick delta.
///
/// Call this after `tick()` to process events that fired this frame.
/// Works correctly with looping motions.
pub fn drain_events(&mut self) -> Vec<&str> {
    let motion_data = self.motion.user_data();
    let mut events = Vec::new();
    while self.event_cursor < motion_data.len() {
        let event = &motion_data[self.event_cursor];
        if event.time > self.last_event_time && event.time <= self.time {
            events.push(event.value.as_str());
        }
        if event.time > self.time {
            break;
        }
        self.event_cursor += 1;
    }
    events
}
```

- [ ] **Step 4: Reset event state on restart/wrap**

In `MotionPlayer::restart()`, add:

```rust
self.last_event_time = 0.0;
self.event_cursor = 0;
```

In `MotionPlayer::tick()`, update `last_event_time` before advancing time. At the start of tick (after the `if self.finished` check):

```rust
self.last_event_time = self.time;
```

For looping motions, when time wraps (`self.time %= duration`), reset the event cursor:

```rust
if self.looping {
    self.time %= duration;
    if wrapped {
        self.event_cursor = 0;
        self.last_event_time = 0.0;
    }
}
```

To detect wrapping, save old time before modulo:

```rust
let old_time = self.time;
self.time += delta_seconds.max(0.0);
// ... in looping branch:
if self.time >= duration {
    self.time %= duration;
    self.event_cursor = 0;
    self.last_event_time = 0.0;
}
```

- [ ] **Step 5: Re-export**

In `src/json/mod.rs`, add `MotionUserData` to the re-exports from `motion3`.

In `src/lib.rs`, add:

```rust
pub use crate::json::MotionUserData;
```

- [ ] **Step 6: Write tests**

In `tests/json.rs`, add:

```rust
#[test]
fn motion3_user_data_parsed() {
    let json = r#"{
        "Version": 3,
        "Meta": {"Duration": 2.0, "Fps": 30.0, "Loop": false, "CurveCount": 0, "TotalSegmentCount": 0, "TotalPointCount": 0},
        "Curves": [],
        "UserData": [
            {"Time": 0.5, "Value": "event_a"},
            {"Time": 1.2, "Value": "event_b"}
        ]
    }"#;
    let motion = Motion3::from_json_str(json).unwrap();
    assert_eq!(motion.user_data().len(), 2);
    assert_eq!(motion.user_data()[0].time(), 0.5);
    assert_eq!(motion.user_data()[0].value(), "event_a");
}
```

In `tests/runtime.rs`, add:

```rust
#[test]
fn motion_player_drain_events() {
    let json = r#"{
        "Version": 3,
        "Meta": {"Duration": 2.0, "Fps": 30.0, "Loop": false, "CurveCount": 0, "TotalSegmentCount": 0, "TotalPointCount": 0},
        "Curves": [],
        "UserData": [
            {"Time": 0.5, "Value": "hello"},
            {"Time": 1.0, "Value": "world"}
        ]
    }"#;
    let motion = Motion3::from_json_str(json).unwrap();
    let mut player = MotionPlayer::new_once(motion);

    // Before any tick, no events
    assert!(player.drain_events().is_empty());

    // Tick past first event
    player.tick(0.6);
    let events = player.drain_events();
    assert_eq!(events, vec!["hello"]);

    // Tick past second event
    player.tick(0.5);
    let events = player.drain_events();
    assert_eq!(events, vec!["world"]);

    // No more events
    assert!(player.drain_events().is_empty());
}
```

- [ ] **Step 7: Run tests and commit**

Run: `cargo test`
Expected: all tests pass

```bash
git add src/json/motion3.rs src/json/mod.rs src/motion.rs src/lib.rs tests/json.rs tests/runtime.rs
git commit -m "feat: add motion user data events"
```

---

### Task 4: Expression PartOpacity Target

**Files:**
- Modify: `src/json/expression3.rs` — add `ExpressionTarget` enum, `Target` field to `ExpressionParameter`
- Modify: `src/json/mod.rs` — re-export `ExpressionTarget`
- Modify: `src/expression.rs` — handle PartOpacity in `apply()`
- Modify: `src/lib.rs` — re-export `ExpressionTarget`
- Test: `tests/json.rs`, `tests/runtime.rs`

**Interfaces:**
- Consumes: `ExpressionParameter::target()`, `ModelRuntime::part_index()`, `ModelRuntime::set_part_opacity_by_index()`
- Produces: `ExpressionTarget { Parameter, PartOpacity }`, `ExpressionParameter::target() -> ExpressionTarget`

- [ ] **Step 1: Add ExpressionTarget enum**

In `src/json/expression3.rs`, add before `ExpressionParameter`:

```rust
/// Target type for an expression parameter.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum ExpressionTarget {
    /// Targets a model parameter (default).
    #[default]
    Parameter,
    /// Targets a part opacity.
    PartOpacity,
}

impl ExpressionTarget {
    fn from_raw(value: Option<&str>) -> Self {
        match value {
            Some("PartOpacity") => Self::PartOpacity,
            _ => Self::Parameter,
        }
    }
}
```

- [ ] **Step 2: Add Target field to ExpressionParameter**

Add to `ExpressionParameter` struct:

```rust
#[serde(
    rename = "Target",
    default,
    deserialize_with = "deserialize_expression_target"
)]
target: ExpressionTarget,
```

Add accessor:

```rust
/// Returns the target type for this parameter.
pub fn target(&self) -> ExpressionTarget {
    self.target
}
```

Add deserializer:

```rust
fn deserialize_expression_target<'de, D>(
    deserializer: D,
) -> std::result::Result<ExpressionTarget, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
        .map(|value| ExpressionTarget::from_raw(value.as_deref()))
}
```

- [ ] **Step 3: Handle PartOpacity in ExpressionPlayer::apply()**

In `src/expression.rs`, modify `ExpressionPlayer::apply()`:

```rust
pub fn apply(&self, runtime: &mut ModelRuntime) {
    if self.finished {
        return;
    }

    let weight = self.fade_weight();
    for parameter in self.expression.parameters() {
        match parameter.target() {
            ExpressionTarget::Parameter => {
                let Some(index) = runtime.parameter_index(parameter.id()) else {
                    continue;
                };
                let Some(current) = runtime.parameter_value_by_index(index) else {
                    continue;
                };
                let value = apply_expression_parameter(current, parameter, weight);
                runtime.set_parameter_by_index(index, value);
            }
            ExpressionTarget::PartOpacity => {
                let Some(index) = runtime.part_index(parameter.id()) else {
                    continue;
                };
                let current = runtime.part_opacity_value(index).unwrap_or(1.0);
                let value = apply_expression_blend(current, parameter.value(), parameter.blend(), weight);
                runtime.set_part_opacity_by_index(index, value);
            }
        }
    }
}
```

Note: `ModelRuntime` needs a `part_opacity_value(index) -> Option<f32>` method. Add it:

```rust
/// Returns the current part opacity for an index (before overrides).
pub fn part_opacity_value(&self, index: usize) -> Option<f32> {
    self.part_opacities.get(index).copied()
}
```

- [ ] **Step 4: Handle PartOpacity in ExpressionManager::apply()**

The `ExpressionManager::apply()` currently only processes parameters. Extend it to also handle PartOpacity targets. The simplest approach: apply PartOpacity directly in `ExpressionPlayer::apply()` (which ExpressionManager calls), and keep the existing parameter blending logic in ExpressionManager for parameters only.

Actually, looking at the code more carefully, `ExpressionManager::apply()` does its own parameter blending that overrides what `ExpressionPlayer::apply()` would do. The cleanest approach:

1. `ExpressionPlayer::apply()` handles PartOpacity directly (as above)
2. `ExpressionManager::apply()` calls each player's `apply()` for PartOpacity, then does its own parameter blending for parameters

Modify `ExpressionManager::apply()`:

```rust
pub fn apply(&self, runtime: &mut ModelRuntime) {
    // Apply PartOpacity from each player directly
    for player in &self.players {
        if player.is_finished() {
            continue;
        }
        let weight = player.fade_weight();
        for parameter in player.expression().parameters() {
            if parameter.target() != ExpressionTarget::PartOpacity {
                continue;
            }
            let Some(index) = runtime.part_index(parameter.id()) else {
                continue;
            };
            let current = runtime.part_opacity_value(index).unwrap_or(1.0);
            let value = apply_expression_blend(current, parameter.value(), parameter.blend(), weight);
            runtime.set_part_opacity_by_index(index, value);
        }
    }

    // Existing parameter blending logic
    let mut values = expression_parameter_values(&self.players, runtime);
    if values.is_empty() {
        return;
    }
    // ... rest of existing code
}
```

- [ ] **Step 5: Re-export**

In `src/json/mod.rs`, add `ExpressionTarget` to re-exports from `expression3`.

In `src/lib.rs`, add:

```rust
pub use crate::json::ExpressionTarget;
```

- [ ] **Step 6: Write tests**

In `tests/json.rs`, add:

```rust
#[test]
fn expression3_part_opacity_target() {
    let json = r#"{
        "Type": "Happy",
        "Parameters": [
            {"Id": "ParamEyeLOpen", "Value": 0.0},
            {"Id": "PartHair", "Value": 0.5, "Target": "PartOpacity"}
        ]
    }"#;
    let expr = Expression3::from_json_str(json).unwrap();
    assert_eq!(expr.parameters()[0].target(), ExpressionTarget::Parameter);
    assert_eq!(expr.parameters()[1].target(), ExpressionTarget::PartOpacity);
    assert_eq!(expr.parameters()[1].id(), "PartHair");
}
```

- [ ] **Step 7: Run tests and commit**

Run: `cargo test`
Expected: all tests pass

```bash
git add src/json/expression3.rs src/json/mod.rs src/expression.rs src/runtime.rs src/lib.rs tests/json.rs tests/runtime.rs
git commit -m "feat: add expression PartOpacity target support"
```

---

### Task 5: Motion Drawable Curves

**Files:**
- Modify: `src/motion.rs` — add `"Drawable"` target handling in `MotionPlayer::apply()`
- Modify: `src/runtime.rs` — add `meshes_mut()` accessor
- Test: `tests/runtime.rs`

**Interfaces:**
- Consumes: `ModelRuntime::drawable_index()`, `ModelRuntime::meshes_mut()`, `Moc3DrawableMesh::set_opacity()`, `Moc3DrawableMesh::set_draw_order()`
- Produces: `ModelRuntime::meshes_mut() -> &mut [Moc3DrawableMesh]` (pub(crate))

- [ ] **Step 1: Add meshes_mut to ModelRuntime**

In `src/runtime.rs`, add method:

```rust
/// Returns mutable access to drawable meshes.
pub(crate) fn meshes_mut(&mut self) -> &mut [Moc3DrawableMesh] {
    &mut self.meshes
}
```

- [ ] **Step 2: Add Drawable target constant**

In `src/motion.rs`, add constant after `PART_OPACITY_TARGET`:

```rust
const DRAWABLE_TARGET: &str = "Drawable";
```

- [ ] **Step 3: Add Drawable match arm**

In `MotionPlayer::apply()`, add a new match arm after the `PART_OPACITY_TARGET` arm:

```rust
DRAWABLE_TARGET => {
    let Some((drawable_id, field)) = curve.id().rsplit_once('.') else {
        continue;
    };
    let Some(drawable_index) = runtime.drawable_index(drawable_id) else {
        continue;
    };
    let Some(mesh) = runtime.meshes_mut().get_mut(drawable_index) else {
        continue;
    };
    match field {
        "Opacity" => {
            let faded = apply_motion_fade(mesh.opacity(), sampled, curve_weight);
            mesh.set_opacity(faded);
        }
        "DrawOrder" => {
            mesh.set_draw_order(sampled);
        }
        _ => {} // VertexPosition not yet supported
    }
}
```

- [ ] **Step 4: Write test**

In `tests/runtime.rs`, add:

```rust
#[test]
fn motion_drawable_opacity_curve() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let drawable_id = runtime.drawable_ids()[0].clone();

    // Create a motion that targets drawable opacity
    let json = format!(r#"{{
        "Version": 3,
        "Meta": {{"Duration": 1.0, "Fps": 30.0, "Loop": false, "CurveCount": 1, "TotalSegmentCount": 1, "TotalPointCount": 2}},
        "Curves": [{{
            "Target": "Drawable",
            "Id": "{}.Opacity",
            "Segments": [0.0, 1.0, 0, 1.0, 0.0]
        }}]
    }}"#, drawable_id);

    let motion = Motion3::from_json_str(&json).unwrap();
    let mut player = MotionPlayer::new_once(motion);

    // Initial opacity should be 1.0
    runtime.update_meshes();
    let initial_opacity = runtime.meshes()[0].opacity();

    // Tick to end - opacity should be 0.0
    player.tick(1.0);
    player.apply(runtime);
    runtime.update_meshes();
    assert!((runtime.meshes()[0].opacity() - 0.0).abs() < 0.01);
}
```

- [ ] **Step 5: Run tests and commit**

Run: `cargo test`
Expected: all tests pass

```bash
git add src/motion.rs src/runtime.rs tests/runtime.rs
git commit -m "feat: add motion drawable curve targets (opacity, draw order)"
```

---

### Task 6: Groups Integration

**Files:**
- Modify: `src/auto/eye_blink.rs` — add `EyeBlinkConfig::for_parameters()`
- Modify: `src/auto/lip_sync.rs` — add `LipSyncConfig::for_parameters()`
- Modify: `src/runtime.rs` — add `eye_blink_config_from_model()` and `lip_sync_config_from_model()`
- Modify: `src/auto/eye_blink.rs` — update `EyeBlink` to use configurable parameter indices
- Modify: `src/auto/lip_sync.rs` — update `LipSync` to use configurable parameter indices
- Test: `tests/runtime.rs`

**Interfaces:**
- Consumes: `Model3::groups()`, `Group::name()`, `Group::ids()`, `ModelRuntime::parameter_index()`
- Produces: `ModelRuntime::eye_blink_config_from_model() -> EyeBlinkConfig`, `ModelRuntime::lip_sync_config_from_model() -> LipSyncConfig`

- [ ] **Step 1: Add parameter indices to EyeBlinkConfig**

In `src/auto/eye_blink.rs`, modify `EyeBlinkConfig`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct EyeBlinkConfig {
    pub min_interval: f32,
    pub max_interval: f32,
    pub close_duration: f32,
    pub open_duration: f32,
    pub weight: f32,
    /// Parameter indices to target. If empty, uses default ParamEyeLOpen/ParamEyeROpen.
    pub parameter_indices: Vec<usize>,
}
```

Update `Default` impl to include `parameter_indices: Vec::new()`.

Add constructor:

```rust
/// Creates a config targeting specific parameter indices.
pub fn for_parameters(indices: Vec<usize>) -> Self {
    Self {
        parameter_indices: indices,
        ..Default::default()
    }
}
```

- [ ] **Step 2: Update EyeBlink to use configurable indices**

In `EyeBlink::apply()`, if `self.config.parameter_indices` is not empty, use those indices instead of looking up by name:

```rust
pub fn apply(&self, runtime: &mut ModelRuntime) {
    if self.config.parameter_indices.is_empty() {
        // Default behavior: target ParamEyeLOpen and ParamEyeROpen
        if let Some(index) = runtime.parameter_index(PARAM_EYE_L_OPEN) {
            runtime.set_parameter_by_index(index, self.blink_value * self.config.weight);
        }
        if let Some(index) = runtime.parameter_index(PARAM_EYE_R_OPEN) {
            runtime.set_parameter_by_index(index, self.blink_value * self.config.weight);
        }
    } else {
        for &index in &self.config.parameter_indices {
            runtime.set_parameter_by_index(index, self.blink_value * self.config.weight);
        }
    }
}
```

- [ ] **Step 3: Add parameter indices to LipSyncConfig**

In `src/auto/lip_sync.rs`, modify `LipSyncConfig`:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct LipSyncConfig {
    pub smoothing: f32,
    pub weight: f32,
    /// Parameter indices to target. If empty, uses default ParamMouthOpenY.
    pub parameter_indices: Vec<usize>,
}
```

Update `Default` impl to include `parameter_indices: Vec::new()`.

Add constructor:

```rust
/// Creates a config targeting specific parameter indices.
pub fn for_parameters(indices: Vec<usize>) -> Self {
    Self {
        parameter_indices: indices,
        ..Default::default()
    }
}
```

- [ ] **Step 4: Update LipSync to use configurable indices**

In `LipSync::apply()`:

```rust
pub fn apply(&self, runtime: &mut ModelRuntime) {
    if self.config.parameter_indices.is_empty() {
        if let Some(index) = runtime.parameter_index(PARAM_MOUTH_OPEN_Y) {
            runtime.set_parameter_by_index(index, self.current_amplitude * self.config.weight);
        }
    } else {
        for &index in &self.config.parameter_indices {
            runtime.set_parameter_by_index(index, self.current_amplitude * self.config.weight);
        }
    }
}
```

- [ ] **Step 5: Add config builders to ModelRuntime**

In `src/runtime.rs`, add methods:

```rust
/// Builds an EyeBlinkConfig from the model's Groups data.
///
/// Reads groups named "EyeBlink" and extracts their parameter indices.
/// Returns a default config if no EyeBlink group is found.
pub fn eye_blink_config_from_model(&self) -> EyeBlinkConfig {
    let indices: Vec<usize> = self.model.groups().iter()
        .filter(|g| g.name() == "EyeBlink" && g.target() == "Parameter")
        .flat_map(|g| g.ids().iter())
        .filter_map(|id| self.parameter_index(id))
        .collect();
    if indices.is_empty() {
        EyeBlinkConfig::default()
    } else {
        EyeBlinkConfig::for_parameters(indices)
    }
}

/// Builds a LipSyncConfig from the model's Groups data.
///
/// Reads groups named "LipSync" and extracts their parameter indices.
/// Returns a default config if no LipSync group is found.
pub fn lip_sync_config_from_model(&self) -> LipSyncConfig {
    let indices: Vec<usize> = self.model.groups().iter()
        .filter(|g| g.name() == "LipSync" && g.target() == "Parameter")
        .flat_map(|g| g.ids().iter())
        .filter_map(|id| self.parameter_index(id))
        .collect();
    if indices.is_empty() {
        LipSyncConfig::default()
    } else {
        LipSyncConfig::for_parameters(indices)
    }
}
```

- [ ] **Step 6: Write tests**

In `tests/runtime.rs`, add:

```rust
#[test]
fn eye_blink_config_from_model_groups() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime();
    let config = runtime.eye_blink_config_from_model();
    // Config should either have indices from groups or be default
    if config.parameter_indices.is_empty() {
        // No EyeBlink group in this model, default config
        assert_eq!(config.min_interval, 2.5);
    } else {
        assert!(!config.parameter_indices.is_empty());
    }
}

#[test]
fn lip_sync_config_from_model_groups() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime();
    let config = runtime.lip_sync_config_from_model();
    if config.parameter_indices.is_empty() {
        assert_eq!(config.smoothing, 0.2);
    } else {
        assert!(!config.parameter_indices.is_empty());
    }
}
```

- [ ] **Step 7: Run tests and commit**

Run: `cargo test`
Expected: all tests pass

```bash
git add src/auto/eye_blink.rs src/auto/lip_sync.rs src/runtime.rs tests/runtime.rs
git commit -m "feat: add Groups integration for EyeBlink/LipSync auto-configuration"
```

---

### Task 7: Lib Re-exports and Doc Updates

**Files:**
- Modify: `src/lib.rs` — ensure all new public types are re-exported
- Modify: `src/json/mod.rs` — ensure all new types are re-exported

- [ ] **Step 1: Verify re-exports in src/json/mod.rs**

Ensure these are re-exported from their respective submodules:
- `MotionUserData` from `motion3`
- `ExpressionTarget` from `expression3`

- [ ] **Step 2: Verify re-exports in src/lib.rs**

Ensure these are in the public API:
- `pub use crate::json::MotionUserData;`
- `pub use crate::json::ExpressionTarget;`

- [ ] **Step 3: Run doc tests**

Run: `cargo test --doc`
Expected: all 3 doctests pass

- [ ] **Step 4: Run full test suite and commit**

Run: `cargo test`
Expected: all tests pass

```bash
git add src/lib.rs src/json/mod.rs
git commit -m "chore: ensure all new types are re-exported"
```
