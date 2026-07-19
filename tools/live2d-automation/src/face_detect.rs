use crate::types::{BoundingBox, DetectedPart, FaceDetection, Layer};
use image::RgbaImage;

/// Detect face region using heuristic proportions (anime character layout).
pub fn detect_face(image: &RgbaImage) -> Option<FaceDetection> {
    let w = image.width() as i32;
    let h = image.height() as i32;

    let face_x = (w as f32 * 0.22) as i32;
    let face_y = (h as f32 * 0.02) as i32;
    let face_w = (w as f32 * 0.56) as i32;
    let face_h = (h as f32 * 0.48) as i32;

    let eye_y = face_y + (face_h as f32 * 0.35) as i32;
    let eye_h = ((face_h as f32 * 0.12) as i32).max(1);
    let brow_y = face_y + (face_h as f32 * 0.25) as i32;
    let brow_h = ((face_h as f32 * 0.08) as i32).max(1);
    let mouth_y = face_y + (face_h as f32 * 0.65) as i32;
    let mouth_h = ((face_h as f32 * 0.12) as i32).max(1);

    let parts = vec![
        DetectedPart {
            name: "left_eye".into(),
            bounds: BoundingBox {
                x: face_x + (face_w as f32 * 0.15) as i32,
                y: eye_y,
                width: ((face_w as f32 * 0.18) as i32).max(1),
                height: eye_h,
            },
            confidence: 0.4,
        },
        DetectedPart {
            name: "right_eye".into(),
            bounds: BoundingBox {
                x: face_x + (face_w as f32 * 0.67) as i32,
                y: eye_y,
                width: ((face_w as f32 * 0.18) as i32).max(1),
                height: eye_h,
            },
            confidence: 0.4,
        },
        DetectedPart {
            name: "mouth".into(),
            bounds: BoundingBox {
                x: face_x + (face_w as f32 * 0.35) as i32,
                y: mouth_y,
                width: ((face_w as f32 * 0.3) as i32).max(1),
                height: mouth_h,
            },
            confidence: 0.35,
        },
        DetectedPart {
            name: "nose".into(),
            bounds: BoundingBox {
                x: face_x + (face_w as f32 * 0.42) as i32,
                y: face_y + (face_h as f32 * 0.45) as i32,
                width: ((face_w as f32 * 0.16) as i32).max(1),
                height: ((face_h as f32 * 0.1) as i32).max(1),
            },
            confidence: 0.33,
        },
        DetectedPart {
            name: "left_eyebrow".into(),
            bounds: BoundingBox {
                x: face_x + (face_w as f32 * 0.12) as i32,
                y: brow_y,
                width: ((face_w as f32 * 0.22) as i32).max(1),
                height: brow_h,
            },
            confidence: 0.3,
        },
        DetectedPart {
            name: "right_eyebrow".into(),
            bounds: BoundingBox {
                x: face_x + (face_w as f32 * 0.66) as i32,
                y: brow_y,
                width: ((face_w as f32 * 0.22) as i32).max(1),
                height: brow_h,
            },
            confidence: 0.3,
        },
    ];

    Some(FaceDetection {
        has_face: true,
        face_bounds: BoundingBox {
            x: face_x,
            y: face_y,
            width: face_w,
            height: face_h,
        },
        parts,
    })
}

/// Extract face part layers from the image based on detection.
pub fn extract_face_parts(image: &RgbaImage, detection: &FaceDetection) -> Vec<Layer> {
    use crate::types::z_order_for_part;

    let mut layers = Vec::new();
    for part in &detection.parts {
        let b = &part.bounds;
        let x = b.x.max(0) as u32;
        let y = b.y.max(0) as u32;
        let w = (b.width.min(image.width() as i32 - b.x).max(1)) as u32;
        let h = (b.height.min(image.height() as i32 - b.y).max(1)) as u32;

        if x + w > image.width() || y + h > image.height() {
            continue;
        }

        let cropped = image::imageops::crop_imm(image, x, y, w, h).to_image();
        layers.push(Layer {
            name: part.name.clone(),
            image: cropped,
            bounds: BoundingBox {
                x: b.x,
                y: b.y,
                width: w as i32,
                height: h as i32,
            },
            z_order: z_order_for_part(&part.name),
            confidence: part.confidence,
        });
    }
    layers
}
