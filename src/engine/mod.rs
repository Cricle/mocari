//! High-level Live2D engine that encapsulates wgpu setup, rendering, and animation.

mod context;
pub mod desktop_pet;
mod model;
mod plugin;
mod render;
#[cfg(target_arch = "wasm32")]
pub mod web;

pub use desktop_pet::DesktopPetConfig;
pub use model::{ModelBounds, fit_model_matrix};
pub use plugin::{FrameContext, Live2dPlugin, RenderContext};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use winit::window::Window;
#[cfg(target_arch = "wasm32")]
use web_sys;

use crate::assets::load_model_runtime;
use crate::auto::{Breath, BreathConfig, EyeBlink, EyeBlinkConfig, LipSync, LipSyncConfig, MouseTracker, MouseTrackerConfig};
use crate::expression::ExpressionManager;
use crate::motion::{MotionPlayer, load_motion};
use crate::runtime::ModelRuntime;
use crate::render::wgpu::{
    WgpuClippingPlan, WgpuLive2dRenderer, WgpuMeshBuffers, WgpuTexture,
};

use context::WgpuContext;
use model::{AnimationState, LoadedModel, MeshState};

const MASK_TEXTURE_SIZE: u32 = 256;
const MODEL_VIEW_FILL: f32 = 1.85;

type FrameCallback = Box<dyn FnMut(&mut FrameContext)>;
type RenderCallback = Box<dyn FnMut(&mut RenderContext)>;

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
    frame_callbacks: Vec<FrameCallback>,
    render_callbacks: Vec<RenderCallback>,
    clear_color: Option<wgpu::Color>,
    last_delta: f32,
    needs_redraw: bool,
    msaa_view: wgpu::TextureView,
    #[cfg(feature = "ai")]
    drivers: Vec<Box<dyn crate::ai::AiDriver>>,
}

impl Live2dEngine {
    /// Creates a new engine from a winit window.
    ///
    /// Internally creates wgpu Instance, Surface, Adapter, Device, Queue,
    /// and configures the surface for rendering.
    pub async fn new(window: Arc<Window>) -> Result<Self, EngineError> {
        let ctx = WgpuContext::new(window).await?;
        let renderer = WgpuLive2dRenderer::new(ctx.device(), ctx.config().format, 4);
        let config = ctx.config();
        let msaa_view = renderer.create_msaa_render_target(
            ctx.device(),
            config.format,
            config.width.max(1),
            config.height.max(1),
        );

        Ok(Self {
            ctx,
            renderer,
            models: Vec::new(),
            plugins: Vec::new(),
            frame_callbacks: Vec::new(),
            render_callbacks: Vec::new(),
            clear_color: None,
            last_delta: 0.0,
            needs_redraw: false,
            msaa_view,
            #[cfg(feature = "ai")]
            drivers: Vec::new(),
        })
    }

    /// Creates a new engine from pre-existing wgpu objects (no winit window needed).
    pub fn from_wgpu(
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        config: wgpu::SurfaceConfiguration,
    ) -> Self {
        let renderer = WgpuLive2dRenderer::new(&device, config.format, 4);
        let msaa_view = renderer.create_msaa_render_target(
            &device,
            config.format,
            config.width.max(1),
            config.height.max(1),
        );
        let ctx = WgpuContext::from_wgpu(device, queue, surface, config);

        Self {
            ctx,
            renderer,
            models: Vec::new(),
            plugins: Vec::new(),
            frame_callbacks: Vec::new(),
            render_callbacks: Vec::new(),
            clear_color: None,
            last_delta: 0.0,
            needs_redraw: false,
            msaa_view,
            #[cfg(feature = "ai")]
            drivers: Vec::new(),
        }
    }

