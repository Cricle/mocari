# High-Level wgpu API Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a high-level `Live2dEngine` API that encapsulates wgpu setup, rendering pipeline, mesh management, and animation coordination, reducing the example from 1987 lines to ~50 lines.

**Architecture:** Composable engine + model handles pattern. The engine owns wgpu device/surface/renderer and manages the full rendering pipeline. Models are loaded via `engine.load_model()` and return lightweight handles. Extension through callbacks and a `Live2dPlugin` trait.

**Tech Stack:** wgpu 30, winit 0.30, tokio (for async device creation)

## Global Constraints

- `#![forbid(unsafe_code)]` — no unsafe code anywhere
- Desktop only: Win/Linux/macOS via winit + wgpu
- Feature-gated: `wgpu` feature enables the high-level API
- Backward compatible: existing `WgpuLive2dRenderer` and low-level APIs remain available
- No breaking changes to `ModelRuntime`, `MotionPlayer`, `ExpressionManager`, etc.
- Rust trait-based plugin system with future FFI bridge capability

---

## Architecture Overview

```
Live2dEngine
├── WgpuContext          — owns Device, Queue, Surface, SurfaceConfiguration
├── WgpuLive2dRenderer   — existing renderer (pipelines, bind group layouts)
├── MeshManager          — per-model mesh buffers, clipping resources, textures
├── AnimationCoordinator — per-model MotionPlayer, ExpressionManager, Physics, Pose, auto-systems
└── PluginManager        — registered Live2dPlugin instances

ModelHandle              — lightweight reference (model index + id)
├── id()                 — model identifier
└── (methods on Live2dEngine that take &ModelHandle):
    ├── engine.play_motion(handle, group)
    ├── engine.play_expression(handle, name)
    ├── engine.set_parameter(handle, id, value)
    ├── engine.set_scale(handle, f32)
    ├── engine.model(handle)      — &ModelRuntime
    └── engine.model_mut(handle)  — &mut ModelRuntime

Live2dPlugin trait
├── on_frame(&mut self, ctx: &mut FrameContext)
├── on_render(&mut self, ctx: &mut RenderContext)
├── on_model_loaded(&mut self, model: &ModelHandle)
└── on_model_unloaded(&mut self, model_id: &str)
```

---

## Core Types

### `Live2dEngine`

The main entry point. Owns all GPU state and manages models.

```rust
pub struct Live2dEngine {
    ctx: WgpuContext,
    renderer: WgpuLive2dRenderer,
    models: Vec<LoadedModel>,
    plugins: Vec<Box<dyn Live2dPlugin>>,
    frame_callbacks: Vec<Box<dyn FnMut(&mut FrameContext)>>,
    render_callbacks: Vec<Box<dyn FnMut(&mut RenderContext)>>,
    last_delta: f32,
    needs_redraw: bool,
}
```

**Construction:**
```rust
impl Live2dEngine {
    pub async fn new(window: Arc<Window>) -> Result<Self, EngineError>;
}
```

Internally:
1. Creates `wgpu::Instance`
2. Creates `wgpu::Surface` from window
3. Requests `wgpu::Adapter` (HighPerformance, compatible with surface)
4. Requests `wgpu::Device` + `wgpu::Queue`
5. Configures surface (preferred format, present mode)
6. Creates `WgpuLive2dRenderer`
7. Returns engine

**Key methods:**
```rust
impl Live2dEngine {
    // Model lifecycle
    pub fn load_model(&mut self, path: &str) -> Result<ModelHandle, EngineError>;
    pub fn unload_model(&mut self, handle: &ModelHandle) -> bool;
    pub fn model(&self, handle: &ModelHandle) -> Option<&ModelRuntime>;
    pub fn model_mut(&mut self, handle: &ModelHandle) -> Option<&mut ModelRuntime>;

    // Model manipulation
    pub fn play_motion(&mut self, handle: &ModelHandle, group: &str) -> Result<(), EngineError>;
    pub fn play_expression(&mut self, handle: &ModelHandle, name: &str) -> Result<(), EngineError>;
    pub fn set_parameter(&mut self, handle: &ModelHandle, id: &str, value: f32);
    pub fn set_scale(&mut self, handle: &ModelHandle, scale: f32);
    pub fn configure_eye_blink(&mut self, handle: &ModelHandle, config: EyeBlinkConfig);
    pub fn configure_breath(&mut self, handle: &ModelHandle, config: BreathConfig);

    // Frame update + render
    pub fn tick(&mut self, delta: f32);
    pub fn render(&mut self) -> Result<(), EngineError>;
    pub fn resize(&mut self, size: PhysicalSize<u32>);
    pub fn needs_redraw(&self) -> bool;
    pub fn last_delta(&self) -> f32;

    // Extension points
    pub fn on_frame(&mut self, callback: impl FnMut(&mut FrameContext) + 'static);
    pub fn on_render(&mut self, callback: impl FnMut(&mut RenderContext) + 'static);
    pub fn add_plugin(&mut self, plugin: Box<dyn Live2dPlugin>);

    // Accessors
    pub fn device(&self) -> &wgpu::Device;
    pub fn queue(&self) -> &wgpu::Queue;
    pub fn renderer(&self) -> &WgpuLive2dRenderer;
}
```

