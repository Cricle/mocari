use crate::types::*;
use anyhow::{Context, Result};
use image::ImageEncoder;
use std::fs;
use std::path::Path;

/// Export the complete model bundle to the output directory.
pub fn export_bundle(result: &PipelineResult, model_name: &str, output_dir: &Path) -> Result<()> {
    fs::create_dir_all(output_dir)?;
    let textures_dir = output_dir.join("textures");
    fs::create_dir_all(&textures_dir)?;
    let motions_dir = output_dir.join("motions");
    fs::create_dir_all(&motions_dir)?;

    // Export textures
    for (i, layer) in result.layers.iter().enumerate() {
        let path = textures_dir.join(format!("texture_{i}.png"));
        let mut buf = Vec::new();
        image::codecs::png::PngEncoder::new(&mut buf)
            .write_image(
                layer.image.as_raw(),
                layer.image.width(),
                layer.image.height(),
                image::ExtendedColorType::Rgba8,
            )
            .context("encoding texture PNG")?;
        fs::write(&path, &buf)?;
    }

    // Export motions
    for motion in &result.motions {
        let json = build_motion_json(motion)?;
        let path = motions_dir.join(format!("{}.motion3.json", motion.name));
        fs::write(&path, &json)?;
    }

    // Build model3.json manually (not via mocari's generate_model_json, which
    // doesn't support physics references or per-type motion grouping).
    let model_json = build_model3_json(model_name, result);
    fs::write(
        output_dir.join(format!("{model_name}.model3.json")),
        &model_json,
    )?;

    // Export physics
    if let Some(ref physics) = result.physics {
        fs::write(
            output_dir.join(format!("{model_name}.physics3.json")),
            serde_json::to_string_pretty(physics)?,
        )?;
    }

    // Export sidecar JSON files (pose3.json, cdi3.json, userdata3.json)
    let sidecars = crate::generators::build_sidecars(result)?;
    if let Some(pose3) = sidecars.pose3 {
        fs::write(output_dir.join(format!("{model_name}.pose3.json")), pose3)?;
    }
    fs::write(output_dir.join(format!("{model_name}.cdi3.json")), sidecars.cdi3)?;
    fs::write(
        output_dir.join(format!("{model_name}.userdata3.json")),
        sidecars.userdata3,
    )?;

    write_mock_moc3(output_dir, model_name)?;

    println!("Exported model bundle to {}", output_dir.display());
    println!("  - {model_name}.model3.json");
    println!("  - {model_name}.moc3 (mock)");
    if result.physics.is_some() {
        println!("  - {model_name}.physics3.json");
    }
    println!("  - {model_name}.pose3.json");
    println!("  - {model_name}.cdi3.json");
    println!("  - {model_name}.userdata3.json");
    println!("  - textures/ ({} files)", result.layers.len());
    println!("  - motions/ ({} files)", result.motions.len());

    Ok(())
}