    /// Creates an engine directly from an HTML canvas element (web only).
    ///
    /// Handles all wgpu initialization internally: instance, surface, adapter,
    /// device, queue, and surface configuration.
    #[cfg(target_arch = "wasm32")]
    pub async fn from_canvas(canvas: web_sys::HtmlCanvasElement) -> Result<Self, EngineError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|e| EngineError::Surface(e.to_string()))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await
            .map_err(|_| EngineError::NoAdapter)?;

        let limits = wgpu::Limits::downlevel_webgl2_defaults();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("mocari.device"),
                required_features: wgpu::Features::empty(),
                required_limits: limits,
                ..Default::default()
            })
            .await
            .map_err(|e| EngineError::Device(e.to_string()))?;

        let size = (canvas.width(), canvas.height());
        let mut config = surface
            .get_default_config(&adapter, size.0.max(1), size.1.max(1))
            .ok_or_else(|| EngineError::Surface("surface not supported".into()))?;
        let capabilities = surface.get_capabilities(&adapter);
        config.format = capabilities.formats[0];
        config.present_mode = [wgpu::PresentMode::Fifo, wgpu::PresentMode::Mailbox]
            .into_iter()
            .find(|mode| capabilities.present_modes.contains(mode))
            .unwrap_or(wgpu::PresentMode::AutoNoVsync);
        config.desired_maximum_frame_latency = 3;
        surface.configure(&device, &config);

        Ok(Self::from_wgpu(device, queue, surface, config))
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

    /// Loads a model from a `.model3.json` file path.
    pub fn load_model(&mut self, path: &str) -> Result<ModelHandle, EngineError> {
        let mut loaded = load_model_runtime(path).map_err(|e| EngineError::ModelLoad(e.to_string()))?;
        let runtime = loaded.runtime().clone();
        let model_dir = loaded.model_dir();
        let bounds = ModelBounds::from_drawables(runtime.meshes())
            .ok_or_else(|| EngineError::ModelLoad("model has no drawable bounds".into()))?;

        // Collect motion groups and expressions before clearing textures
        let motion_groups = model::motion_paths_by_group(&runtime, model_dir);
        let expressions = model::expression_paths(&runtime, model_dir);

        // Upload textures to GPU, then immediately drop CPU-side texture data
        let textures = self.create_textures(loaded.textures())?;
        loaded.clear_textures(); // Free ~50-100MB of CPU memory after GPU upload

        self.register_model(runtime, textures, bounds, PathBuf::from(path), motion_groups, expressions)
    }

    /// Unloads a model by handle. Returns true if found and removed.
    pub fn unload_model(&mut self, handle: &ModelHandle) -> bool {
        if handle.index >= self.models.len() || self.models[handle.index].id != handle.id {
            return false;
        }
        for plugin in &mut self.plugins {
            plugin.on_model_unloaded(&handle.id);
        }
        self.models.remove(handle.index);
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

    /// Renders all models to the surface.
    /// Call this after `tick()` each frame.
    pub fn render(&mut self) -> Result<(), EngineError> {
        render::render_frame(
            &mut self.ctx,
            &self.renderer,
            &mut self.models,
            &mut self.render_callbacks,
            &mut self.plugins,
            self.clear_color,
            &self.msaa_view,
        )?;
        self.needs_redraw = false;
        Ok(())
    }

    /// Handles window resize.
    /// Reconfigures the surface and updates model transforms.
    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.ctx.resize(size);
        let config = self.ctx.config();
        self.msaa_view = self.renderer.create_msaa_render_target(
            self.ctx.device(),
            config.format,
            config.width.max(1),
            config.height.max(1),
        );
        for m in &mut self.models {
            m.transform.update_matrix(
                self.ctx.queue(),
                &model::fit_model_matrix(m.bounds, config.width, config.height, m.scale),
            );
        }
        self.needs_redraw = true;
    }

    /// Registers a callback that runs after Live2D models render, before present.
    pub fn on_render(&mut self, callback: impl FnMut(&mut RenderContext) + 'static) {
        self.render_callbacks.push(Box::new(callback));
    }

    /// Advances all models' animation state by `delta` seconds.
    /// Call this once per frame before `render()`.
    pub fn tick(&mut self, delta: f32) {
        use std::sync::atomic::{AtomicU32, Ordering};
        static TICK_COUNT: AtomicU32 = AtomicU32::new(0);

        let count = TICK_COUNT.fetch_add(1, Ordering::Relaxed);
        // Sample every 30 frames (~0.5s) to catch blinks better
        if count == 0 || count % 30 == 0 {
            eprintln!("[DEBUG] tick() frame={}, delta={:.3}s", count + 1, delta);
            for (i, model) in self.models.iter().enumerate() {
                // Check actual parameter values
                if let Some(idx) = model.runtime.parameter_index("ParamEyeLOpen") {
                    let val = model.runtime.parameter_value_by_index(idx).unwrap_or(0.0);
                    eprintln!("[DEBUG]   model[{}] ParamEyeLOpen = {:.3}", i, val);
                }
                if let Some(idx) = model.runtime.parameter_index("ParamBodyAngleY") {
                    let val = model.runtime.parameter_value_by_index(idx).unwrap_or(0.0);
                    eprintln!("[DEBUG]   model[{}] ParamBodyAngleY = {:.3}", i, val);
                }

                // Check breath state
                if let Some(breath) = &model.animation.breath {
                    eprintln!("[DEBUG]   model[{}] breath exists, checking...", i);
                }
            }
        }

        self.last_delta = delta;

        for model in &mut self.models {
            let changed = model::tick_model(model, delta);
            if changed {
                if let Err(e) =
                    model::update_model_gpu(&self.renderer, self.ctx.device(), self.ctx.queue(), model)
                {
                    eprintln!("engine: GPU update failed: {e}");
                }
                self.needs_redraw = true;
            }
        }

        // Run AI drivers after model tick (parameters are already reset and
        // animations applied; drivers can still inject overrides for next frame).
        #[cfg(feature = "ai")]
        for driver in &mut self.drivers {
            for model in &mut self.models {
                driver.update(delta, &mut model.runtime);
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

    /// Registers a callback that runs each frame after model updates.
    pub fn on_frame(&mut self, callback: impl FnMut(&mut FrameContext) + 'static) {
        self.frame_callbacks.push(Box::new(callback));
    }

    /// Registers an AI driver that runs each frame for all loaded models.
    ///
    /// Drivers run in registration order, after model tick (motion, expressions,
    /// auto-systems) and before rendering. Each driver receives the mutable
    /// runtime so it can set parameters or override drawables.
    #[cfg(feature = "ai")]
    pub fn add_driver(&mut self, driver: impl crate::ai::AiDriver + 'static) {
        self.drivers.push(Box::new(driver));
    }

    /// Removes all AI drivers from the engine.
    #[cfg(feature = "ai")]
    pub fn clear_drivers(&mut self) {
        self.drivers.clear();
    }

    /// Returns true if any model is actively animating and needs continuous redraws.
    pub fn needs_continuous_redraw(&self) -> bool {
        self.models.iter().any(model::is_animating)
    }

    /// Plays a motion from the specified group.
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

    /// Plays a named expression.
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

        let expression = crate::expression::load_expression(path)
            .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
        model.animation.expression_manager.play(expression);
        model.dirty = true;
        self.needs_redraw = true;
        Ok(())
    }

    /// Loads a model from pre-fetched bytes (web-compatible).
    ///
    /// Provide the `.model3.json` content, the `.moc3` binary, and each
    /// referenced texture as PNG bytes in the order listed in the model JSON.
    pub fn load_model_from_bytes(
        &mut self,
        model_json: &str,
        moc3_bytes: &[u8],
        texture_pngs: &[&[u8]],
    ) -> Result<ModelHandle, EngineError> {
        let mut loaded = crate::assets::load_model_from_bytes(model_json, moc3_bytes, texture_pngs)
            .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
        let runtime = loaded.runtime().clone();
        let bounds = ModelBounds::from_drawables(runtime.meshes())
            .ok_or_else(|| EngineError::ModelLoad("model has no drawable bounds".into()))?;

        let textures = self.create_textures(loaded.textures())?;
        loaded.clear_textures(); // Free CPU memory after GPU upload

        self.register_model(runtime, textures, bounds, PathBuf::new(), BTreeMap::new(), Vec::new())
    }

    /// Loads an AI-rigged model directly into the engine.
    ///
    /// Converts the rigged model data into a runtime, decodes textures from
    /// PNG bytes, and sets up GPU resources. Returns a handle for controlling
    /// the model.
    #[cfg(feature = "ai")]
    pub fn load_rigged_model(
        &mut self,
        rigged: crate::ai::RiggedModel,
        model_json_config: Option<&crate::ai::ModelJsonConfig>,
    ) -> Result<ModelHandle, EngineError> {
        let config = model_json_config.cloned().unwrap_or_default();
        let json_str = crate::ai::generate_model_json(&rigged, &config);
        let model3 = crate::json::Model3::from_json_str(&json_str)
            .map_err(|e| EngineError::ModelLoad(e.to_string()))?;

        let texture_pngs: Vec<Vec<u8>> = rigged.textures.clone();
        let runtime = rigged
            .into_runtime(model3)
            .ok_or_else(|| EngineError::ModelLoad("failed to build runtime from rigged model".into()))?;

        let bounds = ModelBounds::from_drawables(runtime.meshes())
            .ok_or_else(|| EngineError::ModelLoad("model has no drawable bounds".into()))?;

        let mut textures = Vec::with_capacity(texture_pngs.len());
        for png_bytes in &texture_pngs {
            let img = image::load_from_memory(png_bytes)
                .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
            let rgba = img.to_rgba8();
            textures.push(
                self.renderer
                    .create_rgba8_texture(self.ctx.device(), self.ctx.queue(), rgba.width(), rgba.height(), rgba.as_raw())
                    .map_err(|e| EngineError::ModelLoad(e.to_string()))?,
            );
        }

        self.register_model(runtime, textures, bounds, PathBuf::new(), BTreeMap::new(), Vec::new())
    }

    fn create_textures(&self, decoded: &[crate::assets::DecodedTexture]) -> Result<Vec<WgpuTexture>, EngineError> {
        decoded
            .iter()
            .map(|tex| {
                self.renderer
                    .create_rgba8_texture(self.ctx.device(), self.ctx.queue(), tex.width(), tex.height(), tex.rgba())
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| EngineError::ModelLoad(e.to_string()))
    }

    fn register_model(
        &mut self,
        runtime: ModelRuntime,
        textures: Vec<WgpuTexture>,
        bounds: ModelBounds,
        path: PathBuf,
        motion_groups: BTreeMap<String, Vec<PathBuf>>,
        expressions: Vec<PathBuf>,
    ) -> Result<ModelHandle, EngineError> {
        let mesh_buffers = WgpuMeshBuffers::from_drawables(self.ctx.device(), runtime.meshes())
            .ok_or_else(|| EngineError::ModelLoad("failed to create mesh buffers".into()))?;

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

        let animation = Self::build_animation_state(&runtime);

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
            path,
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

        for plugin in &mut self.plugins {
            plugin.on_model_loaded(&handle);
        }

        self.needs_redraw = true;
        Ok(handle)
    }

    fn build_animation_state(runtime: &ModelRuntime) -> AnimationState {
        let has_eye_params = runtime.parameter_ids().iter().any(|id| {
            matches!(id.as_str(), "ParamEyeLOpen" | "ParamEyeROpen" | "ParamEyeOpen")
        });
        let eye_blink = if has_eye_params {
            Some(EyeBlink::with_defaults())
        } else {
            None
        };

        let breath = Some(Breath::with_defaults());

        let lip_sync_config = runtime.lip_sync_config_from_model();
        let lip_sync = if !lip_sync_config.parameter_indices.is_empty()
            || runtime.parameter_ids().iter().any(|id| id == "ParamMouthOpenY")
        {
            Some(LipSync::new(lip_sync_config))
        } else {
            None
        };

        let has_tracking_params = runtime.parameter_ids().iter().any(|id| {
            matches!(id.as_str(), "ParamAngleX" | "ParamAngleY" | "ParamEyeBallX" | "ParamEyeBallY")
        });
        let mouse_tracker = if has_tracking_params {
            Some(MouseTracker::with_defaults())
        } else {
            None
        };

        AnimationState {
            motion_player: None,
            expression_manager: ExpressionManager::new(),
            eye_blink,
            breath,
            lip_sync,
            mouse_tracker,
        }
    }

    /// Plays a motion from a JSON string (web-compatible).
    pub fn play_motion_from_json(
        &mut self,
        handle: &ModelHandle,
        motion_json: &str,
    ) -> Result<(), EngineError> {
        let model = self
            .models
            .get_mut(handle.index)
            .filter(|m| m.id == handle.id)
            .ok_or_else(|| EngineError::ModelNotFound(handle.id.clone()))?;

        let motion = crate::motion::load_motion_from_json(motion_json)
            .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
        model.animation.motion_player = Some(MotionPlayer::new(motion));
        model.dirty = true;
        self.needs_redraw = true;
        Ok(())
    }

    /// Plays an expression from a JSON string (web-compatible).
    pub fn play_expression_from_json(
        &mut self,
        handle: &ModelHandle,
        expression_json: &str,
    ) -> Result<(), EngineError> {
        let model = self
            .models
            .get_mut(handle.index)
            .filter(|m| m.id == handle.id)
            .ok_or_else(|| EngineError::ModelNotFound(handle.id.clone()))?;

        let expression = crate::expression::load_expression_from_json(expression_json)
            .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
        model.animation.expression_manager.play(expression);
        model.dirty = true;
        self.needs_redraw = true;
        Ok(())
    }

    /// Sets a parameter value on a model.
    pub fn set_parameter(&mut self, handle: &ModelHandle, id: &str, value: f32) {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
            model.runtime.set_parameter(id, value);
            model.dirty = true;
            self.needs_redraw = true;
        }
    }

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

    /// Configures eye blink for a model.
    /// Pass `Some(config)` to enable, `None` to disable.
    pub fn configure_eye_blink(&mut self, handle: &ModelHandle, config: Option<EyeBlinkConfig>) {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
            if let Some(config) = config {
                model.animation.eye_blink = Some(EyeBlink::new(config));
            } else {
                model.animation.eye_blink = None;
            }
        }
    }

    /// Configures breath for a model.
    /// Pass `Some(config)` to enable, `None` to disable.
    pub fn configure_breath(&mut self, handle: &ModelHandle, config: Option<BreathConfig>) {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
            if let Some(config) = config {
                model.animation.breath = Some(Breath::new(config));
            } else {
                model.animation.breath = None;
            }
        }
    }

    /// Configures lip sync for a model.
    /// Pass `Some(config)` to enable, `None` to disable.
    pub fn configure_lip_sync(&mut self, handle: &ModelHandle, config: Option<LipSyncConfig>) {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
            if let Some(config) = config {
                model.animation.lip_sync = Some(LipSync::new(config));
            } else {
                model.animation.lip_sync = None;
            }
        }
    }

    /// Sets the lip sync amplitude for a model.
    /// Values should be in `0.0..=1.0` from external audio analysis.
    pub fn set_lip_sync_amplitude(&mut self, handle: &ModelHandle, amplitude: f32) {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id)
            && let Some(lip_sync) = model.animation.lip_sync.as_mut()
        {
            lip_sync.set_amplitude(amplitude);
        }
    }

    /// Configures mouse tracking for a model.
    /// Pass `Some(config)` to enable, `None` to disable.
    pub fn configure_mouse_tracker(&mut self, handle: &ModelHandle, config: Option<MouseTrackerConfig>) {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id) {
            if let Some(config) = config {
                model.animation.mouse_tracker = Some(MouseTracker::new(config));
            } else {
                model.animation.mouse_tracker = None;
            }
        }
    }

    /// Sets the mouse tracker target position for a model.
    /// Coordinates are in normalized space: `-1.0..=1.0` where `(0, 0)` is center.
    pub fn set_mouse_tracker_target(&mut self, handle: &ModelHandle, x: f32, y: f32) {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id)
            && let Some(mouse_tracker) = model.animation.mouse_tracker.as_mut()
        {
            mouse_tracker.set_target(x, y);
        }
    }

    /// Drains motion events from a model's active motion player.
    ///
    /// Returns event values that fired since the last call. Call after `tick()`.
    pub fn drain_motion_events(&mut self, handle: &ModelHandle) -> Vec<String> {
        if let Some(model) = self.models.get_mut(handle.index).filter(|m| m.id == handle.id)
            && let Some(player) = model.animation.motion_player.as_mut()
        {
            return player.drain_events().into_iter().map(String::from).collect();
        }
        Vec::new()
    }

    /// Registers a plugin.
    pub fn add_plugin(&mut self, plugin: Box<dyn Live2dPlugin>) {
        self.plugins.push(plugin);
    }

    /// Returns a reference to the underlying window, if available.
    pub fn window(&self) -> Option<&Arc<Window>> {
        self.ctx.window()
    }

    /// Enables or disables click-through on the window.
    ///
    /// When enabled, mouse events pass through the window to whatever is behind it.
    pub fn set_click_through(&self, enabled: bool) -> Result<(), EngineError> {
        self.ctx
            .window()
            .ok_or_else(|| EngineError::Surface("no window available".into()))?
            .set_cursor_hittest(!enabled)
            .map_err(|e| EngineError::Surface(e.to_string()))
    }

    /// Sets the background clear color.
    ///
    /// Pass `None` for the default dark background, or `Some(Color::TRANSPARENT)`
    /// for transparent windows (e.g., desktop pets).
    pub fn set_clear_color(&mut self, color: Option<wgpu::Color>) {
        self.clear_color = color;
    }
}

