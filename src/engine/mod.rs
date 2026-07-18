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
use model::{AnimationState, LoadedModel, MeshState};

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
        self.models.iter().any(|m| model::is_animating(m))
    }
}
