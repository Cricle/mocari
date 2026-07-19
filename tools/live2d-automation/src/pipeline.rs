use crate::{export, face_detect, layer_gen, mesh, motion, physics, rigging, types::*};
use anyhow::{Context, Result};
use image::RgbaImage;
#[cfg(feature = "parallel")]
use rayon::prelude::*;
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
    let mut body_layers = layer_gen::generate_layers(&rgba, &face);
    body_layers.extend(face_layers);
    body_layers.sort_by_key(|l| l.z_order);
    let layers = body_layers;
    println!("  Generated {} layers", layers.len());

    // Step 3: Generate meshes
    println!("Generating meshes...");
    #[cfg(feature = "parallel")]
    let meshes: Vec<ArtMesh> = layers
        .par_iter()
        .map(|layer| {
            let density = mesh_density_for_part(&layer.name);
            mesh::generate_mesh_for_layer(
                layer.bounds.width as f32,
                layer.bounds.height as f32,
                density,
            )
        })
        .collect();
    #[cfg(not(feature = "parallel"))]
    let meshes: Vec<ArtMesh> = layers
        .iter()
        .map(|layer| {
            let density = mesh_density_for_part(&layer.name);
            mesh::generate_mesh_for_layer(
                layer.bounds.width as f32,
                layer.bounds.height as f32,
                density,
            )
        })
        .collect();
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
    #[cfg(feature = "parallel")]
    let motions: Vec<Motion> = motion_types
        .par_iter()
        .flat_map(|t| motion::generate_motions_for_type(&rigging_result.parameters, t))
        .collect();
    #[cfg(not(feature = "parallel"))]
    let motions: Vec<Motion> = motion_types
        .iter()
        .flat_map(|t| motion::generate_motions_for_type(&rigging_result.parameters, t))
        .collect();
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

/// Get mesh density based on body part type.
fn mesh_density_for_part(name: &str) -> f32 {
    match name {
        n if n.contains("eye") || n.contains("eyebrow") => 0.03,
        n if n.contains("mouth") || n.contains("nose") => 0.025,
        _ => 0.02,
    }
}
