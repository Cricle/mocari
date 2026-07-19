pub mod config;
pub mod export;
pub mod face_detect;
pub mod generators;
pub mod layer_gen;
pub mod mesh;
pub mod motion;
pub mod physics;
pub mod pipeline;
pub mod rigging;
pub mod types;

#[cfg(feature = "parallel")]
use rayon::prelude::*;
use types::*;

/// Generate a Live2D model from image bytes (for WASM/web use).
pub fn generate_model_from_bytes(
    image_bytes: &[u8],
    model_name: &str,
    motion_types: &[String],
) -> Result<ModelBundle, String> {
    let img = image::load_from_memory(image_bytes).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();

    // Detect face
    let face = face_detect::detect_face(&rgba).ok_or("No face detected")?;

    // Generate layers
    let face_layers = face_detect::extract_face_parts(&rgba, &face);
    let mut body_layers = layer_gen::generate_layers(&rgba, &face);
    body_layers.extend(face_layers);
    body_layers.sort_by_key(|l| l.z_order);
    let layers = body_layers;

    // Generate meshes
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

    // Setup rigging
    let rigging_result = rigging::setup_rigging(&layers);

    // Configure physics
    let physics_data = physics::configure_physics(&rigging_result.parameters);

    // Generate motions
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

    // Build result
    let result = PipelineResult {
        layers,
        meshes,
        rigging: rigging_result,
        motions,
        physics: Some(physics_data),
    };

    // Export to bytes
    export_to_bytes(&result, model_name)
}

/// Model bundle containing all generated data.
pub struct ModelBundle {
    pub model_json: String,
    pub moc3: Vec<u8>,
    pub textures: Vec<Vec<u8>>,
    pub motions: Vec<(String, Vec<u8>)>,
    pub physics: Option<String>,
}

fn mesh_density_for_part(name: &str) -> f32 {
    match name {
        n if n.contains("eye") || n.contains("eyebrow") => 0.03,
        n if n.contains("mouth") || n.contains("nose") => 0.025,
        _ => 0.02,
    }
}

fn export_to_bytes(result: &PipelineResult, model_name: &str) -> Result<ModelBundle, String> {
    use image::ImageEncoder;

    // Export textures (resize to fit GPU limits, max 2048x2048)
    let max_size = 2048;
    let mut textures = Vec::new();
    for layer in &result.layers {
        let img = &layer.image;
        let (w, h) = (img.width(), img.height());

        // Resize if needed
        let resized = if w > max_size || h > max_size {
            let scale = (max_size as f32 / w.max(h) as f32).min(1.0);
            let new_w = (w as f32 * scale) as u32;
            let new_h = (h as f32 * scale) as u32;
            image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Lanczos3)
        } else {
            img.clone()
        };

        let mut buf = Vec::new();
        image::codecs::png::PngEncoder::new(&mut buf)
            .write_image(
                resized.as_raw(),
                resized.width(),
                resized.height(),
                image::ExtendedColorType::Rgba8,
            )
            .map_err(|e| e.to_string())?;
        textures.push(buf);
    }

    // Export motions
    let mut motions = Vec::new();
    for motion in &result.motions {
        let json = export::build_motion_json(motion).map_err(|e| e.to_string())?;
        motions.push((motion.name.clone(), json.into_bytes()));
    }

    // Build model3.json
    let model_json = export::build_model3_json(model_name, result);

    // Use a real moc3 file as template (Haru model)
    // This allows the model to load and display with the generated textures
    let moc3 = include_bytes!("../../../assets/models/Haru/Haru.moc3").to_vec();

    // Physics JSON
    let physics = result
        .physics
        .as_ref()
        .map(|p| serde_json::to_string_pretty(p).unwrap_or_default());

    Ok(ModelBundle {
        model_json,
        moc3,
        textures,
        motions,
        physics,
    })
}
