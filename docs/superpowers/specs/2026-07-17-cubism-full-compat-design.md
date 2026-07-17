# Cubism Full SDK Compatibility - Design Spec

## Overview

Fill the remaining gaps between Mocari and the official Cubism SDK runtime (Core + Framework layers). The current implementation covers ~90% of the SDK; this spec covers the last 10%.

## Goals

- Runtime drawable color overrides (multiply/screen)
- Motion curve targets for drawables (opacity, draw order, vertex positions)
- Motion user data event callbacks
- Expression PartOpacity target support
- Drawable culling flag (double-sided)
- model3.json Groups integration for EyeBlink/LipSync auto-configuration

## Architecture

All changes follow existing patterns: color overrides mirror parameter overrides, motion drawable curves extend the existing curve target system, expression targets add a `Target` field to the JSON parser.

## Feature 1: Drawable Color Overrides

### Storage

In `ModelRuntime`:
```rust
drawable_multiply_overrides: Vec<Option<[f32; 3]>>,
drawable_screen_overrides: Vec<Option<[f32; 3]>>,
```

Initialized to `vec![None; drawable_count]` in `ModelRuntime::new()`.

### API

```rust
// Read current override (before update_meshes)
pub fn drawable_multiply_color_override(&self, index: usize) -> Option<[f32; 3]>;
pub fn drawable_screen_color_override(&self, index: usize) -> Option<[f32; 3]>;

// Set override by id or index
pub fn set_drawable_multiply_color(&mut self, id: &str, color: [f32; 3]) -> bool;
pub fn set_drawable_multiply_color_by_index(&mut self, index: usize, color: [f32; 3]) -> bool;
pub fn set_drawable_screen_color(&mut self, id: &str, color: [f32; 3]) -> bool;
pub fn set_drawable_screen_color_by_index(&mut self, index: usize, color: [f32; 3]) -> bool;

// Clear overrides
pub fn clear_drawable_color_overrides(&mut self);

// Read current mesh colors (after update_meshes)
pub fn mesh_multiply_color(&self, index: usize) -> Option<[f32; 3]>;
pub fn mesh_screen_color(&self, index: usize) -> Option<[f32; 3]>;
```

### Application

In `update_meshes()`, after `apply_mesh_post_processing()` and before `apply_drawable_visibility()`:
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

## Feature 2: Motion Drawable Curves

### Supported Targets

`MotionPlayer::apply()` currently handles `"Parameter"` and `"PartOpacity"`. Add `"Drawable"`:

| Curve ID | Effect |
|---|---|
| `{name}.Opacity` | `mesh.set_opacity(value)` |
| `{name}.DrawOrder` | `mesh.set_draw_order(value)` |
| `{name}.VertexPosition` | Modify vertex positions directly |

### Implementation

For `Drawable` target curves, `MotionPlayer::apply()` needs access to `&mut ModelRuntime` meshes. The method signature stays the same (`&self, runtime: &mut ModelRuntime`) since it already takes `&mut ModelRuntime`.

New match arm in `MotionPlayer::apply()`:
```rust
"DRAWABLE_TARGET" => {
    let (drawable_id, field) = curve.id().rsplit_once('.').continue;
    let Some(drawable_index) = runtime.drawable_index(drawable_id) else {
        continue;
    };
    match field {
        "Opacity" => {
            if let Some(mesh) = runtime.meshes_mut().get_mut(drawable_index) {
                mesh.set_opacity(apply_motion_fade(mesh.opacity(), sampled, curve_weight));
            }
        }
        "DrawOrder" => {
            if let Some(mesh) = runtime.meshes_mut().get_mut(drawable_index) {
                mesh.set_draw_order(sampled);
            }
        }
        _ => {}
    }
}
```

`ModelRuntime` gets a `pub(crate) fn meshes_mut()` accessor.

### Vertex Position Curves

Deferred — vertex animation is rare and complex. The parser accepts them but they are ignored for now. A follow-up can add `Drawable.VertexPosition.{N}.X/Y` if needed.

## Feature 3: Motion User Data Events

### JSON Format

motion3.json `UserData` array:
```json
{
  "UserData": [
    { "Time": 0.5, "Value": "play_sound:hello" },
    { "Time": 1.2, "Value": "trigger_effect" }
  ]
}
```

### Parser

```rust
// in json/motion3.rs
pub struct MotionUserData {
    time: f32,
    value: String,
}

impl MotionUserData {
    pub fn time(&self) -> f32 { self.time }
    pub fn value(&self) -> &str { &self.value }
}
```

Add to `RawMotion3`:
```rust
#[serde(rename = "UserData", default)]
user_data: Vec<RawMotionUserData>,
```

