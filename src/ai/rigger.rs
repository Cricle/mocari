use super::RigError;

/// Complete model data produced by an AI rigger.
pub struct RiggedModel {
    pub textures: Vec<Vec<u8>>,
    pub meshes: Vec<RiggedMesh>,
    pub parameters: Vec<RiggedParameter>,
    pub deformers: Vec<RiggedDeformer>,
    pub physics: Option<Vec<u8>>,
    pub motions: Vec<(String, Vec<u8>)>,
    pub expressions: Vec<(String, Vec<u8>)>,
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

/// Trait for AI-assisted model rigging.
///
/// Implementors provide the AI backend (local model, cloud API, etc.).
/// mocari ships no implementations.
pub trait AiRigger: Send + Sync {
    fn rig_from_image(&self, image: &[u8]) -> Result<RiggedModel, RigError>;
    fn rig_from_psd(&self, psd: &[u8]) -> Result<RiggedModel, RigError>;
    fn rig_from_description(&self, prompt: &str) -> Result<RiggedModel, RigError>;
}
