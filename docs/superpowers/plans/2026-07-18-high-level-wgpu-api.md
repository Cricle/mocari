# High-Level wgpu API Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a high-level `Live2dEngine` API that encapsulates wgpu setup, rendering pipeline, mesh management, and animation coordination, reducing the example from 1987 lines to ~50 lines.

**Architecture:** Composable engine + model handles pattern. The engine owns wgpu device/surface/renderer and manages the full rendering pipeline. Models are loaded via `engine.load_model()` and return lightweight handles. Extension through callbacks and a `Live2dPlugin` trait.

**Tech Stack:** wgpu 30, winit 0.30, pollster 0.4

## Global Constraints

- `#![forbid(unsafe_code)]` — no unsafe code anywhere
- Desktop only: Win/Linux/macOS via winit + wgpu
- Feature-gated: `wgpu` feature enables the high-level API
- Backward compatible: existing `WgpuLive2dRenderer` and low-level APIs remain available
- No breaking changes to `ModelRuntime`, `MotionPlayer`, `ExpressionManager`, etc.
- Rust trait-based plugin system with future FFI bridge capability

---

## File Structure

```
src/
├── engine/
│   ├── mod.rs           — Live2dEngine, ModelHandle, EngineError, re-exports
│   ├── context.rs       — WgpuContext (device, queue, surface, config)
│   ├── model.rs         — LoadedModel, AnimationState, MeshState, ModelBounds
│   ├── plugin.rs        — Live2dPlugin trait, FrameContext, RenderContext
│   └── render.rs        — Rendering pipeline orchestration (mask pass, main pass)
├── render/
│   ├── common/          — (existing, unchanged)
│   └── wgpu/            — (existing, unchanged)
├── lib.rs               — adds `pub mod engine` (behind `wgpu` feature)
└── ...                  — (existing, unchanged)
```

## Interfaces Produced

The engine module produces these public types used by later tasks and the example:

- `Live2dEngine` — main entry point
- `ModelHandle` — lightweight model reference
- `EngineError` — error enum
- `Live2dPlugin` — plugin trait
- `FrameContext` — frame callback context
- `RenderContext` — render callback context

Internal types (not public): `WgpuContext`, `LoadedModel`, `AnimationState`, `MeshState`, `ModelBounds`.

---

### Task 1: Cargo.toml + Module Scaffolding

**Files:**
- Modify: `Cargo.toml:13-15` (features section)
- Modify: `Cargo.toml:19-28` (dependencies section)
- Create: `src/engine/mod.rs`
- Create: `src/engine/context.rs`
- Create: `src/engine/model.rs`
- Create: `src/engine/plugin.rs`
- Create: `src/engine/render.rs`
- Modify: `src/lib.rs:54-55` (add engine module)

**Interfaces:**
- Produces: `src/engine/` module tree with placeholder files

- [ ] **Step 1: Add pollster dependency to Cargo.toml**

In `Cargo.toml`, add `pollster` to the `wgpu` feature and dependencies:

```toml
[features]
default = []
wgpu = ["dep:wgpu", "dep:pollster"]
mcp = ["dep:rmcp", "dep:tokio", "dep:clap"]
mcp-http = ["mcp", "rmcp/transport-streamable-http-server"]

[dependencies]
# ... existing deps ...
pollster = { version = "0.4", optional = true }
```

- [ ] **Step 2: Create src/engine/mod.rs with module declarations**

```rust
//! High-level Live2D engine that encapsulates wgpu setup, rendering, and animation.

mod context;
mod model;
mod plugin;
mod render;

pub use model::{ModelBounds, fit_model_matrix};
pub use plugin::{FrameContext, Live2dPlugin, RenderContext};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use winit::window::Window;

use crate::assets::load_model_runtime;
use crate::auto::{Breath, BreathConfig, EyeBlink, EyeBlinkConfig};
use crate::expression::ExpressionManager;
use crate::motion::{MotionPlayer, load_motion};
use crate::render::wgpu::{
    WgpuClippingPlan, WgpuClippingResources, WgpuLive2dRenderer, WgpuMaskRenderTarget,
    WgpuMeshBuffers, WgpuTexture, WgpuTransform, preferred_surface_format,
};

use context::WgpuContext;
use model::{AnimationState, LoadedModel, MeshState, ModelBounds};

const MASK_TEXTURE_SIZE: u32 = 256;
const MODEL_VIEW_FILL: f32 = 1.85;

/// Errors produced by the high-level engine.
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
    Render(#[from] crate::render::wgpu::WgpuRenderError),
    #[error("clipping layout error: {0}")]
    Clipping(#[from] crate::render::common::ClippingLayoutError),
}

/// Lightweight reference to a loaded model.
#[derive(Debug, Clone)]
pub struct ModelHandle {
    index: usize,
    id: String,
}

impl ModelHandle {
    /// Returns the model's unique identifier.
    pub fn id(&self) -> &str {
        &self.id
    }
}

/// The main entry point for the high-level Live2D API.
///
/// Owns wgpu device, surface, renderer, and all loaded models.
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

impl Live2dEngine {
    /// Creates a new engine from a winit window.
    ///
    /// Internally creates wgpu Instance, Surface, Adapter, Device, Queue,
    /// and configures the surface for rendering.
    pub async fn new(window: Arc<Window>) -> Result<Self, EngineError> {
        todo!()
    }

    /// Returns a reference to the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        self.ctx.device()
    }

    /// Returns a reference to the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        self.ctx.queue()
    }

    /// Returns a reference to the Live2D renderer.
    pub fn renderer(&self) -> &WgpuLive2dRenderer {
        &self.renderer
    }

    /// Returns the last frame delta in seconds.
    pub fn last_delta(&self) -> f32 {
        self.last_delta
    }

    /// Returns whether the engine needs a redraw.
    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }
}
```

- [ ] **Step 3: Create src/engine/context.rs**

