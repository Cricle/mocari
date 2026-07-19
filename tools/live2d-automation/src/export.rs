use crate::types::*;
use anyhow::{Context, Result};
use image::ImageEncoder;
use mocari::ai::{generate_model_json, ModelJsonConfig, RiggedMesh, RiggedModel, RiggedParameter};
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
    let mut texture_pngs: Vec<Vec<u8>> = Vec::new();
    for (i, layer) in result.layers.iter().enumerate() {
        let path = textures_dir.join(format!("texture_{i}.png"));
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        encoder
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

    // Build RiggedModel from pipeline results
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

    // Export motions as JSON bytes
    let mut motions_data: Vec<(String, Vec<u8>)> = Vec::new();
    for motion in &result.motions {
        let json = build_motion_json(motion);
        let path = motions_dir.join(format!("{}.motion3.json", motion.name));
        fs::write(&path, &json)?;
        motions_data.push((motion.name.clone(), json.into_bytes()));
    }

    let rigged_model = RiggedModel {
        textures: texture_pngs,
        meshes,
        parameters,
        deformers: Vec::new(),
        physics: None,
        motions: motions_data,
        expressions: Vec::new(),
    };

    // Generate and write model3.json
    let config = ModelJsonConfig {
        moc_file: format!("{model_name}.moc3"),
        texture_dir: "textures".into(),
        motion_dir: "motions".into(),
        expression_dir: String::new(),
    };
    let model_json = generate_model_json(&rigged_model, &config);

    // Parse and enrich with Groups and HitAreas
    let mut model_json_value: serde_json::Value =
        serde_json::from_str(&model_json).context("parsing generated model3.json")?;

    // Add Groups
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
    model_json_value["Groups"] = serde_json::json!(groups);

    // Add HitAreas
    let hit_areas: Vec<serde_json::Value> = result
        .rigging
        .hit_areas
        .iter()
        .map(|h| {
            serde_json::json!({
                "Id": h.id,
                "Name": h.name
            })
        })
        .collect();
    model_json_value["HitAreas"] = serde_json::json!(hit_areas);

    let model_json_str =
        serde_json::to_string_pretty(&model_json_value).context("serializing model3.json")?;
    fs::write(output_dir.join(format!("{model_name}.model3.json")), &model_json_str)?;

    // Export physics
    if let Some(ref physics) = result.physics {
        let physics_str = serde_json::to_string_pretty(physics)?;
        fs::write(output_dir.join(format!("{model_name}.physics3.json")), physics_str)?;
    }

    // Write moc3 binary (using mocari's rigged model → runtime path isn't needed for export;
    // we write a placeholder moc3 header for now since the Python version also writes a mock)
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

fn build_motion_json(motion: &Motion) -> String {
    let mut curves = Vec::new();
    for curve in &motion.curves {
        curves.push(serde_json::json!({
            "Target": curve.target,
            "Id": curve.id,
            "Segments": curve.segments
        }));
    }

    let data = serde_json::json!({
        "Version": 3,
        "Meta": {
            "Duration": motion.duration,
            "Fps": motion.fps,
            "Loop": motion.is_loop
        },
        "Curves": curves
    });

    serde_json::to_string_pretty(&data).unwrap_or_default()
}

fn write_mock_moc3(output_dir: &Path, model_name: &str) -> Result<()> {
    let path = output_dir.join(format!("{model_name}.moc3"));
    let mut data = Vec::new();
    data.extend_from_slice(b"MOC3");
    data.extend_from_slice(&3u32.to_le_bytes());
    data.extend_from_slice(&32u32.to_le_bytes()); // data offset
    data.extend_from_slice(&0u32.to_le_bytes()); // data size placeholder
    data.resize(32, 0); // pad to data offset
    fs::write(path, data)?;
    Ok(())
}
