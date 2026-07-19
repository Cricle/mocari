use image::RgbaImage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct DetectedPart {
    pub name: String,
    pub bounds: BoundingBox,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct FaceDetection {
    pub has_face: bool,
    pub face_bounds: BoundingBox,
    pub parts: Vec<DetectedPart>,
}

#[derive(Debug, Clone)]
pub struct Layer {
    pub name: String,
    pub image: RgbaImage,
    pub bounds: BoundingBox,
    pub z_order: i32,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct Bone {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
    pub position: [f32; 2],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub id: String,
    pub name: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
}

#[derive(Debug, Clone)]
pub struct ArtMesh {
    pub vertices: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
}

#[derive(Debug, Clone)]
pub struct PhysicsParam {
    pub id: String,
    pub weight: f32,
    pub param_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Particle {
    pub index: usize,
    pub position: [f32; 2],
    pub mass: f32,
    pub damping: f32,
    pub stiffness: f32,
    pub fixed: bool,
}

#[derive(Debug, Clone)]
pub struct PhysicsGroup {
    pub name: String,
    pub input: Vec<PhysicsParam>,
    pub output: Vec<PhysicsParam>,
    pub particles: Vec<Particle>,
}

#[derive(Debug, Clone)]
pub struct Motion {
    pub name: String,
    pub motion_type: String,
    pub duration: f32,
    pub fps: f32,
    pub is_loop: bool,
    pub curves: Vec<MotionCurve>,
}

#[derive(Debug, Clone)]
pub struct MotionCurve {
    pub target: String,
    pub id: String,
    pub segments: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct RiggingResult {
    pub bones: Vec<Bone>,
    pub parameters: Vec<Parameter>,
    pub bone_weights: Vec<(String, String, f32)>, // (mesh_name, bone_id, weight)
    pub hit_areas: Vec<HitArea>,
    pub groups: Vec<ParameterGroup>,
}

#[derive(Debug, Clone)]
pub struct HitArea {
    pub id: String,
    pub name: String,
    pub bounds: BoundingBox,
}

#[derive(Debug, Clone)]
pub struct ParameterGroup {
    pub name: String,
    pub ids: Vec<String>,
}

pub struct PipelineResult {
    pub layers: Vec<Layer>,
    pub meshes: Vec<ArtMesh>,
    pub rigging: RiggingResult,
    pub motions: Vec<Motion>,
    pub physics: Option<serde_json::Value>,
}

const LAYER_ORDER: &[&str] = &[
    "back_hair",
    "body",
    "left_arm",
    "right_arm",
    "left_leg",
    "right_leg",
    "head",
    "face_base",
    "mouth",
    "nose",
    "left_eye",
    "right_eye",
    "left_eyebrow",
    "right_eyebrow",
    "front_hair",
    "left_hand",
    "right_hand",
    "accessories",
];

pub fn z_order_for_part(name: &str) -> i32 {
    LAYER_ORDER
        .iter()
        .position(|&n| n == name)
        .map(|p| p as i32)
        .unwrap_or(50)
}