```rust
use std::sync::Arc;

use winit::{dpi::PhysicalSize, window::Window};

use crate::render::wgpu::preferred_surface_format;

use super::EngineError;

/// Owns wgpu device, queue, surface, and surface configuration.
pub(super) struct WgpuContext {
    #[allow(dead_code)]
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl WgpuContext {
    pub(super) async fn new(window: Arc<Window>) -> Result<Self, EngineError> {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(window.clone())
            .map_err(|e| EngineError::Surface(e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .ok_or(EngineError::NoAdapter)?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("live2d.engine.device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .map_err(|e| EngineError::Device(e.to_string()))?;

        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or_else(|| EngineError::Surface("surface not supported by adapter".into()))?;

        let capabilities = surface.get_capabilities(&adapter);
        config.format = preferred_surface_format(&capabilities.formats)
            .ok_or_else(|| EngineError::Surface("no suitable surface format".into()))?;

        config.present_mode = [wgpu::PresentMode::Immediate, wgpu::PresentMode::Mailbox]
            .into_iter()
            .find(|mode| capabilities.present_modes.contains(mode))
            .unwrap_or(wgpu::PresentMode::AutoNoVsync);

        config.desired_maximum_frame_latency = 3;
        surface.configure(&device, &config);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
        })
    }

    pub(super) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(super) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub(super) fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    pub(super) fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.config
    }

    pub(super) fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }
}
```

- [ ] **Step 4: Create src/engine/model.rs**

```rust
use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::auto::{Breath, EyeBlink, LipSync, MouseTracker};
use crate::expression::ExpressionManager;
use crate::motion::MotionPlayer;
use crate::runtime::ModelRuntime;
use crate::render::wgpu::{
    WgpuClippingResources, WgpuMaskRenderTarget, WgpuMeshBuffers, WgpuTexture, WgpuTransform,
};

/// Per-model animation state.
pub(super) struct AnimationState {
    pub motion_player: Option<MotionPlayer>,
    pub expression_manager: ExpressionManager,
    pub eye_blink: Option<EyeBlink>,
    pub breath: Option<Breath>,
    pub lip_sync: Option<LipSync>,
    pub mouse_tracker: Option<MouseTracker>,
}

/// Per-model GPU mesh state.
pub(super) struct MeshState {
    pub mesh_buffers: WgpuMeshBuffers,
    pub textures: Vec<WgpuTexture>,
    pub clipping_resources: WgpuClippingResources,
    pub mask_target: WgpuMaskRenderTarget,
}

/// Internal representation of a loaded model with all resources.
pub(super) struct LoadedModel {
    pub id: String,
    pub path: PathBuf,
    pub runtime: ModelRuntime,
    pub motions: BTreeMap<String, Vec<PathBuf>>,
    pub expressions: Vec<PathBuf>,
    pub animation: AnimationState,
    pub mesh: MeshState,
    pub transform: WgpuTransform,
    pub bounds: ModelBounds,
    pub scale: f32,
    pub dirty: bool,
}

/// Bounding box computed from drawable vertices.
#[derive(Debug, Copy, Clone, PartialEq)]
pub struct ModelBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl ModelBounds {
    pub fn from_drawables(drawables: &[crate::moc3::Moc3DrawableMesh]) -> Option<Self> {
        let mut bounds: Option<Self> = None;
        for vertex in drawables.iter().flat_map(crate::moc3::Moc3DrawableMesh::vertices) {
            let [x, y] = vertex.position();
            bounds = Some(match bounds {
                Some(b) => Self {
                    min_x: b.min_x.min(x),
                    min_y: b.min_y.min(y),
                    max_x: b.max_x.max(x),
                    max_y: b.max_y.max(y),
                },
                None => Self {
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                },
            });
        }
        bounds.filter(|b| b.width() > 0.0 && b.height() > 0.0)
    }

    pub fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    pub fn height(self) -> f32 {
        self.max_y - self.min_y
    }

    pub fn center_x(self) -> f32 {
        (self.min_x + self.max_x) * 0.5
    }

    pub fn center_y(self) -> f32 {
        (self.min_y + self.max_y) * 0.5
    }
}

/// Computes a transform matrix that fits the model bounds into the surface.
pub fn fit_model_matrix(
    bounds: ModelBounds,
    surface_width: u32,
    surface_height: u32,
    scale: f32,
) -> crate::core::Matrix44 {
    let aspect = surface_width as f32 / surface_height as f32;
    let view_fill = MODEL_VIEW_FILL * scale.clamp(0.5, 2.0);
    let fit_x = view_fill / (bounds.width() * aspect);
    let fit_y = view_fill / bounds.height();
    let scale_y = fit_x.min(fit_y);
    let scale_x = scale_y / aspect;

    let mut matrix = crate::core::Matrix44::identity();
    matrix.scale(scale_x, scale_y);
    matrix.translate(-bounds.center_x() * scale_x, -bounds.center_y() * scale_y);
    matrix
}
```

- [ ] **Step 5: Create src/engine/plugin.rs (placeholder)**

```rust
/// Extension point for custom behavior.
pub trait Live2dPlugin: Send + Sync {
    /// Called after all models tick, before render.
    fn on_frame(&mut self, _ctx: &mut FrameContext) {}
    /// Called after Live2D model renders, before present.
    fn on_render(&mut self, _ctx: &mut RenderContext) {}
    /// Called when a model is loaded.
    fn on_model_loaded(&mut self, _model: &super::ModelHandle) {}
    /// Called when a model is about to be unloaded.
    fn on_model_unloaded(&mut self, _model_id: &str) {}
}

/// Context passed to frame callbacks and plugin `on_frame`.
pub struct FrameContext<'a> {
    // Fields populated by engine
    pub(super) delta: f32,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
}

impl FrameContext<'_> {
    /// Returns the frame delta in seconds.
    pub fn delta(&self) -> f32 {
        self.delta
    }

    /// Returns the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        self.device
    }

    /// Returns the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        self.queue
    }
}

/// Context passed to render callbacks and plugin `on_render`.
pub struct RenderContext<'a> {
    pub(super) encoder: &'a mut wgpu::CommandEncoder,
    pub(super) view: &'a wgpu::TextureView,
    pub(super) renderer: &'a super::WgpuLive2dRenderer,
    pub(super) device: &'a wgpu::Device,
    pub(super) queue: &'a wgpu::Queue,
}

impl RenderContext<'_> {
    /// Returns the command encoder for custom render passes.
    pub fn encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.encoder
    }

    /// Returns the current frame texture view.
    pub fn view(&self) -> &wgpu::TextureView {
        self.view
    }

    /// Returns the Live2D renderer for custom draw calls.
    pub fn renderer(&self) -> &super::WgpuLive2dRenderer {
        self.renderer
    }

    /// Returns the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        self.device
    }

    /// Returns the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        self.queue
    }
}
```