### `WgpuContext`

Internal struct owning wgpu state.

```rust
struct WgpuContext {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}
```

### `LoadedModel`

Internal struct for a loaded model with all its GPU resources.

```rust
struct LoadedModel {
    id: String,
    path: PathBuf,
    runtime: ModelRuntime,
    motions: BTreeMap<String, Vec<PathBuf>>,
    expressions: Vec<PathBuf>,
    animation: AnimationState,
    mesh: MeshState,
    transform: WgpuTransform,
    bounds: ModelBounds,
    scale: f32,
    dirty: bool,
}
```

### `AnimationState`

Per-model animation state.

```rust
struct AnimationState {
    motion_player: Option<MotionPlayer>,
    expression_manager: ExpressionManager,
    eye_blink: Option<EyeBlink>,
    lip_sync: Option<LipSync>,
    breath: Option<Breath>,
    mouse_tracker: Option<MouseTracker>,
}
```

### `MeshState`

Per-model GPU mesh state.

```rust
struct MeshState {
    mesh_buffers: WgpuMeshBuffers,
    textures: Vec<WgpuTexture>,
    clipping_resources: WgpuClippingResources,
    mask_target: WgpuMaskRenderTarget,
}
```

### `ModelHandle`

Lightweight reference to a loaded model. All model manipulation goes through `Live2dEngine` methods that accept `&ModelHandle`.

```rust
pub struct ModelHandle {
    index: usize,
    id: String,
}

impl ModelHandle {
    pub fn id(&self) -> &str;
}
```

**Model manipulation via engine:**
```rust
// Play motion from a group (plays first motion in group)
engine.play_motion(&handle, "Idle")?;

// Play named expression
engine.play_expression(&handle, "happy")?;

// Set parameter value
engine.set_parameter(&handle, "ParamAngleX", 15.0);

// Set model scale
engine.set_scale(&handle, 1.5);

// Direct runtime access for advanced use
let runtime: &ModelRuntime = engine.model(&handle)?;
let runtime: &mut ModelRuntime = engine.model_mut(&handle)?;

// Configure auto-systems
engine.configure_eye_blink(&handle, EyeBlinkConfig { enabled: false, ..Default::default() });
engine.configure_breath(&handle, BreathConfig { enabled: true, ..Default::default() });
```

### `ModelBounds`

Computed from drawable vertices for auto-centering.

```rust
struct ModelBounds {
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}
```

---

## Plugin System

### `Live2dPlugin` Trait

```rust
pub trait Live2dPlugin: Send + Sync {
    fn on_frame(&mut self, _ctx: &mut FrameContext) {}
    fn on_render(&mut self, _ctx: &mut RenderContext) {}
    fn on_model_loaded(&mut self, _model: &ModelHandle) {}
    fn on_model_unloaded(&mut self, _model_id: &str) {}
}
```

### `FrameContext`

Passed to frame callbacks and plugin `on_frame`.

```rust
pub struct FrameContext<'a> {
    models: &'a mut [LoadedModel],
    delta: f32,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl FrameContext<'_> {
    pub fn delta(&self) -> f32;
    pub fn device(&self) -> &wgpu::Device;
    pub fn queue(&self) -> &wgpu::Queue;
    pub fn model(&self, handle: &ModelHandle) -> Option<&ModelRuntime>;
    pub fn model_mut(&mut self, handle: &ModelHandle) -> Option<&mut ModelRuntime>;
}
```

### `RenderContext`

Passed to render callbacks and plugin `on_render`.

```rust
pub struct RenderContext<'a> {
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    renderer: &'a WgpuLive2dRenderer,
    device: &'a wgpu::Device,
    queue: &'a wgpu::Queue,
}

impl RenderContext<'_> {
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder;
    pub fn view(&self) -> &wgpu::TextureView;
    pub fn renderer(&self) -> &WgpuLive2dRenderer;
    pub fn device(&self) -> &wgpu::Device;
    pub fn queue(&self) -> &wgpu::Queue;
}
```

---

## Rendering Pipeline

### `engine.render()` internals:

