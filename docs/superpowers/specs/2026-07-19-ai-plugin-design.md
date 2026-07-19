# AI Plugin System Design

## Goal

Add optional AI integration to mocari as plugins — AI-assisted model rigging and AI-driven runtime character control — without affecting the core library or adding external dependencies.

## Architecture

Two independent subsystems, both behind a single `ai` feature flag:

1. **AI Rigging Pipeline** — pre-processing step that produces a `RiggedModel`, which the engine converts into a live model using existing moc3/GPU pipelines.
2. **AI Runtime Drivers** — per-frame hooks that inject parameter changes before the engine ticks, composable and independent from render plugins.

### Design Principles

- **Zero AI dependencies.** mocari defines traits only. Users implement them with their preferred AI backend (OpenAI, ONNX, llama.cpp, etc.).
- **Feature-gated.** `ai` feature enables the module; no impact on builds without it.
- **Composable.** Multiple drivers can run simultaneously. Drivers don't know about each other.
- **Reuses existing pipelines.** `RiggedModel` conversion goes through `load_model_from_bytes()` — no special rendering path.

## Module Structure

```
src/ai/
├── mod.rs      // pub use exports, feature gate
├── rigger.rs   // AiRigger trait + RiggedModel data types
├── driver.rs   // AiDriver trait
└── error.rs    // RigError
```

`Cargo.toml`:
```toml
[features]
ai = []  # no external deps
```

`lib.rs`:
```rust
#[cfg(feature = "ai")]
pub mod ai;
```

## Rigging Pipeline

### Data Types

```rust
/// Complete model data produced by an AI rigger.
pub struct RiggedModel {
    pub textures: Vec<Vec<u8>>,              // PNG bytes, one per layer
    pub meshes: Vec<RiggedMesh>,             // per-layer mesh data
    pub parameters: Vec<RiggedParameter>,    // animatable parameters
    pub deformers: Vec<RiggedDeformer>,      // rotation/warp deformers
    pub physics: Option<Vec<u8>>,            // physics3.json bytes (optional)
    pub motions: Vec<(String, Vec<u8>)>,     // (name, motion3.json bytes)
    pub expressions: Vec<(String, Vec<u8>)>, // (name, exp3.json bytes)
}

pub struct RiggedMesh {
    pub texture_index: usize,
    pub vertices: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
    pub opacity: f32,
}

pub struct RiggedParameter {
    pub id: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub keyframes: Vec<ParameterKeyframe>,
}

pub struct ParameterKeyframe {
    pub time: f32,
    pub value: f32,
    pub interpolation: InterpolationType,
}

pub enum InterpolationType {
    Linear,
    Bezier([f32; 4]),
    Stepped,
}

pub struct RiggedDeformer {
    pub id: String,
    pub deformer_type: DeformerType,
    pub children: Vec<DeformerChild>,
    pub origin: [f32; 2],
}

pub enum DeformerType {
    Rotation { angle_range: [f32; 2] },
    Warp { vertex_count: usize },
}

pub enum DeformerChild {
    Mesh(usize),
    Deformer(usize),
}
```

### Trait

```rust
pub trait AiRigger: Send + Sync {
    /// Rig from a single character image (PNG/JPEG bytes).
    fn rig_from_image(&self, image: &[u8]) -> Result<RiggedModel, RigError>;

    /// Rig from a layered PSD file.
    fn rig_from_psd(&self, psd: &[u8]) -> Result<RiggedModel, RigError>;

    /// Rig from a text description.
    fn rig_from_description(&self, prompt: &str) -> Result<RiggedModel, RigError>;
}
```

### Engine Integration

```rust
impl Live2dEngine {
    /// Converts a RiggedModel into a live model using existing pipelines.
    ///
    /// Internally: RiggedModel → moc3 binary + model3.json → load_model_from_bytes()
    pub fn load_rigged_model(&mut self, rigged: &RiggedModel) -> Result<ModelHandle, EngineError>;
}
```