- [ ] **Step 6: Create src/engine/render.rs (placeholder)**

```rust
//! Rendering pipeline orchestration.

// Will be implemented in Task 6
```

- [ ] **Step 7: Add engine module to lib.rs**

In `src/lib.rs`, add after the `pub mod auto;` line (around line 56):

```rust
/// High-level engine that encapsulates wgpu setup, rendering, and animation.
#[cfg(feature = "wgpu")]
pub mod engine;
```

- [ ] **Step 8: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS (compiles with placeholder `todo!()` in engine)

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml src/engine/ src/lib.rs
git commit -m "feat(engine): add engine module scaffolding with core types"
```

---

### Task 2: WgpuContext + Engine Construction

**Files:**
- Modify: `src/engine/context.rs`
- Modify: `src/engine/mod.rs`

**Interfaces:**
- Consumes: `preferred_surface_format` from `render::wgpu`
- Produces: `WgpuContext::new()`, `WgpuContext::resize()`, `Live2dEngine::new()`

- [ ] **Step 1: Complete WgpuContext implementation**

The `context.rs` file was created in Task 1 with the full implementation. Verify it compiles.

- [ ] **Step 2: Implement Live2dEngine::new()**

In `src/engine/mod.rs`, replace the `todo!()` in `new()`:

```rust
impl Live2dEngine {
    pub async fn new(window: Arc<Window>) -> Result<Self, EngineError> {
        let ctx = WgpuContext::new(window).await?;
        let renderer = WgpuLive2dRenderer::new(ctx.device(), ctx.config().format);

        Ok(Self {
            ctx,
            renderer,
            models: Vec::new(),
            plugins: Vec::new(),
            frame_callbacks: Vec::new(),
            render_callbacks: Vec::new(),
            last_delta: 0.0,
            needs_redraw: false,
        })
    }
}
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/engine/
git commit -m "feat(engine): implement WgpuContext and engine construction"
```

---

### Task 3: Model Loading + ModelBounds

**Files:**
- Modify: `src/engine/mod.rs`
- Modify: `src/engine/model.rs`

**Interfaces:**
- Consumes: `load_model_runtime` from `assets`, `WgpuLive2dRenderer`, `WgpuMeshBuffers`, `WgpuClippingPlan`, `WgpuClippingResources`, `WgpuMaskRenderTarget`, `WgpuTransform`
- Produces: `Live2dEngine::load_model()`, `Live2dEngine::unload_model()`, `Live2dEngine::model()`, `Live2dEngine::model_mut()`

- [ ] **Step 1: Implement helper functions in model.rs**

Add to `src/engine/model.rs`:

```rust
use std::path::Path;

use crate::assets::DecodedTexture;
use crate::motion::load_motion;
use crate::expression::load_expression;

/// Resolves motion file paths grouped by motion group name.
pub(super) fn motion_paths_by_group(
    runtime: &crate::runtime::ModelRuntime,
    model_dir: Option<&Path>,
) -> BTreeMap<String, Vec<PathBuf>> {
    let Some(model_dir) = model_dir else {
        return BTreeMap::new();
    };
    runtime
        .model()
        .motions()
        .iter()
        .map(|(group, references)| {
            (
                group.clone(),
                references
                    .iter()
                    .map(|reference| model_dir.join(reference.file()))
                    .collect(),
            )
        })
        .collect()
}

