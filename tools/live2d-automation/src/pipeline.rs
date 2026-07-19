use crate::{export, face_detect, layer_gen, mesh, motion, physics, rigging, types::*};
use anyhow::{Context, Result};
use image::RgbaImage;
use std::path::Path;

/// Run the full automation pipeline: image → face detect → layers → mesh → rigging → physics → motions → export.
pub fn run_pipeline(
    image_path: &str,
    output_dir: &str,
    model_name: &str,
    motion_types: &[String],
) -> Result<()> {
    println!("Loading image: {image_path}");
    let img = image::open(image_path).context("failed to open image")?;
    let rgba: RgbaImage = img.to_rgba8();

    // Step 1: Detect face
    println!("Detecting face...");
    let face = face_detect::detect_face(&rgba).context("no face detected")?;

    // Step 2: Generate layers
    println!("Generating layers...");
    let face_layers = face_detect::extract_face_parts(&rgba, &face);
    let body_layers = layer_gen::generate_layers(&rgba, &face);
    let mut layers = body_layers;
    layers.extend(face_layers);
    layers.sort_by_key(|l| l.z_order);
    println!("  Generated {} layers", layers.len());

    // Step 3: Generate meshes
    println!("Generating meshes...");
    let mut meshes = Vec::new();
    for layer in &layers {
        let density = if layer.name.contains("eye") || layer.name.contains("eyebrow") {
            0.03
        } else if layer.name.contains("mouth") || layer.name.contains("nose") {
            0.025
        } else {
            0.02
        };
        let art_mesh = mesh::generate_mesh_for_layer(
            layer.bounds.width as f32,
            layer.bounds.height as f32,
            density,
        );
        meshes.push(art_mesh);
    }
    println!("  Generated {} meshes", meshes.len());

    // Step 4: Setup rigging
    println!("Setting up rigging...");
    let rigging_result = rigging::setup_rigging(&layers);
    println!(
        "  {} bones, {} parameters",
        rigging_result.bones.len(),
        rigging_result.parameters.len()
    );

    // Step 5: Configure physics
    println!("Configuring physics...");
    let physics_data = physics::configure_physics(&rigging_result.parameters);

    // Step 6: Generate motions
    println!("Generating motions ({})...", motion_types.join(", "));
    let motions = motion::generate_motions(&rigging_result.parameters, motion_types);
    println!("  Generated {} motions", motions.len());

    // Step 7: Export
    println!("Exporting model bundle...");
    let result = PipelineResult {
        layers,
        meshes,
        rigging: rigging_result,
        motions,
        physics: Some(physics_data),
    };
    export::export_bundle(&result, model_name, Path::new(output_dir))?;

    println!("Done!");
    Ok(())
}
