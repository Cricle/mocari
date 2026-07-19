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

fn bone(id: &str, name: &str, parent: Option<&str>, x: f32, y: f32) -> Bone {
    Bone {
        id: id.into(),
        name: name.into(),
        parent: parent.map(|s| s.into()),
        position: [x, y],
    }
}

/// Set up rigging from layers: bones, parameters, deformers, bindings.
pub fn setup_rigging(layers: &[Layer]) -> RiggingResult {
    let mut bones = vec![bone("root", "Root", None, 0.0, 0.0)];

    // Find head center from the "head" layer if present
    let head_center = layers
        .iter()
        .find(|l| l.name == "head")
        .map(|l| {
            [
                l.bounds.x as f32 + l.bounds.width as f32 / 2.0,
                l.bounds.y as f32 + l.bounds.height as f32 / 2.0,
            ]
        })
        .unwrap_or([0.0, -50.0]);

    bones.push(bone("head", "Head", Some("root"), head_center[0], head_center[1]));
    bones.push(bone("neck", "Neck", Some("head"), 0.0, -50.0));

    let body_center = layers
        .iter()
        .find(|l| l.name == "body")
        .map(|l| {
            [
                l.bounds.x as f32 + l.bounds.width as f32 / 2.0,
                l.bounds.y as f32 + l.bounds.height as f32 / 2.0,
            ]
        })
        .unwrap_or([0.0, 50.0]);

    bones.push(bone("torso", "Torso", Some("root"), body_center[0], body_center[1]));
    bones.push(bone("left_arm", "Left Arm", Some("torso"), -80.0, -100.0));
    bones.push(bone("left_forearm", "Left Forearm", Some("left_arm"), -60.0, 80.0));
    bones.push(bone("right_arm", "Right Arm", Some("torso"), 80.0, -100.0));
    bones.push(bone("right_forearm", "Right Forearm", Some("right_arm"), 60.0, 80.0));
    bones.push(bone("left_leg", "Left Leg", Some("root"), -40.0, 150.0));
    bones.push(bone("left_shin", "Left Shin", Some("left_leg"), 0.0, 100.0));
    bones.push(bone("right_leg", "Right Leg", Some("root"), 40.0, 150.0));
    bones.push(bone("right_shin", "Right Shin", Some("right_leg"), 0.0, 100.0));
    bones.push(bone("left_eye", "Left Eye", Some("head"), -30.0, -20.0));
    bones.push(bone("right_eye", "Right Eye", Some("head"), 30.0, -20.0));
    bones.push(bone("mouth", "Mouth", Some("head"), 0.0, 20.0));

    let parameters = standard_parameters();

    // Bone-to-mesh weights
    let bone_mesh_mapping: &[(&str, &[&str])] = &[
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
    let mut bone_weights = Vec::new();
    for (bone_id, mesh_names) in bone_mesh_mapping {
        for mesh_name in *mesh_names {
            if layer_names.contains(mesh_name) {
                bone_weights.push((mesh_name.to_string(), bone_id.to_string(), 1.0));
            }
        }
    }

    // Hit areas
    let hit_areas = vec![
        HitArea {
            id: "HitHead".into(),
            name: "Head".into(),
            bounds: layers
                .iter()
                .find(|l| l.name == "head")
                .map(|l| l.bounds.clone())
                .unwrap_or(BoundingBox { x: -100, y: -200, width: 200, height: 200 }),
        },
        HitArea {
            id: "HitBody".into(),
            name: "Body".into(),
            bounds: layers
                .iter()
                .find(|l| l.name == "body")
                .map(|l| l.bounds.clone())
                .unwrap_or(BoundingBox { x: -100, y: 0, width: 200, height: 300 }),
        },
    ];

    // Parameter groups
    let groups = vec![
        ParameterGroup { name: "Head".into(), ids: param_ids_containing(&parameters, "Angle", Some("Body")) },
        ParameterGroup { name: "Body".into(), ids: param_ids_containing(&parameters, "Body", None) },
        ParameterGroup { name: "Eyes".into(), ids: param_ids_containing(&parameters, "Eye", None) },
        ParameterGroup { name: "Brows".into(), ids: param_ids_containing(&parameters, "Brow", None) },
        ParameterGroup { name: "Mouth".into(), ids: param_ids_containing(&parameters, "Mouth", None) },
        ParameterGroup { name: "Arms".into(), ids: param_ids_containing(&parameters, "Arm", None) },
        ParameterGroup { name: "Legs".into(), ids: param_ids_containing(&parameters, "Leg", None) },
        ParameterGroup { name: "Hair".into(), ids: param_ids_containing(&parameters, "Hair", None) },
        ParameterGroup { name: "Clothing".into(), ids: param_ids_containing(&parameters, "Cloth", None) },
        ParameterGroup { name: "Accessories".into(), ids: param_ids_containing(&parameters, "Accessory", None) },
    ];

    RiggingResult {
        bones,
        parameters,
        bone_weights,
        hit_areas,
        groups,
    }
}

fn param_ids_containing(params: &[Parameter], includes: &str, excludes: Option<&str>) -> Vec<String> {
    params
        .iter()
        .filter(|p| {
            p.id.contains(includes)
                && excludes
                    .map(|ex| !p.id.contains(ex))
                    .unwrap_or(true)
        })
        .map(|p| p.id.clone())
        .collect()
}