/// Window configuration for [`run`] and [`run_with_config`].
///
/// Use builder methods to customize; defaults produce a standard 900×900 window.
pub struct RunConfig {
    pub title: String,
    pub width: u32,
    pub height: u32,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            title: "Live2D - Mocari".into(),
            width: 900,
            height: 900,
        }
    }
}

impl RunConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
}

/// Runs a Live2D model in a window with default settings.
///
/// This is the simplest way to display a model. It creates a window,
/// initializes the engine, loads the model, and runs the event loop
/// until the window is closed.
///
/// ```no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     Ok(mocari::engine::run("assets/models/Ren/Ren.model3.json")?)
/// }
/// ```
pub fn run(model_path: &str) -> Result<(), EngineError> {
    run_with_config(model_path, RunConfig::default())
}

/// Runs a Live2D model with custom window configuration.
///
/// ```no_run
/// use mocari::engine::RunConfig;
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     Ok(mocari::engine::run_with_config(
///         "assets/models/Ren/Ren.model3.json",
///         RunConfig::new().title("My Model").size(800, 600),
///     )?)
/// }
/// ```
pub fn run_with_config(model_path: &str, config: RunConfig) -> Result<(), EngineError> {
    let model_path = model_path.to_owned();

    let event_loop = winit::event_loop::EventLoop::new()
        .map_err(|e| EngineError::Surface(e.to_string()))?;
    event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let app = Live2dApp::new_regular(model_path, config);

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = app;
        event_loop
            .run_app(&mut app)
            .map_err(|e| EngineError::Surface(e.to_string()))
    }
}

/// Runs a model as a desktop pet with default pet settings.
///
/// Creates a transparent, frameless, always-on-top window with click-through
/// enabled. Clicking on the model area toggles click-through and allows
/// dragging the window. Press Escape to quit.
///
/// ```no_run
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     Ok(mocari::engine::run_desktop_pet("assets/models/Ren/Ren.model3.json")?)
/// }
/// ```
pub fn run_desktop_pet(model_path: &str) -> Result<(), EngineError> {
    run_desktop_pet_with_config(model_path, DesktopPetConfig::default())
}

/// Runs a desktop pet with custom configuration.
pub fn run_desktop_pet_with_config(
    model_path: &str,
    config: DesktopPetConfig,
) -> Result<(), EngineError> {
    let model_path = model_path.to_owned();

    let event_loop = winit::event_loop::EventLoop::new()
        .map_err(|e| EngineError::Surface(e.to_string()))?;
    // Don't set Wait mode - we need continuous updates for animations
    // event_loop.set_control_flow(winit::event_loop::ControlFlow::Wait);

    let mut app = Live2dApp::new_desktop_pet(model_path, config);

    event_loop
        .run_app(&mut app)
        .map_err(|e| EngineError::Surface(e.to_string()))
}

enum WindowConfig {
    Regular { title: String, width: u32, height: u32 },
    DesktopPet(DesktopPetConfig),
}

struct Live2dApp {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    model_path: String,
    window_config: WindowConfig,
    state: Option<LiveAppState>,
}

struct LiveAppState {
    engine: Live2dEngine,
    window: Arc<Window>,
    last_frame: std::time::Instant,
    is_interactive: bool,
}

impl Live2dApp {
    fn new_regular(model_path: String, config: RunConfig) -> Self {
        Self {
            model_path,
            window_config: WindowConfig::Regular {
                title: config.title,
                width: config.width,
                height: config.height,
            },
            state: None,
        }
    }

    fn new_desktop_pet(model_path: String, config: DesktopPetConfig) -> Self {
        Self {
            model_path,
            window_config: WindowConfig::DesktopPet(config),
            state: None,
        }
    }

    fn is_desktop_pet(&self) -> bool {
        matches!(self.window_config, WindowConfig::DesktopPet(_))
    }
}

impl winit::application::ApplicationHandler for Live2dApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let window = match &self.window_config {
            WindowConfig::Regular { title, width, height } => {
                let attrs = Window::default_attributes()
                    .with_title(title)
                    .with_inner_size(winit::dpi::LogicalSize::new(*width, *height));
                match event_loop.create_window(attrs) {
                    Ok(w) => Arc::new(w),
                    Err(e) => {
                        eprintln!("failed to create window: {e}");
                        event_loop.exit();
                        return;
                    }
                }
            }
            WindowConfig::DesktopPet(config) => {
                match config.create_window(event_loop) {
                    Ok(w) => w,
                    Err(e) => {
                        eprintln!("failed to create window: {e}");
                        event_loop.exit();
                        return;
                    }
                }
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.init_engine(event_loop, window);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = window;
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let is_pet = self.is_desktop_pet();
        let Some(state) = self.state.as_mut() else {
            return;
        };

        match event {
            winit::event::WindowEvent::CloseRequested => event_loop.exit(),
            winit::event::WindowEvent::Resized(size) => {
                state.engine.resize(size);
                state.window.request_redraw();
            }
            winit::event::WindowEvent::ScaleFactorChanged { .. } => {
                state.engine.resize(state.window.inner_size());
                state.window.request_redraw();
            }
            winit::event::WindowEvent::RedrawRequested => {
                let now = std::time::Instant::now();
                let delta = now.duration_since(state.last_frame).as_secs_f32();
                state.last_frame = now;

                state.engine.tick(delta);
                if let Err(e) = state.engine.render() {
                    eprintln!("render failed: {e}");
                    event_loop.exit();
                }
            }
            winit::event::WindowEvent::CursorMoved { position, .. } if is_pet => {
                let size = state.window.inner_size();
                let cx = size.width as f64 / 2.0;
                let cy = size.height as f64 / 2.0;
                let radius = (size.width.min(size.height) as f64) * 0.4;
                let dx = position.x - cx;
                let dy = position.y - cy;
                let over_model = dx * dx + dy * dy < radius * radius;

                if over_model && !state.is_interactive {
                    state.is_interactive = true;
                    let _ = state.engine.set_click_through(false);
                } else if !over_model && state.is_interactive {
                    state.is_interactive = false;
                    let _ = state.engine.set_click_through(true);
                }
            }
            winit::event::WindowEvent::MouseInput {
                state: button_state,
                button: winit::event::MouseButton::Left,
                ..
            } if is_pet => {
                if button_state == winit::event::ElementState::Pressed && state.is_interactive {
                    let _ = state.window.drag_window();
                }
            }
            winit::event::WindowEvent::KeyboardInput { event, .. } if is_pet
                && event.state == winit::event::ElementState::Pressed
                && event.logical_key
                    == winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape)
            => {
                event_loop.exit();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        if let Some(state) = self.state.as_ref()
            && state.engine.needs_continuous_redraw()
        {
            state.window.request_redraw();
        }
    }
}

impl Live2dApp {
    #[cfg(not(target_arch = "wasm32"))]
    fn init_engine(&mut self, event_loop: &winit::event_loop::ActiveEventLoop, window: Arc<Window>) {
        match pollster::block_on(Live2dEngine::new(window.clone())) {
            Ok(mut engine) => {
                if let WindowConfig::DesktopPet(ref config) = self.window_config {
                    engine.set_clear_color(config.clear_color);
                }
                if !self.model_path.is_empty() {
                    match engine.load_model(&self.model_path) {
                        Ok(handle) => {
                            // Enable all auto-animations for desktop pet
                            if self.is_desktop_pet() {
                                eprintln!("[DEBUG] Model loaded, configuring auto-animations...");

                                // Check if model has eye parameters
                                if let Some(model) = engine.models.get(handle.index) {
                                    let param_ids = model.runtime.parameter_ids();
                                    eprintln!("[DEBUG] Model has {} parameters", param_ids.len());
                                    for (i, id) in param_ids.iter().enumerate() {
                                        if id.contains("Eye") || id.contains("Mouth") || id.contains("Body") {
                                            eprintln!("[DEBUG] Parameter[{}]: {}", i, id);
                                        }
                                    }
                                }

                                // Configure more visible and lively animations for desktop pet
                                use crate::auto::{EyeBlinkConfig, BreathConfig};

                                // More frequent, slower eye blinks
                                let blink_config = EyeBlinkConfig {
                                    min_interval: 1.5,
                                    max_interval: 3.5,
                                    close_duration: 0.15,
                                    open_duration: 0.15,
                                    weight: 1.0,
                                    parameter_indices: Vec::new(),
                                };
                                engine.configure_eye_blink(&handle, Some(blink_config));

                                // Configure breath to use ParamBodyAngleY with stronger effect
                                if let Some(model) = engine.models.get(handle.index) {
                                    if let Some(idx) = model.runtime.parameter_index("ParamBodyAngleY") {
                                        let breath_config = BreathConfig {
                                            cycle_speed: 0.3,  // Faster breathing
                                            weight: 0.15,      // Stronger effect (was 1.0 but in 0-1 range)
                                            parameter_indices: vec![idx],
                                        };
                                        engine.configure_breath(&handle, Some(breath_config));
                                        eprintln!("[DEBUG] Breath configured for ParamBodyAngleY (index {})", idx);
                                    } else {
                                        engine.configure_breath(&handle, Some(Default::default()));
                                    }
                                } else {
                                    engine.configure_breath(&handle, Some(Default::default()));
                                }

                                engine.configure_lip_sync(&handle, Some(Default::default()));
                                engine.configure_mouse_tracker(&handle, Some(Default::default()));

                                eprintln!("[DEBUG] Auto-animations configured");

                                // Verify animations are set
                                if let Some(model) = engine.models.get(handle.index) {
                                    eprintln!("[DEBUG] eye_blink: {:?}", model.animation.eye_blink.is_some());
                                    eprintln!("[DEBUG] breath: {:?}", model.animation.breath.is_some());
                                    eprintln!("[DEBUG] lip_sync: {:?}", model.animation.lip_sync.is_some());
                                    eprintln!("[DEBUG] mouse_tracker: {:?}", model.animation.mouse_tracker.is_some());
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("failed to load model: {e}");
                            event_loop.exit();
                            return;
                        }
                    }
                }
                window.request_redraw();
                self.state = Some(LiveAppState {
                    engine,
                    window,
                    last_frame: std::time::Instant::now(),
                    is_interactive: false,
                });
            }
            Err(e) => {
                eprintln!("failed to initialize engine: {e}");
                event_loop.exit();
            }
        }
    }
}