`Motion3` stores `Vec<MotionUserData>`, accessible via `user_data() -> &[MotionUserData]`.

### Runtime Events

`MotionPlayer` tracks the last event time to detect crossings:

```rust
// new field in MotionPlayer
last_event_time: f32,
event_cursor: usize,  // index into sorted user data
```

New method:
```rust
/// Returns user data events whose time falls within (last_time, current_time].
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

Called by user after `tick()`. Works correctly with looping (resets cursor on wrap).

## Feature 4: Expression PartOpacity Target

### JSON Format

```json
{
  "Parameters": [
    {"Id": "ParamEyeLOpen", "Value": 0.0},
    {"Id": "PartHair", "Value": 0.5, "Blend": "Add", "Target": "PartOpacity"}
  ]
}
```

### Parser Changes

Add `Target` field to `ExpressionParameter`:
```rust
#[serde(rename = "Target", default)]
target: ExpressionTarget,

pub enum ExpressionTarget { Parameter, PartOpacity }
```

Default is `Parameter` (backward compatible with existing files).

### Player Changes

`ExpressionPlayer::apply()` checks target:
```rust
match parameter.target() {
    ExpressionTarget::Parameter => {
        // existing parameter logic
    }
    ExpressionTarget::PartOpacity => {
        let Some(index) = runtime.part_index(parameter.id()) else { continue };
        let value = apply_expression_blend(current_part_opacity, parameter.value(), parameter.blend(), weight);
        runtime.set_part_opacity_by_index(index, value);
    }
}
```

`ExpressionManager::apply()` also handles PartOpacity targets.

## Feature 5: Culling Flag

### Moc3 Parsing

Bit 2 of drawable flags = `is_double_sided`. In the official SDK, when this flag is set, back-face culling is disabled.

```rust
// in moc3/drawable.rs
const DRAWABLE_DOUBLE_SIDED: u8 = 1 << 2;

impl Moc3DrawableMesh {
    pub fn is_double_sided(&self) -> bool {
        self.drawable_flags & DRAWABLE_DOUBLE_SIDED != 0
    }
}
```

### Renderer Exposure

`DrawableInfo` gets `is_double_sided() -> bool`.
`ModelRuntime` gets `is_drawable_double_sided(index) -> bool`.
wgpu renderer can use this to select pipeline variant.

## Feature 6: Groups Integration

### API

```rust
impl ModelRuntime {
    /// Builds an EyeBlinkConfig from model Groups data.
    pub fn eye_blink_config_from_model(&self) -> EyeBlinkConfig {
        let params = self.model.groups().iter()
            .filter(|g| g.name() == "EyeBlink")
            .flat_map(|g| g.ids().iter())
            .filter_map(|id| self.parameter_index(id))
            .collect();
        EyeBlinkConfig::for_parameters(params)
    }

    /// Builds a LipSyncConfig from model Groups data.
    pub fn lip_sync_config_from_model(&self) -> LipSyncConfig {
        let params = self.model.groups().iter()
            .filter(|g| g.name() == "LipSync")
            .flat_map(|g| g.ids().iter())
            .filter_map(|id| self.parameter_index(id))
            .collect();
        LipSyncConfig::for_parameters(params)
    }
}
```

### Config Changes

`EyeBlinkConfig` and `LipSyncConfig` get `for_parameters(indices: Vec<usize>)` constructors. The existing default constructors use hardcoded parameter ids; the new constructors accept model-specific indices.

## Typical Frame Update (Updated)

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
// Process motion events
for managed in motion_manager.active_mut() {
    for event in managed.player_mut().drain_events() {
        // handle event
    }
}
expression_manager.tick(delta);
expression_manager.apply(&mut runtime);

// 4. Overrides and physics
runtime.apply_parameter_overrides();
runtime.apply_physics(delta);
runtime.apply_pose(delta);

// 5. Drawable color overrides
runtime.apply_drawable_color_overrides();

// 6. Rebuild meshes
runtime.update_meshes();
```

## Public API Exports

```rust
// lib.rs additions
pub use json::MotionUserData;
pub use json::ExpressionTarget;
```

## Testing

- **Drawable colors**: set override, update_meshes, verify mesh color changed; clear override, verify reverts to keyform default
- **Motion drawable curves**: motion with Drawable.Opacity curve, verify mesh opacity changes during playback
- **Motion events**: motion with UserData array, tick past event times, verify drain_events returns correct values; looping resets correctly
- **Expression PartOpacity**: expression targeting PartOpacity, verify part opacity changes
- **Culling flag**: model with double-sided drawable, verify is_double_sided returns correct value
- **Groups integration**: model with EyeBlink/LipSync groups, verify config builders extract correct parameter indices
