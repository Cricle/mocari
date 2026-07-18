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
    #[allow(dead_code)]
    pub lip_sync: Option<LipSync>,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
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
) -> crate::core::Mat4 {
    let aspect = surface_width as f32 / surface_height as f32;
    let view_fill = MODEL_VIEW_FILL * scale.clamp(0.5, 2.0);
    let fit_x = view_fill / (bounds.width() * aspect);
    let fit_y = view_fill / bounds.height();
    let scale_y = fit_x.min(fit_y);
    let scale_x = scale_y / aspect;

    let mut matrix = crate::core::Mat4::IDENTITY;
    matrix.x_axis.x = scale_x;
    matrix.y_axis.y = scale_y;
    matrix.w_axis.x = -bounds.center_x() * scale_x;
    matrix.w_axis.y = -bounds.center_y() * scale_y;
    matrix
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::Mat4Ext;

    #[test]
    fn model_bounds_from_empty_drawables_returns_none() {
        assert!(ModelBounds::from_drawables(&[]).is_none());
    }

    #[test]
    fn model_bounds_computes_correct_extents() {
        use crate::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};

        let mesh = Moc3DrawableMesh::from_parts(
            0,
            0,
            1.0,
            0.0,
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

        assert!((wide.scale_x()).abs() < (wide.scale_y()).abs());
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

        assert!((large.scale_x()).abs() > (normal.scale_x()).abs());
        assert!((large.scale_y()).abs() > (normal.scale_y()).abs());
    }
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

/// Returns true if the model has any running animation (including auto-systems).
pub(super) fn is_animating(model: &LoadedModel) -> bool {
    model.animation.motion_player.is_some()
        || model.animation.expression_manager.active_expression_count() > 0
        || model.runtime.physics().is_some()
        || model.animation.eye_blink.is_some()
        || model.animation.breath.is_some()
        || model.animation.lip_sync.as_ref().is_some_and(|l| l.is_active())
        || model.animation.mouse_tracker.as_ref().is_some_and(|m| m.is_active())
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
    if let Some(lip_sync) = model.animation.lip_sync.as_mut() {
        lip_sync.tick(delta);
        lip_sync.apply(&mut model.runtime);
    }
    if let Some(mouse_tracker) = model.animation.mouse_tracker.as_mut() {
        mouse_tracker.tick(delta);
        mouse_tracker.apply(&mut model.runtime);
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
        Ok(update) => {
            // Clear dirty flags after successful GPU upload
            for mesh in model.runtime.meshes_mut() {
                mesh.clear_dirty();
            }
            update
        }
        Err(_) => {
            let mesh_buffers = WgpuMeshBuffers::from_drawables(device, model.runtime.meshes())
                .ok_or(crate::render::wgpu::WgpuRenderError::MissingDrawable { drawable_index: 0 })?;
            model.mesh.mesh_buffers = mesh_buffers;
            rebuild_clipping(renderer, device, model)?;
            for mesh in model.runtime.meshes_mut() {
                mesh.clear_dirty();
            }
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
