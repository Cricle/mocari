use crate::types::*;

/// Standard Live2D parameters used by the auto-rigger.
fn standard_parameters() -> Vec<Parameter> {
    vec![
        Parameter { id: "ParamBodyAngleX".into(), name: "Body Angle X".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamBodyAngleY".into(), name: "Body Angle Y".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamBodyAngleZ".into(), name: "Body Angle Z".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamBreath".into(), name: "Breath".into(), min: 0.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamAngleX".into(), name: "Head Angle X".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamAngleY".into(), name: "Head Angle Y".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamAngleZ".into(), name: "Head Angle Z".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamEyeLOpen".into(), name: "Left Eye Open".into(), min: 0.0, max: 1.0, default: 1.0 },
        Parameter { id: "ParamEyeROpen".into(), name: "Right Eye Open".into(), min: 0.0, max: 1.0, default: 1.0 },
        Parameter { id: "ParamEyeBallX".into(), name: "Eye Ball X".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamEyeBallY".into(), name: "Eye Ball Y".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamEyeLSmile".into(), name: "Left Eye Smile".into(), min: 0.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamEyeRSmile".into(), name: "Right Eye Smile".into(), min: 0.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamBrowLY".into(), name: "Left Brow Y".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamBrowRY".into(), name: "Right Brow Y".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamBrowLAngle".into(), name: "Left Brow Angle".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamBrowRAngle".into(), name: "Right Brow Angle".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamBrowLForm".into(), name: "Left Brow Form".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamBrowRForm".into(), name: "Right Brow Form".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamMouthOpenY".into(), name: "Mouth Open".into(), min: 0.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamMouthForm".into(), name: "Mouth Form".into(), min: -1.0, max: 1.0, default: 0.0 },
        Parameter { id: "ParamArmLA".into(), name: "Left Arm Angle".into(), min: -120.0, max: 120.0, default: 0.0 },
        Parameter { id: "ParamArmRA".into(), name: "Right Arm Angle".into(), min: -120.0, max: 120.0, default: 0.0 },
        Parameter { id: "ParamLegL".into(), name: "Left Leg".into(), min: -90.0, max: 90.0, default: 0.0 },
        Parameter { id: "ParamLegR".into(), name: "Right Leg".into(), min: -90.0, max: 90.0, default: 0.0 },
        Parameter { id: "ParamHairFront".into(), name: "Hair Front".into(), min: -45.0, max: 45.0, default: 0.0 },
        Parameter { id: "ParamHairSide".into(), name: "Hair Side".into(), min: -45.0, max: 45.0, default: 0.0 },
        Parameter { id: "ParamHairBack".into(), name: "Hair Back".into(), min: -45.0, max: 45.0, default: 0.0 },
        Parameter { id: "ParamClothA".into(), name: "Cloth A".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamClothB".into(), name: "Cloth B".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamClothC".into(), name: "Cloth C".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamAccessoryA".into(), name: "Accessory A".into(), min: -30.0, max: 30.0, default: 0.0 },
        Parameter { id: "ParamAccessoryB".into(), name: "Accessory B".into(), min: -30.0, max: 30.0, default: 0.0 },
    ]
}

fn make_bone(id: &str, name: &str, parent: Option<&str>, x: f32, y: f32) -> Bone {
    Bone {
        id: id.into(),
        name: name.into(),
        parent: parent.map(|s| s.into()),
        position: [x, y],
    }
}

/// Set up rigging from layers: bones, parameters, deformers, bindings.
pub fn setup_rigging(layers: &[Layer]) -> RiggingResult {
    let mut bones = vec![make_bone("root", "Root", None, 0.0, 0.0)];

    let head_center = layers
        .iter()
        .find(|l| l.name == "head")
        .map(|l| [l.bounds.x as f32 + l.bounds.width as f32 / 2.0, l.bounds.y as f32 + l.bounds.height as f32 / 2.0])
        .unwrap_or([0.0, -50.0]);

    bones.push(make_bone("head", "Head", Some("root"), head_center[0], head_center[1]));
    bones.push(make_bone("neck", "Neck", Some("head"), 0.0, -50.0));

    let body_center = layers
        .iter()
        .find(|l| l.name == "body")
        .map(|l| [l.bounds.x as f32 + l.bounds.width as f32 / 2.0, l.bounds.y as f32 + l.bounds.height as f32 / 2.0])
        .unwrap_or([0.0, 50.0]);

    bones.push(make_bone("torso", "Torso", Some("root"), body_center[0], body_center[1]));
    bones.push(make_bone("left_arm", "Left Arm", Some("torso"), -80.0, -100.0));
    bones.push(make_bone("left_forearm", "Left Forearm", Some("left_arm"), -60.0, 80.0));
    bones.push(make_bone("right_arm", "Right Arm", Some("torso"), 80.0, -100.0));
    bones.push(make_bone("right_forearm", "Right Forearm", Some("right_arm"), 60.0, 80.0));
    bones.push(make_bone("left_leg", "Left Leg", Some("root"), -40.0, 150.0));
    bones.push(make_bone("left_shin", "Left Shin", Some("left_leg"), 0.0, 100.0));
    bones.push(make_bone("right_leg", "Right Leg", Some("root"), 40.0, 150.0));
    bones.push(make_bone("right_shin", "Right Shin", Some("right_leg"), 0.0, 100.0));
    bones.push(make_bone("left_eye", "Left Eye", Some("head"), -30.0, -20.0));
    bones.push(make_bone("right_eye", "Right Eye", Some("head"), 30.0, -20.0));
    bones.push(make_bone("mouth", "Mouth", Some("head"), 0.0, 20.0));

    let parameters = standard_parameters();

    let hit_areas = vec![
        HitArea { id: "HitHead".into(), name: "Head".into() },
        HitArea { id: "HitBody".into(), name: "Body".into() },
        HitArea { id: "HitLeftHand".into(), name: "Left Hand".into() },
        HitArea { id: "HitRightHand".into(), name: "Right Hand".into() },
    ];

    let groups = vec![
        ParameterGroup { name: "Head".into(), ids: filter_params(&parameters, "Angle", Some("Body")) },
        ParameterGroup { name: "Body".into(), ids: filter_params(&parameters, "Body", None) },
        ParameterGroup { name: "Eyes".into(), ids: filter_params(&parameters, "Eye", None) },
        ParameterGroup { name: "Brows".into(), ids: filter_params(&parameters, "Brow", None) },
        ParameterGroup { name: "Mouth".into(), ids: filter_params(&parameters, "Mouth", None) },
        ParameterGroup { name: "Arms".into(), ids: filter_params(&parameters, "Arm", None) },
        ParameterGroup { name: "Legs".into(), ids: filter_params(&parameters, "Leg", None) },
        ParameterGroup { name: "Hair".into(), ids: filter_params(&parameters, "Hair", None) },
        ParameterGroup { name: "Clothing".into(), ids: filter_params(&parameters, "Cloth", None) },
        ParameterGroup { name: "Accessories".into(), ids: filter_params(&parameters, "Accessory", None) },
    ];

    RiggingResult { bones, parameters, hit_areas, groups }
}

/// Create deformers for each mesh.
pub fn create_deformers(meshes: &[ArtMesh]) -> Vec<Deformer> {
    let mut deformers = Vec::new();

    for (i, mesh) in meshes.iter().enumerate() {
        // Calculate mesh center
        let center = calculate_mesh_center(mesh);

        // Create warp deformer
        deformers.push(Deformer {
            id: format!("mesh_{i}_warp"),
            name: format!("Mesh {i} Warp"),
            deformer_type: DeformerType::Warp,
            origin: center,
            influence: 1.0,
            control_points: vec![
                [center[0] - 50.0, center[1] - 50.0],
                [center[0] + 50.0, center[1] - 50.0],
                [center[0] - 50.0, center[1] + 50.0],
                [center[0] + 50.0, center[1] + 50.0],
            ],
        });

        // Create rotation deformer
        deformers.push(Deformer {
            id: format!("mesh_{i}_rotation"),
            name: format!("Mesh {i} Rotation"),
            deformer_type: DeformerType::Rotation,
            origin: center,
            influence: 1.0,
            control_points: vec![center],
        });

        // Create bezier deformer for complex deformations
        deformers.push(Deformer {
            id: format!("mesh_{i}_bezier"),
            name: format!("Mesh {i} Bezier"),
            deformer_type: DeformerType::Bezier,
            origin: center,
            influence: 1.0,
            control_points: create_bezier_control_points(center, mesh),
        });
    }

    deformers
}

/// Create Bezier control points for a mesh.
fn create_bezier_control_points(_center: [f32; 2], mesh: &ArtMesh) -> Vec<[f32; 2]> {
    // Calculate mesh bounds
    let (min_x, max_x, min_y, max_y) = mesh.vertices.iter().fold(
        (f32::MAX, f32::MIN, f32::MAX, f32::MIN),
        |(min_x, max_x, min_y, max_y), v| {
            (min_x.min(v[0]), max_x.max(v[0]), min_y.min(v[1]), max_y.max(v[1]))
        },
    );

    let width = max_x - min_x;
    let height = max_y - min_y;

    // Create 4x4 Bezier control point grid
    let mut points = Vec::with_capacity(16);
    for row in 0..4 {
        for col in 0..4 {
            let x = min_x + width * (col as f32 / 3.0);
            let y = min_y + height * (row as f32 / 3.0);
            points.push([x, y]);
        }
    }

    points
}

/// Calculate the center of a mesh.
fn calculate_mesh_center(mesh: &ArtMesh) -> [f32; 2] {
    if mesh.vertices.is_empty() {
        return [0.0, 0.0];
    }

    let mut sum_x = 0.0;
    let mut sum_y = 0.0;

    for vertex in &mesh.vertices {
        sum_x += vertex[0];
        sum_y += vertex[1];
    }

    let count = mesh.vertices.len() as f32;
    [sum_x / count, sum_y / count]
}

/// Create bone-to-mesh weights.
pub fn create_bone_weights(layers: &[Layer]) -> Vec<BoneWeight> {
    let mut weights = Vec::new();

    // Define bone-mesh mapping
    let mapping: &[(&str, &[&str])] = &[
        ("head", &["head", "face_base", "back_hair", "front_hair"]),
        ("left_eye", &["left_eye"]),
        ("right_eye", &["right_eye"]),
        ("mouth", &["mouth"]),
        ("torso", &["body"]),
        ("left_arm", &["left_arm"]),
        ("right_arm", &["right_arm"]),
        ("left_leg", &["left_leg"]),
        ("right_leg", &["right_leg"]),
    ];

    let layer_names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();

    for (bone_id, mesh_names) in mapping {
        for mesh_name in *mesh_names {
            if layer_names.contains(mesh_name) {
                weights.push(BoneWeight {
                    mesh_name: mesh_name.to_string(),
                    bone_id: bone_id.to_string(),
                    weight: 1.0,
                });
            }
        }
    }

    weights
}

fn filter_params(params: &[Parameter], includes: &str, excludes: Option<&str>) -> Vec<String> {
    params
        .iter()
        .filter(|p| p.id.contains(includes) && excludes.is_none_or(|ex| !p.id.contains(ex)))
        .map(|p| p.id.clone())
        .collect()
}