pub fn build_model3_json(model_name: &str, result: &PipelineResult) -> String {
    use serde_json::Map;

    // FileReferences
    let mut file_refs = Map::new();
    file_refs.insert("Moc".into(), serde_json::json!(format!("{model_name}.moc3")));

    let textures: Vec<serde_json::Value> = result
        .layers
        .iter()
        .enumerate()
        .map(|(i, _)| serde_json::json!(format!("textures/texture_{i}.png")))
        .collect();
    file_refs.insert("Textures".into(), serde_json::json!(textures));

    if result.physics.is_some() {
        file_refs.insert("Physics".into(), serde_json::json!(format!("{model_name}.physics3.json")));
    }

    // Sidecar file references — pose, display info (cdi3), user data. These
    // are emitted by `generators::build_sidecars` below.
    file_refs.insert("Pose".into(), serde_json::json!(format!("{model_name}.pose3.json")));
    file_refs.insert("DisplayInfo".into(), serde_json::json!(format!("{model_name}.cdi3.json")));
    file_refs.insert("UserData".into(), serde_json::json!(format!("{model_name}.userdata3.json")));

    let mut motions_by_type: Map<String, serde_json::Value> = Map::new();
    for motion in &result.motions {
        let entry = serde_json::json!({ "File": format!("motions/{}.motion3.json", motion.name) });
        motions_by_type
            .entry(motion.motion_type.clone())
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .unwrap()
            .push(entry);
    }
    if !motions_by_type.is_empty() {
        file_refs.insert("Motions".into(), serde_json::Value::Object(motions_by_type));
    }

    // Groups
    let groups: Vec<serde_json::Value> = result
        .rigging
        .groups
        .iter()
        .map(|g| {
            serde_json::json!({
                "Target": "Parameter",
                "Name": g.name,
                "Ids": g.ids
            })
        })
        .collect();

    // HitAreas
    let hit_areas: Vec<serde_json::Value> = result
        .rigging
        .hit_areas
        .iter()
        .map(|h| serde_json::json!({ "Id": h.id, "Name": h.name }))
        .collect();

    // Deformers
    let deformers: Vec<serde_json::Value> = result
        .rigging
        .bones
        .iter()
        .enumerate()
        .flat_map(|(i, _)| {
            let center = calculate_mesh_center_from_layers(result, i);
            let mesh = result.meshes.get(i);

            // Create bezier control points if mesh is available
            let bezier_points = if let Some(m) = mesh {
                create_bezier_points_for_mesh(m)
            } else {
                create_default_bezier_points(center)
            };

            let bezier_json: Vec<serde_json::Value> = bezier_points
                .iter()
                .map(|p| serde_json::json!({ "X": p[0], "Y": p[1] }))
                .collect();

            vec![
                serde_json::json!({
                    "Id": format!("mesh_{i}_warp"),
                    "Name": format!("Mesh {i} Warp"),
                    "Type": "Warp",
                    "Origin": { "X": center[0], "Y": center[1] }
                }),
                serde_json::json!({
                    "Id": format!("mesh_{i}_rotation"),
                    "Name": format!("Mesh {i} Rotation"),
                    "Type": "Rotation",
                    "Origin": { "X": center[0], "Y": center[1] }
                }),
                serde_json::json!({
                    "Id": format!("mesh_{i}_bezier"),
                    "Name": format!("Mesh {i} Bezier"),
                    "Type": "Bezier",
                    "Origin": { "X": center[0], "Y": center[1] },
                    "ControlPoints": bezier_json
                }),
            ]
        })
        .collect();

    // BoneWeights
    let bone_weights: Vec<serde_json::Value> = create_bone_weights_for_export(result)
        .iter()
        .map(|w| {
            serde_json::json!({
                "Mesh": w.mesh_name,
                "Bone": w.bone_id,
                "Weight": w.weight
            })
        })
        .collect();

    // Build top-level map with Version first
    let mut root = Map::new();
    root.insert("Version".into(), serde_json::json!(3));
    root.insert("FileReferences".into(), serde_json::Value::Object(file_refs));
    root.insert("Groups".into(), serde_json::json!(groups));
    root.insert("HitAreas".into(), serde_json::json!(hit_areas));
    root.insert("Deformers".into(), serde_json::json!(deformers));
    root.insert("BoneWeights".into(), serde_json::json!(bone_weights));

    serde_json::to_string_pretty(&serde_json::Value::Object(root)).unwrap_or_default()
}

/// Create Bezier control points for a mesh.
fn create_bezier_points_for_mesh(mesh: &crate::types::ArtMesh) -> Vec<[f32; 2]> {
    if mesh.vertices.is_empty() {
        return create_default_bezier_points([0.0, 0.0]);
    }

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

/// Create default Bezier control points around a center point.
fn create_default_bezier_points(center: [f32; 2]) -> Vec<[f32; 2]> {
    let size = 50.0;
    let mut points = Vec::with_capacity(16);
    for row in 0..4 {
        for col in 0..4 {
            let x = center[0] - size + (2.0 * size) * (col as f32 / 3.0);
            let y = center[1] - size + (2.0 * size) * (row as f32 / 3.0);
            points.push([x, y]);
        }
    }
    points
}

fn calculate_mesh_center_from_layers(result: &PipelineResult, index: usize) -> [f32; 2] {
    if let Some(layer) = result.layers.get(index) {
        [
            layer.bounds.x as f32 + layer.bounds.width as f32 / 2.0,
            layer.bounds.y as f32 + layer.bounds.height as f32 / 2.0,
        ]
    } else {
        [0.0, 0.0]
    }
}

fn create_bone_weights_for_export(result: &PipelineResult) -> Vec<BoneWeight> {
    let mut weights = Vec::new();

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

    let layer_names: Vec<&str> = result.layers.iter().map(|l| l.name.as_str()).collect();

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

pub fn build_motion_json(motion: &Motion) -> Result<String> {
    let curves: Vec<serde_json::Value> = motion
        .curves
        .iter()
        .map(|c| {
            serde_json::json!({
                "Target": c.target,
                "Id": c.id,
                "Segments": c.segments
            })
        })
        .collect();

    let data = serde_json::json!({
        "Version": 3,
        "Meta": {
            "Duration": motion.duration,
            "Fps": motion.fps,
            "Loop": motion.is_loop
        },
        "Curves": curves
    });

    serde_json::to_string_pretty(&data).context("serializing motion JSON")
}

fn write_mock_moc3(output_dir: &Path, model_name: &str) -> Result<()> {
    let path = output_dir.join(format!("{model_name}.moc3"));
    let mut data = Vec::with_capacity(32);
    data.extend_from_slice(b"MOC3");
    data.extend_from_slice(&3u32.to_le_bytes());
    data.extend_from_slice(&32u32.to_le_bytes());
    data.extend_from_slice(&0u32.to_le_bytes());
    data.resize(32, 0);
    fs::write(path, data)?;
    Ok(())
}