The conversion pipeline:
1. Encode `RiggedModel` → moc3 binary (new encoder in `src/moc3/encode.rs`)
2. Generate `model3.json` from parameters, textures, motions, expressions
3. Call existing `load_model_from_bytes()` which handles GPU resource creation

The moc3 encoder is a new component. The existing `src/moc3/` module only parses moc3 → GPU data; this adds the reverse direction. The encoder writes the binary format defined by the Cubism SDK spec (header, counts, sections, keyform bindings, etc.).

No special rendering code — the rigged model goes through the same path as any pre-built model.

## Runtime Drivers

### Trait

```rust
/// A per-frame AI driver that injects parameter changes.
///
/// Multiple drivers can be registered simultaneously.
/// They run in registration order, before the engine's tick.
pub trait AiDriver: Send + Sync {
    /// Called every frame before engine tick.
    /// Use `model.set_parameter()` to drive the character.
    fn update(&mut self, delta: f32, model: &mut RuntimeModel);
}
```

### Engine Integration

```rust
impl Live2dEngine {
    pub fn add_driver(&mut self, driver: Box<dyn AiDriver>);
    pub fn remove_driver(&mut self, index: usize);
}
```

In `tick()`:
```rust
pub fn tick(&mut self, delta: f32) {
    // Drivers run first — they set parameters that the animation system may override
    for model in &mut self.models {
        for driver in &mut self.drivers {
            driver.update(delta, &mut model.runtime);
        }
    }
    // ... existing tick logic (auto-systems, motions, physics)
}
```

### Example Driver Implementations

These are NOT shipped with mocari — users write them:

| Driver | Input Source | Parameters Driven |
|--------|-------------|-------------------|
| `EmotionDriver` | LLM emotion labels | `ParamHappy`, `ParamSad`, `ParamAngry`, etc. |
| `VoiceDriver` | TTS amplitude / phonemes | `ParamMouthOpenY`, `ParamMouthForm` |
| `FaceDriver` | MediaPipe landmarks | `ParamAngleX/Y/Z`, `ParamEyeBallX/Y`, `ParamBodyAngleX` |

Usage:
```rust
engine.add_driver(Box::new(EmotionDriver::new(llm_client)));
engine.add_driver(Box::new(VoiceDriver::new(audio_source)));
// Both run every frame, independently
```

## Error Handling

```rust
pub enum RigError {
    /// Input image/PSD could not be decoded.
    InvalidInput(String),
    /// AI inference failed.
    InferenceFailed(String),
    /// Output data is invalid or incomplete.
    InvalidOutput(String),
}
```

`load_rigged_model` returns the existing `EngineError` since it delegates to internal pipelines.

## Testing Strategy

- **Unit tests**: `RiggedModel` serialization round-trip (to/from moc3 + model3.json)
- **Mock rigger**: `struct MockRigger` returns hardcoded `RiggedModel` → verify `load_rigged_model` produces a valid model
- **Mock driver**: `struct MockDriver { called: bool }` → verify `tick()` invokes drivers in order
- **No AI inference tests**: actual model inference is the user's responsibility

## Files Changed

| File | Action |
|------|--------|
| `Cargo.toml` | Add `ai = []` feature |
| `src/lib.rs` | Add `#[cfg(feature = "ai")] pub mod ai;` |
| `src/ai/mod.rs` | NEW: module root, re-exports |
| `src/ai/rigger.rs` | NEW: `AiRigger` trait + `RiggedModel` types |
| `src/ai/driver.rs` | NEW: `AiDriver` trait |
| `src/ai/error.rs` | NEW: `RigError` |
| `src/engine/mod.rs` | Add `drivers` field, `add_driver()`, `load_rigged_model()`, update `tick()` |
| `src/moc3/encode.rs` | NEW: moc3 binary encoder (RiggedModel → moc3 bytes) |
