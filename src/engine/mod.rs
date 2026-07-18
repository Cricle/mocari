//! High-level Live2D engine that encapsulates wgpu setup, rendering, and animation.

mod context;
pub mod desktop_pet;
mod model;
mod plugin;
mod render;

pub use desktop_pet::DesktopPetConfig;
pub use model::{ModelBounds, fit_model_matrix};
pub use plugin::{FrameContext, Live2dPlugin, RenderContext};

use std::path::PathBuf;
use std::sync::Arc;

use winit::window::Window;

use crate::assets::load_model_runtime;
use crate::auto::{Breath, BreathConfig, EyeBlink, EyeBlinkConfig};
use crate::expression::ExpressionManager;
use crate::motion::{MotionPlayer, load_motion};
use crate::render::wgpu::{
    WgpuClippingPlan, WgpuLive2dRenderer, WgpuMeshBuffers,
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
    last_delta: f32,
    needs_redraw: bool,
    idle_fps: u32,
    last_render: Option<std::time::Instant>,
}

impl Live2dEngine {
    /// Creates a new engine from a winit window.
    ///
    /// Internally creates wgpu Instance, Surface, Adapter, Device, Queue,
    /// and configures the surface for rendering.
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
            idle_fps: 15,
            last_render: None,
        })
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

        for plugin in &mut self.plugins {
            plugin.on_model_loaded(&handle);
        }

        self.needs_redraw = true;
        Ok(handle)
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
        )?;
        self.needs_redraw = false;
        self.last_render = Some(std::time::Instant::now());
        Ok(())
    }

    /// Handles window resize.
    /// Reconfigures the surface and updates model transforms.
    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.ctx.resize(size);
        let config = self.ctx.config();
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

    /// Returns true if any model is actively animating and needs continuous redraws.
    pub fn needs_continuous_redraw(&self) -> bool {
        self.models.iter().any(model::is_animating)
    }

    /// Returns true if any model has active motion/expression (not just auto-systems).
    pub fn has_active_animation(&self) -> bool {
        self.models.iter().any(model::is_active)
    }

    /// Sets the frame rate cap when only auto-systems (eye blink, breath) are active.
    ///
    /// Default is 15 fps. Set to 0 to disable idle throttling.
    pub fn set_idle_fps(&mut self, fps: u32) {
        self.idle_fps = fps;
    }

    /// Returns the instant when the next frame should render.
    ///
    /// If active animation is running, returns now (full speed).
    /// If only auto-systems, returns now + idle interval.
    pub fn next_render_time(&self) -> std::time::Instant {
        let now = std::time::Instant::now();
        if self.has_active_animation() || self.idle_fps == 0 {
            return now;
        }
        let interval = std::time::Duration::from_secs_f64(1.0 / self.idle_fps as f64);
        match self.last_render {
            Some(last) => last + interval,
            None => now,
        }
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

    /// Registers a plugin.
    pub fn add_plugin(&mut self, plugin: Box<dyn Live2dPlugin>) {
        self.plugins.push(plugin);
    }

    /// Returns a reference to the underlying window.
    pub fn window(&self) -> &Arc<Window> {
        self.ctx.window()
    }

    /// Enables or disables click-through on the window.
    ///
    /// When enabled, mouse events pass through the window to whatever is behind it.
    pub fn set_click_through(&self, enabled: bool) -> Result<(), EngineError> {
        self.ctx
            .window()
            .set_cursor_hittest(!enabled)
            .map_err(|e| EngineError::Surface(e.to_string()))
    }
}
