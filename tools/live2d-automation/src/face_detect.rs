use crate::types::{BoundingBox, DetectedPart, FaceDetection, Layer};
use image::RgbaImage;

// Face region proportions (anime character heuristic)
const FACE_X_RATIO: f32 = 0.22;
const FACE_Y_RATIO: f32 = 0.02;
const FACE_W_RATIO: f32 = 0.56;
const FACE_H_RATIO: f32 = 0.48;

// Feature positions relative to face
const EYE_Y_RATIO: f32 = 0.35;
const EYE_H_RATIO: f32 = 0.12;
const BROW_Y_RATIO: f32 = 0.25;
const BROW_H_RATIO: f32 = 0.08;
const MOUTH_Y_RATIO: f32 = 0.65;
const MOUTH_H_RATIO: f32 = 0.12;

const LEFT_EYE_X: f32 = 0.15;
const LEFT_EYE_W: f32 = 0.18;
const RIGHT_EYE_X: f32 = 0.67;
const RIGHT_EYE_W: f32 = 0.18;
const MOUTH_X: f32 = 0.35;
const MOUTH_W: f32 = 0.3;
const NOSE_X: f32 = 0.42;
const NOSE_Y: f32 = 0.45;
const NOSE_W: f32 = 0.16;
const NOSE_H: f32 = 0.1;
const LEFT_BROW_X: f32 = 0.12;
const LEFT_BROW_W: f32 = 0.22;
const RIGHT_BROW_X: f32 = 0.66;
const RIGHT_BROW_W: f32 = 0.22;

/// Detect face region using heuristic proportions (anime character layout).
pub fn detect_face(image: &RgbaImage) -> Option<FaceDetection> {
    let w = image.width() as f32;
    let h = image.height() as f32;

    let face_x = (w * FACE_X_RATIO) as i32;
    let face_y = (h * FACE_Y_RATIO) as i32;
    let face_w = (w * FACE_W_RATIO) as i32;
    let face_h = (h * FACE_H_RATIO) as i32;

    let eye_y = face_y + (face_h as f32 * EYE_Y_RATIO) as i32;
    let eye_h = ((face_h as f32 * EYE_H_RATIO) as i32).max(1);
    let brow_y = face_y + (face_h as f32 * BROW_Y_RATIO) as i32;
    let brow_h = ((face_h as f32 * BROW_H_RATIO) as i32).max(1);
    let mouth_y = face_y + (face_h as f32 * MOUTH_Y_RATIO) as i32;
    let mouth_h = ((face_h as f32 * MOUTH_H_RATIO) as i32).max(1);

    let part = |name: &str, x_ratio: f32, y: i32, w_ratio: f32, h: i32, conf: f32| -> DetectedPart {
        DetectedPart {
            name: name.into(),
            bounds: BoundingBox {
                x: face_x + (face_w as f32 * x_ratio) as i32,
                y,
                width: ((face_w as f32 * w_ratio) as i32).max(1),
                height: h,
            },
            confidence: conf,
        }
    };

    let parts = vec![
        part("left_eye", LEFT_EYE_X, eye_y, LEFT_EYE_W, eye_h, 0.4),
        part("right_eye", RIGHT_EYE_X, eye_y, RIGHT_EYE_W, eye_h, 0.4),
        part("mouth", MOUTH_X, mouth_y, MOUTH_W, mouth_h, 0.35),
        part("nose", NOSE_X, face_y + (face_h as f32 * NOSE_Y) as i32, NOSE_W, ((face_h as f32 * NOSE_H) as i32).max(1), 0.33),
        part("left_eyebrow", LEFT_BROW_X, brow_y, LEFT_BROW_W, brow_h, 0.3),
        part("right_eyebrow", RIGHT_BROW_X, brow_y, RIGHT_BROW_W, brow_h, 0.3),
    ];

    Some(FaceDetection {
        face_bounds: BoundingBox { x: face_x, y: face_y, width: face_w, height: face_h },
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
            bounds: BoundingBox { x: b.x, y: b.y, width: w as i32, height: h as i32 },
            z_order: z_order_for_part(&part.name),
            confidence: part.confidence,
        });
    }
    layers
}
