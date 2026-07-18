use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::auto::{Breath, EyeBlink, LipSync, MouseTracker};
use crate::expression::ExpressionManager;
use crate::motion::MotionPlayer;
use crate::runtime::ModelRuntime;
use crate::render::wgpu::{
    WgpuClippingPlan, WgpuClippingResources, WgpuLive2dRenderer, WgpuMaskRenderTarget,
    WgpuMeshBuffers, WgpuTexture, WgpuTransform,
};

use super::MODEL_VIEW_FILL;

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

/// Returns true if the model is actively animating.
pub(super) fn is_animating(model: &LoadedModel) -> bool {
    model.animation.motion_player.is_some()
        || model.animation.expression_manager.active_expression_count() > 0
        || model.runtime.physics().is_some()
        || model.animation.eye_blink.is_some()
        || model.animation.breath.is_some()
}

/// Advances one model's animation state by `delta` seconds.
/// Returns true if the model state changed (needs GPU update).
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
) -> Result<(), super::EngineError> {
    let update = match model.mesh.mesh_buffers.update_drawables(queue, model.runtime.meshes()) {
        Ok(update) => update,
        Err(_) => {
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