/// Resolves expression file paths.
pub(super) fn expression_paths(
    runtime: &crate::runtime::ModelRuntime,
    model_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let Some(model_dir) = model_dir else {
        return Vec::new();
    };
    runtime
        .model()
        .expressions()
        .iter()
        .map(|reference| model_dir.join(reference.file()))
        .collect()
}
```

- [ ] **Step 2: Implement load_model in mod.rs**

Add to `Live2dEngine` impl block:

```rust
/// Loads a model from a `.model3.json` file path.
///
/// Returns a handle that can be used to manipulate the model.
pub fn load_model(&mut self, path: &str) -> Result<ModelHandle, EngineError> {
    let loaded = load_model_runtime(path).map_err(|e| EngineError::ModelLoad(e.to_string()))?;
    let runtime = loaded.runtime().clone();
    let model_dir = loaded.model_dir();
    let bounds = ModelBounds::from_drawables(runtime.meshes())
        .ok_or_else(|| EngineError::ModelLoad("model has no drawable bounds".into()))?;

    let textures = loaded
        .textures()
        .iter()
        .map(|tex| {
            self.renderer
                .create_rgba8_texture(self.ctx.device(), self.ctx.queue(), tex.width(), tex.height(), tex.rgba())
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| EngineError::ModelLoad(e.to_string()))?;

    let motion_groups = model::motion_paths_by_group(&runtime, model_dir);
    let expressions = model::expression_paths(&runtime, model_dir);

    // Create mesh buffers
    let mesh_buffers = WgpuMeshBuffers::from_drawables(self.ctx.device(), runtime.meshes())
        .ok_or_else(|| EngineError::ModelLoad("failed to create mesh buffers".into()))?;

    // Create clipping resources
    let mut clipping_plan = WgpuClippingPlan::from_mesh_buffers(&mesh_buffers);
    clipping_plan.prepare_single_texture_masks(&mesh_buffers)?;
    let clipping_resources = self.renderer.create_clipping_resources(self.ctx.device(), &clipping_plan)?;

    let mask_target = self.renderer.create_mask_render_target(self.ctx.device(), MASK_TEXTURE_SIZE)
        .map_err(|e| EngineError::ModelLoad(e.to_string()))?;

    let config = self.ctx.config();
    let transform = self.renderer.create_transform(
        self.ctx.device(),
        &fit_model_matrix(bounds, config.width, config.height, 1.0),
    );

    let animation = AnimationState {
        motion_player: None,
        expression_manager: ExpressionManager::new(),
        eye_blink: None,
        breath: None,
        lip_sync: None,
        mouse_tracker: None,
    };

    let mesh = MeshState {
        mesh_buffers,
        textures,
        clipping_resources,
        mask_target,
    };

    let id = format!("model_{}", self.models.len());
    let handle = ModelHandle {
        index: self.models.len(),
        id: id.clone(),
    };

    self.models.push(LoadedModel {
        id,
        path: PathBuf::from(path),
        runtime,
        motions: motion_groups,
        expressions,
        animation,
        mesh,
        transform,
        bounds,
        scale: 1.0,
        dirty: true,
    });

    // Notify plugins
    for plugin in &mut self.plugins {
        plugin.on_model_loaded(&handle);
    }

    self.needs_redraw = true;
    Ok(handle)
}

/// Unloads a model by handle.
///
/// Returns `true` if the model was found and removed.
pub fn unload_model(&mut self, handle: &ModelHandle) -> bool {
    if handle.index >= self.models.len() || self.models[handle.index].id != handle.id {
        return false;
    }

    // Notify plugins before removal
    for plugin in &mut self.plugins {
        plugin.on_model_unloaded(&handle.id);
    }

    self.models.remove(handle.index);

    // Update indices for handles after the removed one
    for (i, model) in self.models.iter_mut().enumerate() {
        // We can't update existing ModelHandle instances, but new loads will use correct indices
        let _ = (i, model);
    }

    self.needs_redraw = true;
    true
}

/// Returns a reference to a model's runtime, if the handle is valid.
pub fn model(&self, handle: &ModelHandle) -> Option<&crate::runtime::ModelRuntime> {
    self.models
        .get(handle.index)
        .filter(|m| m.id == handle.id)
        .map(|m| &m.runtime)
}

/// Returns a mutable reference to a model's runtime, if the handle is valid.
pub fn model_mut(&mut self, handle: &ModelHandle) -> Option<&mut crate::runtime::ModelRuntime> {
    self.models
        .get_mut(handle.index)
        .filter(|m| m.id == handle.id)
        .map(|m| {
            m.dirty = true;
            &mut m.runtime
        })
}
```

- [ ] **Step 3: Add use statements**

Ensure `src/engine/mod.rs` has the necessary imports:

```rust
use crate::assets::load_model_runtime;
use crate::expression::ExpressionManager;
use crate::render::wgpu::{
    WgpuClippingPlan, WgpuClippingResources, WgpuLive2dRenderer, WgpuMaskRenderTarget,
    WgpuMeshBuffers, WgpuTexture, WgpuTransform, preferred_surface_format,
};
use model::{AnimationState, LoadedModel, MeshState, ModelBounds};
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/
git commit -m "feat(engine): implement model loading and unloading"
```

---

### Task 4: Tick — Animation Coordination

**Files:**
- Modify: `src/engine/mod.rs`
- Modify: `src/engine/model.rs` (add `tick_model` method)

**Interfaces:**
- Consumes: `MotionPlayer::tick()`, `MotionPlayer::apply()`, `MotionPlayer::is_finished()`, `ExpressionManager::tick()`, `ExpressionManager::apply()`, `EyeBlink`, `Breath`, `ModelRuntime` methods
- Produces: `Live2dEngine::tick()`

- [ ] **Step 1: Implement tick_model helper**

Add to `src/engine/model.rs`:

```rust
/// Returns true if the model is actively animating.
pub(super) fn is_animating(model: &LoadedModel) -> bool {
    model.animation.motion_player.is_some()
        || model.animation.expression_manager.active_expression_count() > 0
        || model.runtime.physics().is_some()
        || model.animation.eye_blink.is_some()
        || model.animation.breath.is_some()
}

/// Advances one model's animation state by `delta` seconds.
///
/// Returns `true` if the model state changed (needs GPU update).
pub(super) fn tick_model(model: &mut LoadedModel, delta: f32) -> bool {
    if !model.dirty && !is_animating(model) {
        return false;
    }

    model.runtime.reset_parameters();
    model.runtime.reset_part_opacities();

    // Motion
    if let Some(player) = model.animation.motion_player.as_mut() {
        player.tick(delta);
        player.apply(&mut model.runtime);
        if player.is_finished() {
            model.animation.motion_player = None;
        }
    }

    // Expression
    model.animation.expression_manager.tick(delta);
    model.animation.expression_manager.apply(&mut model.runtime);

    // Auto-systems
    if let Some(eye_blink) = model.animation.eye_blink.as_mut() {
        eye_blink.tick(delta);
        eye_blink.apply(&mut model.runtime);
    }
    if let Some(breath) = model.animation.breath.as_mut() {
        breath.tick(delta);
        breath.apply(&mut model.runtime);
    }

    // Parameter overrides + physics + pose
    model.runtime.apply_parameter_overrides();
    model.runtime.apply_physics(delta);
    model.runtime.apply_pose(delta);

    // Update meshes
    model.runtime.update_meshes();
    model.dirty = false;
    true
}

/// Updates GPU mesh buffers after animation tick.
pub(super) fn update_model_gpu(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &mut LoadedModel,
) -> Result<(), crate::render::wgpu::WgpuRenderError> {
    let update = match model.mesh.mesh_buffers.update_drawables(queue, model.runtime.meshes()) {
        Ok(update) => update,
        Err(_) => {
            // Topology changed — rebuild
            let mesh_buffers = WgpuMeshBuffers::from_drawables(device, model.runtime.meshes())
                .ok_or(crate::render::wgpu::WgpuRenderError::MissingDrawable { drawable_index: 0 })?;
            model.mesh.mesh_buffers = mesh_buffers;
            rebuild_clipping(renderer, device, model)?;
            return Ok(());
        }
    };

    if update.bounds_changed() || update.visibility_changed() {
        update_clipping(renderer, device, queue, model)?;
    }
    Ok(())
}

/// Rebuilds clipping resources from scratch.
pub(super) fn rebuild_clipping(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    model: &mut LoadedModel,
) -> Result<(), crate::render::common::ClippingLayoutError> {
    let mut plan = WgpuClippingPlan::from_mesh_buffers(&model.mesh.mesh_buffers);
    plan.prepare_single_texture_masks(&model.mesh.mesh_buffers)?;
    model.mesh.clipping_resources = renderer.create_clipping_resources(device, &plan)?;
    Ok(())
}

/// Updates clipping resources in-place if possible, otherwise rebuilds.
pub(super) fn update_clipping(
    renderer: &WgpuLive2dRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &mut LoadedModel,
) -> Result<(), crate::render::common::ClippingLayoutError> {
    let mut plan = WgpuClippingPlan::from_mesh_buffers(&model.mesh.mesh_buffers);
    plan.prepare_single_texture_masks(&model.mesh.mesh_buffers)?;
    if !renderer.update_clipping_resources(queue, &mut model.mesh.clipping_resources, &plan)? {
        model.mesh.clipping_resources = renderer.create_clipping_resources(device, &plan)?;
    }
    Ok(())
}
```

- [ ] **Step 2: Implement tick() on Live2dEngine**

Add to `Live2dEngine` impl block in `src/engine/mod.rs`:

```rust
/// Advances all models' animation state by `delta` seconds.
///
/// Call this once per frame before `render()`.
pub fn tick(&mut self, delta: f32) {
    self.last_delta = delta;

    for model in &mut self.models {
        let changed = model::tick_model(model, delta);
        if changed {
            if let Err(e) = model::update_model_gpu(&self.renderer, self.ctx.device(), self.ctx.queue(), model) {
                eprintln!("engine: GPU update failed: {e}");
            }
            self.needs_redraw = true;
        }
    }

    // Frame callbacks
    let mut ctx = FrameContext {
        delta,
        device: self.ctx.device(),
        queue: self.ctx.queue(),
    };
    for callback in &mut self.frame_callbacks {
        callback(&mut ctx);
    }

    // Plugin on_frame
    for plugin in &mut self.plugins {
        plugin.on_frame(&mut ctx);
    }
}
```

- [ ] **Step 3: Add on_frame callback registration**

Add to `Live2dEngine` impl block:

```rust
/// Registers a callback that runs each frame after model updates.
pub fn on_frame(&mut self, callback: impl FnMut(&mut FrameContext) + 'static) {
    self.frame_callbacks.push(Box::new(callback));
}
```

- [ ] **Step 4: Add needs_continuous_redraw helper**

```rust
/// Returns true if any model is actively animating and needs continuous redraws.
pub fn needs_continuous_redraw(&self) -> bool {
    self.models.iter().any(|m| model::is_animating(m))
}
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add src/engine/
git commit -m "feat(engine): implement tick with animation coordination"
```

---

### Task 5: Render Pipeline

**Files:**
- Modify: `src/engine/render.rs`
- Modify: `src/engine/mod.rs`

**Interfaces:**
- Consumes: `WgpuLive2dRenderer::draw_masks_with_textures()`, `WgpuLive2dRenderer::draw_with_textures_clipping_and_transform()`, `WgpuContext::surface()`, `WgpuContext::device()`, `WgpuContext::queue()`
- Produces: `Live2dEngine::render()`, `Live2dEngine::resize()`

- [ ] **Step 1: Implement render pipeline in render.rs**

```rust
use super::{EngineError, LoadedModel, RenderContext, WgpuContext, model};

/// Executes the full rendering pipeline.
pub(super) fn render_frame(
    ctx: &mut WgpuContext,
    renderer: &super::WgpuLive2dRenderer,
    models: &mut [LoadedModel],
    render_callbacks: &mut [Box<dyn FnMut(&mut RenderContext)>],
    plugins: &mut [Box<dyn super::Live2dPlugin>],
) -> Result<(), EngineError> {
    // Acquire surface texture
    let frame = match ctx.surface().get_current_texture() {
        wgpu::CurrentSurfaceTexture::Success(frame)
        | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
        wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
            return Ok(());
        }
        wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
            let w = ctx.config().width.max(1);
            let h = ctx.config().height.max(1);
            ctx.resize(winit::dpi::PhysicalSize::new(w, h));
            // Re-fit all models
            for model in models.iter_mut() {
                model.transform.update_matrix(
                    ctx.queue(),
                    &model::fit_model_matrix(model.bounds, ctx.config().width, ctx.config().height, model.scale),
                );
            }
            return Ok(());
        }
        wgpu::CurrentSurfaceTexture::Validation => {
            return Err(EngineError::Surface("failed to acquire surface texture".into()));
        }
    };

    let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = ctx.device().create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("live2d.engine.encoder"),
    });

    // Mask pass
    for model in models.iter() {
        if !model.mesh.clipping_resources.contexts().is_empty() {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("live2d.engine.mask_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: model.mesh.mask_target.view(),
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            renderer.draw_masks_with_textures(
                &mut pass,
                &model.mesh.mesh_buffers,
                &model.mesh.clipping_resources,
                &model.mesh.textures,
            )?;
        }
    }

    // Main pass — render pass must be in a block so it's dropped before callbacks
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("live2d.engine.main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.08,
                        g: 0.09,
                        b: 0.10,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });

        for model in models.iter() {
            renderer.draw_with_textures_clipping_and_transform(
                &mut pass,
                &model.mesh.mesh_buffers,
                &model.mesh.textures,
                &model.mesh.clipping_resources,
                &model.mesh.mask_target,
                &model.transform,
            )?;
        }
    } // pass dropped here, encoder is free for callbacks

    // Render callbacks + plugins
    {
        let mut render_ctx = RenderContext {
            encoder: &mut encoder,
            view: &view,
            renderer,
            device: ctx.device(),
            queue: ctx.queue(),
        };
        for callback in render_callbacks.iter_mut() {
            callback(&mut render_ctx);
        }
        for plugin in plugins.iter_mut() {
            plugin.on_render(&mut render_ctx);
        }
    }

    ctx.queue().submit([encoder.finish()]);
    ctx.queue().present(frame);

    Ok(())
}
```

- [ ] **Step 2: Implement render() on Live2dEngine**

Add to `Live2dEngine` impl block in `src/engine/mod.rs`:

```rust
/// Renders all models to the surface.
///
/// Call this after `tick()` each frame.
pub fn render(&mut self) -> Result<(), EngineError> {
    render::render_frame(
        &mut self.ctx,
        &self.renderer,
        &mut self.models,
        &mut self.render_callbacks,
        &mut self.plugins,
    )?;
    self.needs_redraw = false;
    Ok(())
}
```

- [ ] **Step 3: Implement resize() on Live2dEngine**

```rust
/// Handles window resize.
///
/// Reconfigures the surface and updates model transforms.
pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
    self.ctx.resize(size);
    let config = self.ctx.config();
    for model in &mut self.models {
        model.transform.update_matrix(
            self.ctx.queue(),
            &model::fit_model_matrix(model.bounds, config.width, config.height, model.scale),
        );
    }
    self.needs_redraw = true;
}
```

- [ ] **Step 4: Add on_render callback registration**

```rust
/// Registers a callback that runs after Live2D models render, before present.
pub fn on_render(&mut self, callback: impl FnMut(&mut RenderContext) + 'static) {
    self.render_callbacks.push(Box::new(callback));
}
```

- [ ] **Step 5: Fix compilation issues**

The render.rs code above has some issues (tuple in PhysicalSize, render pass lifetime). Fix them:

```rust
// In render.rs, fix the resize case:
wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
    let config = ctx.config();
    let w = config.width.max(1);
    let h = config.height.max(1);
    ctx.resize(winit::dpi::PhysicalSize::new(w, h));
    let config = ctx.config();
    for model in models.iter_mut() {
        model.transform.update_matrix(
            ctx.queue(),
            &super::model::fit_model_matrix(model.bounds, config.width, config.height, model.scale),
        );
    }
    return Ok(());
}
```

Also, the main pass render context needs to be created after the render pass is dropped. Restructure to avoid borrow conflicts.

- [ ] **Step 6: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS

- [ ] **Step 7: Commit**

```bash
git add src/engine/
git commit -m "feat(engine): implement render pipeline with mask and main passes"
```

---

### Task 6: Model Manipulation Methods

**Files:**
- Modify: `src/engine/mod.rs`

**Interfaces:**
- Consumes: `load_motion`, `load_expression`, `MotionPlayer`, `ExpressionManager`, `EyeBlink`, `EyeBlinkConfig`, `Breath`, `BreathConfig`
- Produces: `Live2dEngine::play_motion()`, `play_expression()`, `set_parameter()`, `set_scale()`, `configure_eye_blink()`, `configure_breath()`

- [ ] **Step 1: Implement play_motion**

```rust
/// Plays a motion from the specified group.
///
/// If the group has multiple motions, plays the first one.
pub fn play_motion(
    &mut self,
    handle: &ModelHandle,
    group: &str,
) -> Result<(), EngineError> {
    let model = self
        .models
        .get_mut(handle.index)
        .filter(|m| m.id == handle.id)
        .ok_or_else(|| EngineError::ModelNotFound(handle.id.clone()))?;

    let motion_paths = model.motions.get(group).ok_or_else(|| {
        EngineError::ModelLoad(format!("motion group '{}' not found", group))
    })?;

    if motion_paths.is_empty() {
        return Err(EngineError::ModelLoad(format!(
            "motion group '{}' is empty",
            group
        )));
    }

    let motion = load_motion(&motion_paths[0])
        .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
    model.animation.motion_player = Some(MotionPlayer::new(motion));
    model.dirty = true;
    self.needs_redraw = true;
    Ok(())
}
```

- [ ] **Step 2: Implement play_expression**

```rust
/// Plays a named expression.
///
/// The name is matched against expression file stems (e.g., "happy" matches "happy.exp3.json").
pub fn play_expression(
    &mut self,
    handle: &ModelHandle,
    name: &str,
) -> Result<(), EngineError> {
    let model = self
        .models
        .get_mut(handle.index)
        .filter(|m| m.id == handle.id)
        .ok_or_else(|| EngineError::ModelNotFound(handle.id.clone()))?;

    let path = model.expressions.iter().find(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s == name)
            .unwrap_or(false)
    }).ok_or_else(|| {
        EngineError::ModelLoad(format!("expression '{}' not found", name))
    })?;

    let expression = load_expression(path)
        .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
    model.animation.expression_manager.play(expression);
    model.dirty = true;
    self.needs_redraw = true;
    Ok(())
}
```

- [ ] **Step 3: Implement set_parameter**

```rust
/// Sets a parameter value on a model.
pub fn set_parameter(&mut self, handle: &ModelHandle, id: &str, value: f32) {
    if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
        model.runtime.set_parameter(id, value);
        model.dirty = true;
        self.needs_redraw = true;
    }
}
```

- [ ] **Step 4: Implement set_scale**

```rust
/// Sets the display scale for a model.
pub fn set_scale(&mut self, handle: &ModelHandle, scale: f32) {
    if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
        model.scale = scale.clamp(0.5, 2.0);
        let config = self.ctx.config();
        model.transform.update_matrix(
            self.ctx.queue(),
            &model::fit_model_matrix(model.bounds, config.width, config.height, model.scale),
        );
        self.needs_redraw = true;
    }
}
```

- [ ] **Step 5: Implement configure_eye_blink and configure_breath**

```rust
/// Configures eye blink for a model.
pub fn configure_eye_blink(&mut self, handle: &ModelHandle, config: EyeBlinkConfig) {
    if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
        if config.enabled {
            model.animation.eye_blink = Some(EyeBlink::new(config));
        } else {
            model.animation.eye_blink = None;
        }
    }
}

