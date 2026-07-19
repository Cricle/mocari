use crate::types::{BoundingBox, FaceDetection, Layer};
use image::RgbaImage;

/// Generate body layers from the full image using the face detection as a guide.
///
/// Creates simplified body part layers based on face position and image dimensions.
pub fn generate_layers(image: &RgbaImage, face: &FaceDetection) -> Vec<Layer> {
    let w = image.width() as i32;
    let h = image.height() as i32;
    let fb = &face.face_bounds;

    let mut layers = Vec::new();

    // Body: lower portion below face
    let body_y = fb.y + fb.height;
    let body_h = h - body_y;
    if body_h > 0 {
        layers.push(make_layer(image, "body", 0, body_y, w, body_h, 1));
    }

    // Head: face region
    layers.push(make_layer(
        image,
        "head",
        fb.x,
        fb.y,
        fb.width,
        fb.height,
        6,
    ));

    // Sort by z-order
    layers.sort_by_key(|l| l.z_order);
    layers
}

fn make_layer(
    image: &RgbaImage,
    name: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    z_order: i32,
) -> Layer {
    let x = x.max(0) as u32;
    let y = y.max(0) as u32;
    let w = (width.min(image.width() as i32 - x as i32).max(1)) as u32;
    let h = (height.min(image.height() as i32 - y as i32).max(1)) as u32;

    let cropped = image::imageops::crop_imm(image, x, y, w, h).to_image();
    Layer {
        name: name.to_string(),
        image: cropped,
        bounds: BoundingBox {
            x: x as i32,
            y: y as i32,
            width: w as i32,
            height: h as i32,
        },
        z_order,
        confidence: 0.5,
    }
}
