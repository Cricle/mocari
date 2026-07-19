use super::RigError;
use crate::json::Model3;
use crate::moc3::{
    Moc3ArtMeshInfo, Moc3ArtMeshes, Moc3ArtMeshKeyformInfo, Moc3ArtMeshKeyforms,
    Moc3CanvasInfo, Moc3Deformers, Moc3Glues, Moc3Ids, Moc3KeyformBindings,
    Moc3OffscreenInfo, Moc3Parts,
};
use crate::runtime::ModelRuntime;

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

impl RiggedModel {
    /// Converts this rigged model into a [`ModelRuntime`].
    ///
    /// The caller provides the parsed `model3.json` data. Textures are not
    /// included in the runtime — they should be decoded separately for the
    /// renderer.
    ///
    /// Returns `None` if the assembled components produce an invalid mesh set.
    pub fn into_runtime(self, model: Model3) -> Option<ModelRuntime> {
        let mesh_count = self.meshes.len();

        // Parameter IDs and ranges.
        let param_ids: Vec<String> = self.parameters.iter().map(|p| p.id.clone()).collect();
        let param_min: Vec<f32> = self.parameters.iter().map(|p| p.min).collect();
        let param_max: Vec<f32> = self.parameters.iter().map(|p| p.max).collect();
        let param_default: Vec<f32> = self.parameters.iter().map(|p| p.default).collect();

        // Art mesh IDs.
        let mesh_ids: Vec<String> = (0..mesh_count).map(|i| format!("Mesh{i}")).collect();

        // Build UV arrays and position indices from rigged mesh data.
        let mut all_uvs = Vec::new();
        let mut all_position_indices: Vec<i16> = Vec::new();

        for mesh in &self.meshes {
            for uv in &mesh.uvs {
                all_uvs.push(uv[0]);
                all_uvs.push(uv[1]);
            }
            // Position indices are local to each mesh (0-based).
            for i in 0..mesh.vertices.len() as i16 {
                all_position_indices.push(i);
            }
        }

        // Flatten all vertex positions into a single XY array.
        let mut all_positions = Vec::new();
        for mesh in &self.meshes {
            for v in &mesh.vertices {
                all_positions.push(v[0]);
                all_positions.push(v[1]);
            }
        }

        // Build art mesh infos and keyforms.
        let mut mesh_infos = Vec::with_capacity(mesh_count);
        let mut keyform_begin_indices = Vec::with_capacity(mesh_count);
        let mut keyform_counts_vec = Vec::with_capacity(mesh_count);
        let mut vertex_counts = Vec::with_capacity(mesh_count);
        let mut keyforms = Vec::with_capacity(mesh_count);
        let mut keyform_positions = Vec::new();
        let mut uv_offset: i32 = 0;
        let mut pos_idx_offset: i32 = 0;

        for (i, mesh) in self.meshes.iter().enumerate() {
            let vert_count = mesh.vertices.len() as i32;
            let pos_idx_count = vert_count; // 1:1 mapping

            mesh_infos.push(Moc3ArtMeshInfo::new(
                mesh.texture_index as i32,
                0, // no double-sided flag
                pos_idx_count,
                uv_offset,
                pos_idx_offset,
                vert_count,
                0, // no masks
                0,
            ));

            // One keyform per mesh with the mesh vertices as positions.
            keyform_begin_indices.push(i as i32);
            keyform_counts_vec.push(1);
            vertex_counts.push(vert_count);

            let pos_begin = (keyform_positions.len() / 2) as i32;
            keyforms.push(Moc3ArtMeshKeyformInfo::new(mesh.opacity, 0.0, pos_begin));
            for v in &mesh.vertices {
                keyform_positions.push(v[0]);
                keyform_positions.push(v[1]);
            }

            uv_offset += vert_count * 2;
            pos_idx_offset += vert_count;
        }

        // Flatten indices. Each mesh's indices are offset by the cumulative
        // vertex count of preceding meshes.
        let mut all_indices = Vec::new();
        let mut vertex_base: u16 = 0;
        for mesh in &self.meshes {
            for &idx in &mesh.indices {
                all_indices.push(idx + vertex_base);
            }
            vertex_base += mesh.vertices.len() as u16;
        }

        // Build moc3 types.
        let art_meshes = Moc3ArtMeshes::from_parts(
            mesh_infos,
            all_uvs,
            all_position_indices,
            Vec::new(), // no masks
        )
        .ok()?;

        let art_mesh_keyforms = Moc3ArtMeshKeyforms::from_parts(
            keyform_begin_indices,
            keyform_counts_vec,
            vertex_counts,
            keyforms,
            keyform_positions,
        )
        .ok()?;

        let bindings = Moc3KeyformBindings::from_parts(param_min, param_max, param_default);

        let ids = Moc3Ids::from_parts(Vec::new(), mesh_ids, param_ids);

        let deformers = Moc3Deformers::empty();

        let canvas = Moc3CanvasInfo::from_parts(1.0, 0.0, 0.0, 2.0, 2.0);

        let parts = Moc3Parts::from_parts(
            vec![-1],      // no parent
            vec![0],       // band index 0
            vec![0],       // keyform begin
            vec![1],       // 1 keyform
            vec![0.0],     // draw order
            vec![1.0],     // opacity
        );

        let offscreen = Moc3OffscreenInfo::from_parts(
            vec![-1],              // part parent indices
            vec![-1; mesh_count],  // drawable parent part indices
            vec![-1],              // part offscreen indices
            Vec::new(),            // offscreen owner part indices
        );

        let glues = Moc3Glues::from_parts(
            Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(),
            Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new(),
        )
        .ok()?;

        ModelRuntime::new(
            model,
            canvas,
            art_meshes,
            art_mesh_keyforms,
            deformers,
            bindings,
            ids,
            offscreen,
            glues,
            parts,
            None, // no draw order groups
            None, // no pose
        )
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::model_json::{ModelJsonConfig, generate_model_json};

    fn make_triangle_model() -> RiggedModel {
        RiggedModel {
            textures: vec![vec![0u8; 8]], // dummy PNG
            meshes: vec![RiggedMesh {
                texture_index: 0,
                vertices: vec![[-1.0, -1.0], [1.0, -1.0], [0.0, 1.0]],
                uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0]],
                indices: vec![0, 1, 2],
                opacity: 1.0,
            }],
            parameters: vec![RiggedParameter {
                id: "ParamAngleX".into(),
                min: -30.0,
                max: 30.0,
                default: 0.0,
                keyframes: vec![],
            }],
            deformers: vec![],
            physics: None,
            motions: vec![],
            expressions: vec![],
        }
    }

    #[test]
    fn into_runtime_produces_valid_runtime() {
        let rigged = make_triangle_model();
        let json = generate_model_json(&rigged, &ModelJsonConfig::default());
        let model3 = Model3::from_json_str(&json).unwrap();
        let runtime = rigged.into_runtime(model3).expect("into_runtime failed");

        // Should have 1 drawable (the triangle mesh).
        assert_eq!(runtime.meshes().len(), 1);
        // Should have 1 parameter.
        assert_eq!(runtime.parameter_ids().len(), 1);
        assert_eq!(runtime.parameter_ids()[0], "ParamAngleX");
    }

    #[test]
    fn into_runtime_mesh_has_correct_vertex_count() {
        let rigged = make_triangle_model();
        let json = generate_model_json(&rigged, &ModelJsonConfig::default());
        let model3 = Model3::from_json_str(&json).unwrap();
        let runtime = rigged.into_runtime(model3).unwrap();
        let mesh = &runtime.meshes()[0];
        assert_eq!(mesh.vertices().len(), 3);
    }

    #[test]
    fn into_runtime_with_multiple_meshes() {
        let rigged = RiggedModel {
            textures: vec![vec![0u8; 8]],
            meshes: vec![
                RiggedMesh {
                    texture_index: 0,
                    vertices: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                    uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                    indices: vec![0, 1, 2],
                    opacity: 1.0,
                },
                RiggedMesh {
                    texture_index: 0,
                    vertices: vec![[2.0, 0.0], [3.0, 0.0], [2.0, 1.0]],
                    uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                    indices: vec![0, 1, 2],
                    opacity: 0.8,
                },
            ],
            parameters: vec![RiggedParameter {
                id: "ParamAngleX".into(),
                min: -30.0,
                max: 30.0,
                default: 0.0,
                keyframes: vec![],
            }],
            deformers: vec![],
            physics: None,
            motions: vec![],
            expressions: vec![],
        };
        let json = generate_model_json(&rigged, &ModelJsonConfig::default());
        let model3 = Model3::from_json_str(&json).unwrap();
        let runtime = rigged.into_runtime(model3);
        assert!(runtime.is_some(), "into_runtime returned None for multi-mesh model");
        let runtime = runtime.unwrap();
        assert_eq!(runtime.meshes().len(), 2);
    }
}
