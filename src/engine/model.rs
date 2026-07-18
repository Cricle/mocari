use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::auto::{Breath, EyeBlink, LipSync, MouseTracker};
use crate::expression::ExpressionManager;
use crate::motion::MotionPlayer;
use crate::runtime::ModelRuntime;
use crate::render::wgpu::{
    WgpuClippingResources, WgpuMaskRenderTarget, WgpuMeshBuffers, WgpuTexture, WgpuTransform,
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