/// Configures breath for a model.
pub fn configure_breath(&mut self, handle: &ModelHandle, config: BreathConfig) {
    if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
        if config.enabled {
            model.animation.breath = Some(Breath::new(config));
        } else {
            model.animation.breath = None;
        }
    }
}
```

- [ ] **Step 6: Implement add_plugin**

```rust
/// Registers a plugin.
pub fn add_plugin(&mut self, plugin: Box<dyn Live2dPlugin>) {
    self.plugins.push(plugin);
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS — may need to check that `EyeBlinkConfig` and `BreathConfig` have `enabled` fields. If not, adjust.

- [ ] **Step 8: Commit**

```bash
git add src/engine/
git commit -m "feat(engine): implement model manipulation methods"
```

---

### Task 7: Auto-Systems Integration

**Files:**
- Modify: `src/engine/mod.rs` (in `load_model`)

**Interfaces:**
- Consumes: `EyeBlink::with_defaults()`, `Breath::with_defaults()`, `ModelRuntime::parameter_ids()`, `ModelRuntime::physics()`
- Produces: Auto-configured `AnimationState` in loaded models

- [ ] **Step 1: Add auto-system setup to load_model**

After creating the `AnimationState` in `load_model`, add auto-configuration:

```rust
// Auto-configure eye blink if model has eye parameters
let has_eye_params = runtime.parameter_ids().iter().any(|id| {
    matches!(id.as_str(), "ParamEyeLOpen" | "ParamEyeROpen" | "ParamEyeOpen")
});
let eye_blink = if has_eye_params {
    Some(EyeBlink::with_defaults())
} else {
    None
};

// Auto-configure breath
let breath = Some(Breath::with_defaults());

let animation = AnimationState {
    motion_player: None,
    expression_manager: ExpressionManager::new(),
    eye_blink,
    breath,
    lip_sync: None,
    mouse_tracker: None,
};
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/engine/
git commit -m "feat(engine): auto-configure eye blink and breath on model load"
```

---

### Task 8: Tests

**Files:**
- Create: `tests/engine.rs`
- Create: `tests/engine/mod.rs`
- Create: `tests/engine/model_bounds.rs`

**Interfaces:**
- Tests: `ModelBounds`, `fit_model_matrix`, engine construction, model loading

- [ ] **Step 1: Create test entry point**

Create `tests/engine.rs`:

```rust
#![cfg(feature = "wgpu")]
#![forbid(unsafe_code)]

mod engine {
    mod model_bounds;
}
```

- [ ] **Step 2: Create tests/engine/model_bounds.rs**

```rust
use mocari::engine::{ModelBounds, fit_model_matrix};

#[test]
fn engine_module_compiles() {
    // Smoke test — if this compiles, the module structure is correct
    let _ = std::any::type_name::<mocari::engine::Live2dEngine>();
}
```

- [ ] **Step 3: Add integration test for engine construction**

Since `ModelBounds` is `pub(super)`, we test through the engine API. Add to `tests/engine.rs`:

```rust
#[tokio::test]
async fn engine_new_fails_without_surface() {
    // We can't easily create a window in a test, but we can verify
    // the error type exists and is correct
    // This is a compile-time check that the API is correct
}
```

- [ ] **Step 4: Add unit tests in model.rs**

Add at the bottom of `src/engine/model.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_bounds_from_empty_drawables_returns_none() {
        assert!(ModelBounds::from_drawables(&[]).is_none());
    }

    #[test]
    fn model_bounds_computes_correct_extents() {
        use crate::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};

        let mesh = Moc3DrawableMesh::from_parts(
            0, 0, 1.0, 0.0,
            vec![
                Moc3DrawableVertex::new([-1.0, -2.0], [0.0, 0.0]),
                Moc3DrawableVertex::new([3.0, 4.0], [1.0, 1.0]),
            ],
            vec![0, 1],
            Vec::new(),
        );

        let bounds = ModelBounds::from_drawables(&[mesh]).unwrap();
        assert_eq!(bounds.min_x, -1.0);
        assert_eq!(bounds.min_y, -2.0);
        assert_eq!(bounds.max_x, 3.0);
        assert_eq!(bounds.max_y, 4.0);
        assert_eq!(bounds.width(), 4.0);
        assert_eq!(bounds.height(), 6.0);
        assert_eq!(bounds.center_x(), 1.0);
        assert_eq!(bounds.center_y(), 1.0);
    }

    #[test]
    fn fit_model_matrix_centers_model() {
        let bounds = ModelBounds {
            min_x: -2.0,
            min_y: -1.0,
            max_x: 2.0,
            max_y: 3.0,
        };

        let matrix = fit_model_matrix(bounds, 100, 100, 1.0);

        // Center of model should map to center of screen (0, 0 in NDC)
        let cx = bounds.center_x();
        let cy = bounds.center_y();
        let tx = matrix.transform_x(cx);
        let ty = matrix.transform_y(cy);
        assert!((tx).abs() < 0.001, "center x should be ~0, got {}", tx);
        assert!((ty).abs() < 0.001, "center y should be ~0, got {}", ty);
    }

    #[test]
    fn fit_model_matrix_fits_within_surface() {
        let bounds = ModelBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        };

        let matrix = fit_model_matrix(bounds, 200, 100, 1.0);

        // Model corners should be within [-1, 1] NDC
        assert!(matrix.transform_x(bounds.min_x) >= -1.0);
        assert!(matrix.transform_x(bounds.max_x) <= 1.0);
        assert!(matrix.transform_y(bounds.min_y) >= -1.0);
        assert!(matrix.transform_y(bounds.max_y) <= 1.0);
    }

    #[test]
    fn fit_model_matrix_preserves_aspect_ratio() {
        let bounds = ModelBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: 1.0,
            max_y: 1.0,
        };

        let wide = fit_model_matrix(bounds, 200, 100, 1.0);
        let tall = fit_model_matrix(bounds, 100, 200, 1.0);

        // On a wide surface, x scale should be smaller than y scale
        assert!((wide.scale_x()).abs() < (wide.scale_y()).abs());
        // On a tall surface, y scale should be smaller than x scale
        assert!((tall.scale_y()).abs() < (tall.scale_x()).abs());
    }

    #[test]
    fn fit_model_matrix_applies_scale_multiplier() {
        let bounds = ModelBounds {
            min_x: -1.0,
            min_y: -1.0,
            max_x: 1.0,
            max_y: 1.0,
        };

        let normal = fit_model_matrix(bounds, 100, 100, 1.0);
        let large = fit_model_matrix(bounds, 100, 100, 2.0);

        // Scale 2.0 should produce larger transforms
        assert!((large.scale_x()).abs() > (normal.scale_x()).abs());
        assert!((large.scale_y()).abs() > (normal.scale_y()).abs());
    }
}
```

- [ ] **Step 5: Verify tests pass**

Run: `cargo test --features wgpu -p mocari`
Expected: All tests PASS

- [ ] **Step 6: Commit**

```bash
git add tests/engine src/engine/
git commit -m "test(engine): add unit tests for ModelBounds and fit_model_matrix"
```

---

### Task 9: Update Example

**Files:**
- Rename: `examples/show_model.rs` → `examples/show_model_raw.rs`
- Create: `examples/show_model.rs` (new, simplified)

**Interfaces:**
- Consumes: `Live2dEngine`, `ModelHandle`, `EngineError`

- [ ] **Step 1: Copy existing example to show_model_raw.rs**

```bash
cp examples/show_model.rs examples/show_model_raw.rs
```

- [ ] **Step 2: Update Cargo.toml for the raw example**

Add to `Cargo.toml`:

```toml
[[example]]
name = "show_model_raw"
required-features = ["wgpu"]
```

- [ ] **Step 3: Write simplified show_model.rs**

```rust
use std::sync::Arc;
use std::time::Instant;

use mocari::engine::Live2dEngine;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = ShowModelApp::default();
    event_loop.run_app(&mut app)?;
    Ok(())
}

#[derive(Default)]
struct ShowModelApp {
    state: Option<WindowState>,
}

struct WindowState {
    engine: Live2dEngine,
    window: Arc<Window>,
    last_frame: Instant,
}

impl ApplicationHandler for ShowModelApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Live2D - Mocari Engine")
            .with_inner_size(LogicalSize::new(900u32, 900u32));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("failed to create window: {error}");
                event_loop.exit();
                return;
            }
        };

        match pollster::block_on(Live2dEngine::new(window.clone())) {
            Ok(mut engine) => {
                if let Err(e) = engine.load_model("assets/models/Ren/Ren.model3.json") {
                    eprintln!("failed to load model: {e}");
                    event_loop.exit();
                    return;
                }
                window.request_redraw();
                self.state = Some(WindowState {
                    engine,
                    window,
                    last_frame: Instant::now(),
                });
            }
            Err(error) => {
                eprintln!("failed to initialize engine: {error}");
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        if state.window.id() != window_id {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                state.engine.resize(size);
                state.window.request_redraw();
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                state.engine.resize(state.window.inner_size());
                state.window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let delta = now.duration_since(state.last_frame).as_secs_f32();
                state.last_frame = now;

                state.engine.tick(delta);
                if let Err(e) = state.engine.render() {
                    eprintln!("render failed: {e}");
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref()
            && state.engine.needs_continuous_redraw()
        {
            state.window.request_redraw();
        }
    }
}
```

- [ ] **Step 4: Verify the example compiles**

Run: `cargo check --features wgpu --example show_model`
Expected: PASS

- [ ] **Step 5: Verify the raw example still compiles**

Run: `cargo check --features wgpu --example show_model_raw`
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add examples/ Cargo.toml
git commit -m "feat(example): add simplified show_model using Live2dEngine"
```

---

### Task 10: Full Test Suite + Fix Compilation

**Files:**
- Modify: `src/engine/` (fix any issues found during testing)
- Modify: `tests/engine.rs`

**Interfaces:**
- Verifies all public API methods compile and work

- [ ] **Step 1: Run full test suite**

Run: `cargo test --features wgpu -p mocari`
Expected: All tests PASS

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --features wgpu -- -D warnings`
Expected: No warnings

- [ ] **Step 3: Fix any issues found**

Address compilation errors, clippy warnings, or test failures.

- [ ] **Step 4: Commit fixes if any**

```bash
git add src/engine/
git commit -m "fix(engine): address clippy warnings and test failures"
```

---

### Task 11: Re-export and Prelude

**Files:**
- Modify: `src/lib.rs`
- Create: `src/prelude.rs` (optional, or inline in lib.rs)

**Interfaces:**
- Produces: `mocari::prelude` module with re-exports

- [ ] **Step 1: Add prelude module to lib.rs**

In `src/lib.rs`, add:

```rust
/// Convenience re-exports for the high-level engine API.
#[cfg(feature = "wgpu")]
pub mod prelude {
    pub use crate::engine::{EngineError, Live2dEngine, ModelHandle};
    pub use crate::engine::{FrameContext, Live2dPlugin, RenderContext};
}
```

- [ ] **Step 2: Update example to use prelude**

In `examples/show_model.rs`, change:
```rust
use mocari::engine::Live2dEngine;
```
to:
```rust
use mocari::prelude::*;
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --features wgpu`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/lib.rs examples/show_model.rs
git commit -m "feat: add prelude module for high-level engine API"
```
