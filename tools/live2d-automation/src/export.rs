use crate::types::*;
use anyhow::{Context, Result};
use image::ImageEncoder;
use mocari::ai::{RiggedMesh, RiggedModel, RiggedParameter};
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
    let mut texture_pngs = Vec::with_capacity(result.layers.len());
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
        texture_pngs.push(buf);
    }

    // Export motions
    let mut motions_data = Vec::with_capacity(result.motions.len());
    for motion in &result.motions {
        let json = build_motion_json(motion)?;
        let path = motions_dir.join(format!("{}.motion3.json", motion.name));
        fs::write(&path, &json)?;
        motions_data.push((motion.name.clone(), json.into_bytes()));
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

    // Build RiggedModel for moc3 encoding (future: real moc3 binary)
    let meshes: Vec<RiggedMesh> = result
        .meshes
        .iter()
        .enumerate()
        .map(|(i, m)| RiggedMesh {
            texture_index: i.min(result.layers.len().saturating_sub(1)),
            vertices: m.vertices.clone(),
            uvs: m.uvs.clone(),
            indices: m.indices.clone(),
            opacity: 1.0,
        })
        .collect();

    let parameters: Vec<RiggedParameter> = result
        .rigging
        .parameters
        .iter()
        .map(|p| RiggedParameter {
            id: p.id.clone(),
            min: p.min,
            max: p.max,
            default: p.default,
            keyframes: Vec::new(),
        })
        .collect();

    // Suppress unused variable warning — RiggedModel is built for future moc3 encoding
    let _rigged_model = RiggedModel {
        textures: texture_pngs,
        meshes,
        parameters,
        deformers: Vec::new(),
        physics: None,
        motions: motions_data,
        expressions: Vec::new(),
    };

    write_mock_moc3(output_dir, model_name)?;

    println!("Exported model bundle to {}", output_dir.display());
    println!("  - {model_name}.model3.json");
    println!("  - {model_name}.moc3 (mock)");
    if result.physics.is_some() {
        println!("  - {model_name}.physics3.json");
    }
    println!("  - textures/ ({} files)", result.layers.len());
    println!("  - motions/ ({} files)", result.motions.len());

    Ok(())
}

fn build_model3_json(model_name: &str, result: &PipelineResult) -> String {
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

    // Build top-level map with Version first
    let mut root = Map::new();
    root.insert("Version".into(), serde_json::json!(3));
    root.insert("FileReferences".into(), serde_json::Value::Object(file_refs));
    root.insert("Groups".into(), serde_json::json!(groups));
    root.insert("HitAreas".into(), serde_json::json!(hit_areas));

    serde_json::to_string_pretty(&serde_json::Value::Object(root)).unwrap_or_default()
}

fn build_motion_json(motion: &Motion) -> Result<String> {
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