```
1. Acquire surface texture
2. Create command encoder

3. Mask pass:
   - For each model with clipping:
     - Begin render pass targeting mask_target
     - renderer.draw_masks_with_textures()

4. Main pass:
   - Begin render pass targeting frame view (clear to background color)
   - For each model:
     - renderer.draw_with_textures_clipping_and_transform()
   - Execute user render callbacks
   - Execute plugin on_render()

5. Submit command encoder
6. Present frame
7. needs_redraw = false
```

### `engine.resize()` internals:

```
1. Update surface config (width, height)
2. Reconfigure surface
3. For each model:
   - Recalculate fit_model_matrix()
   - Update transform
```

### `engine.tick()` internals:

```
1. For each model:
   a. If dirty or animating:
      - Reset parameters and part opacities
      - Tick motion player, apply if finished → remove
      - Tick expression manager, apply
      - Apply parameter overrides
      - Apply physics (if configured)
      - Apply pose (if configured)
      - Update meshes (fast path: update_drawables, slow path: rebuild)
      - Update clipping resources if bounds/visibility changed
      - Set needs_redraw = true

2. Execute frame callbacks
3. Execute plugin on_frame()
```

**Note:** `tick()` advances simulation state. `render()` draws the current state. This separation allows headless ticking (e.g., for MCP server or testing) without a surface.

---

## Model Auto-Systems

When loading a model, the engine automatically configures:

- **EyeBlink** — if model has eye parameters (ParamEyeLOpen, ParamEyeROpen, ParamEyeOpen)
- **Breath** — always enabled with default config
- **Physics** — if .model3.json references a physics file
- **Pose** — if .model3.json references a pose file

Users can disable/configure these per-model:
```rust
engine.configure_eye_blink(&handle, EyeBlinkConfig { enabled: false, ..Default::default() });
engine.configure_breath(&handle, BreathConfig { enabled: true, ..Default::default() });
```

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("wgpu surface error: {0}")]
    Surface(String),
    #[error("no suitable wgpu adapter found")]
    NoAdapter,
    #[error("wgpu device request failed: {0}")]
    Device(String),
    #[error("model load failed: {0}")]
    ModelLoad(String),
    #[error("model not found: {0}")]
    ModelNotFound(String),
    #[error("render error: {0}")]
    Render(#[from] WgpuRenderError),
}
```

---

## Module Structure

```
src/
├── engine/
│   ├── mod.rs           — Live2dEngine, ModelHandle, EngineError
│   ├── context.rs       — WgpuContext (device, queue, surface, config)
│   ├── model.rs         — LoadedModel, AnimationState, MeshState, ModelBounds
│   ├── plugin.rs        — Live2dPlugin trait, FrameContext, RenderContext, PluginManager
│   └── render.rs        — Rendering pipeline orchestration
├── render/
│   ├── common/          — (existing, unchanged)
│   └── wgpu/            — (existing, unchanged)
├── lib.rs               — adds `pub mod engine` (behind `wgpu` feature)
└── ...                  — (existing, unchanged)
```

The `prelude` module re-exports for convenience:
```rust
// src/prelude.rs (or inline in lib.rs)
pub use crate::engine::{Live2dEngine, ModelHandle, EngineError};
pub use crate::engine::plugin::{Live2dPlugin, FrameContext, RenderContext};
```

---

## Cargo.toml Changes

```toml
[features]
wgpu = ["dep:wgpu", "dep:pollster"]
```

Add `pollster` as a dependency (for `block_on` in engine creation):
```toml
pollster = { version = "0.4", optional = true }
```

---

## Testing Strategy

- **Unit tests**: `ModelBounds`, `fit_model_matrix()`, resize logic
- **Integration tests**: Load model, tick, render (using noop wgpu backend)
- **Example test**: Verify the simplified example compiles and runs

---

## Migration Path

- Existing `WgpuLive2dRenderer` and low-level APIs remain available
- Users can adopt `Live2dEngine` incrementally
- The example is updated to use `Live2dEngine`
- Old example code can be preserved as `examples/show_model_raw.rs` for users who want full control

---

## Example: Before vs After

### Before (1987 lines):
- Manual wgpu Instance/Adapter/Device/Queue setup
- Manual surface configuration
- Manual render pass orchestration
- Manual clipping resource management
- Manual mesh buffer lifecycle
- Manual animation coordination
- Manual transform/bounds calculation
- 16-field `LoadedModel` struct

### After (~50 lines):
```rust
let mut engine = Live2dEngine::new(window).await?;
let handle = engine.load_model("model.model3.json")?;
engine.play_motion(&handle, "Idle")?;
// ... event loop calls engine.tick(delta) and engine.render()
```
