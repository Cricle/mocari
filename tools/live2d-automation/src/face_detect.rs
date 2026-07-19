use crate::types::{BoundingBox, DetectedPart, FaceDetection, Layer};
use image::RgbaImage;
use imageproc::contours;
use imageproc::contrast::adaptive_threshold;
use imageproc::edges::canny;

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

// Skin color detection thresholds (HSV-like)
const SKIN_R_MIN: u8 = 95;
const SKIN_G_MIN: u8 = 40;
const SKIN_B_MIN: u8 = 20;
const SKIN_MAX_DIFF: u8 = 15;
const SKIN_MIN_CR: f32 = 1.0;
const SKIN_MAX_CR: f32 = 1.6;

/// Detect face region using multiple backends with graceful fallback.
pub fn detect_face(image: &RgbaImage) -> Option<FaceDetection> {
    // Try skin-color detection first
    if let Some(detection) = detect_face_skin_color(image) {
        return Some(detection);
    }

    // Fall back to heuristic detection
    detect_face_heuristic(image)
}

/// Detect face using skin-color segmentation (no OpenCV required).
fn detect_face_skin_color(image: &RgbaImage) -> Option<FaceDetection> {
    let width = image.width();
    let height = image.height();

    // Find skin-colored pixels
    let mut skin_mask = vec![false; (width * height) as usize];
    let mut min_x = width;
    let mut max_x = 0;
    let mut min_y = height;
    let mut max_y = 0;

    for y in 0..height {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            let r = pixel[0];
            let g = pixel[1];
            let b = pixel[2];

            if is_skin_pixel(r, g, b) {
                skin_mask[(y * width + x) as usize] = true;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }

    // Check if we found enough skin pixels
    let skin_count = skin_mask.iter().filter(|&&x| x).count();
    let total_pixels = (width * height) as usize;
    let skin_ratio = skin_count as f32 / total_pixels as f32;

    if !(0.05..=0.6).contains(&skin_ratio) {
        return None; // Too little or too much skin
    }

    // Find the largest connected component (face)
    let face_bounds = find_largest_component(&skin_mask, width, height, min_x, max_x, min_y, max_y)?;

    // Validate face proportions
    let face_w = face_bounds.width as f32;
    let face_h = face_bounds.height as f32;
    let aspect_ratio = face_w / face_h;

    if !(0.5..=1.5).contains(&aspect_ratio) {
        return None; // Not a valid face shape
    }

    // Extract facial features from face region using edge detection
    let parts = detect_features_with_edges(image, &face_bounds);

    Some(FaceDetection {
        face_bounds,
        parts,
    })
}

/// Check if a pixel is likely skin-colored.
fn is_skin_pixel(r: u8, g: u8, b: u8) -> bool {
    // RGB rule-based skin detection
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);

    if r <= SKIN_R_MIN || g <= SKIN_G_MIN || b <= SKIN_B_MIN {
        return false;
    }

    if max - min <= SKIN_MIN_CR as u8 {
        return false;
    }

    let diff = (r as i32 - g as i32).unsigned_abs() as u8;
    if diff > SKIN_MAX_DIFF {
        return false;
    }

    // YCbCr-like check
    let r_f = r as f32;
    let g_f = g as f32;
    let b_f = b as f32;

    let cr = 0.5 * r_f - 0.4187 * g_f - 0.0813 * b_f + 128.0;
    (SKIN_MIN_CR * 100.0..=SKIN_MAX_CR * 100.0).contains(&cr)
}

/// Find the largest connected component in the skin mask.
fn find_largest_component(
    mask: &[bool],
    width: u32,
    _height: u32,
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
) -> Option<BoundingBox> {
    // Simple bounding box approach (faster than full connected components)
    let face_w = max_x - min_x + 1;
    let face_h = max_y - min_y + 1;

    // Validate minimum size
    if face_w < 20 || face_h < 20 {
        return None;
    }

    // Check density within bounding box
    let mut skin_in_box = 0;
    let total_in_box = face_w * face_h;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            if mask[(y * width + x) as usize] {
                skin_in_box += 1;
            }
        }
    }

    let density = skin_in_box as f32 / total_in_box as f32;
    if density < 0.3 {
        return None; // Not dense enough
    }

    Some(BoundingBox {
        x: min_x as i32,
        y: min_y as i32,
        width: face_w as i32,
        height: face_h as i32,
    })
}

/// Detect facial features using Canny edge detection and contour analysis.
fn detect_features_with_edges(image: &RgbaImage, face: &BoundingBox) -> Vec<DetectedPart> {
    let x = face.x.max(0) as u32;
    let y = face.y.max(0) as u32;
    let w = (face.width.min(image.width() as i32 - face.x).max(1)) as u32;
    let h = (face.height.min(image.height() as i32 - face.y).max(1)) as u32;

    if x + w > image.width() || y + h > image.height() {
        return extract_features_from_face(face);
    }

    // Crop face region
    let face_crop = image::imageops::crop_imm(image, x, y, w, h).to_image();

    // Convert to grayscale
    let gray = image::imageops::grayscale(&face_crop);

    // Apply Canny edge detection for precise boundaries
    let edges = canny(&gray, 50.0, 100.0);

    // Also apply adaptive threshold for region detection
    let thresholded = adaptive_threshold(&gray, 15);

    // Combine edges and threshold for better feature detection
    let mut combined = edges.clone();
    for y in 0..combined.height() {
        for x in 0..combined.width() {
            let edge_val = edges.get_pixel(x, y)[0];
            let thresh_val = thresholded.get_pixel(x, y)[0];
            if edge_val > 0 || thresh_val > 0 {
                combined.put_pixel(x, y, image::Luma([255]));
            }
        }
    }

    // Find contours in the combined image
    let feature_contours = contours::find_contours::<i32>(&combined);

    let mut parts = Vec::new();
    let face_w = face.width as f32;
    let face_h = face.height as f32;

    // Classify contours by position within face
    for contour in &feature_contours {
        if contour.points.len() < 5 {
            continue;
        }

        let bounds = contour_bounds_local(contour);
        let center_x = (bounds.x + bounds.width / 2) as f32 / face_w;
        let center_y = (bounds.y + bounds.height / 2) as f32 / face_h;
        let size_ratio = (bounds.width * bounds.height) as f32 / (face_w * face_h);

        // Skip very small or very large features
        if !(0.001..=0.2).contains(&size_ratio) {
            continue;
        }

        // Classify by position
        let part_name = if center_y > 0.2 && center_y < 0.5 {
            if center_x < 0.35 {
                "left_eye"
            } else if center_x > 0.65 {
                "right_eye"
            } else if center_x > 0.35 && center_x < 0.65 {
                "nose"
            } else {
                continue;
            }
        } else if center_y > 0.5 && center_y < 0.75 {
            if center_x > 0.25 && center_x < 0.75 {
                "mouth"
            } else {
                continue;
            }
        } else if center_y > 0.1 && center_y < 0.3 {
            if center_x < 0.35 {
                "left_eyebrow"
            } else if center_x > 0.65 {
                "right_eyebrow"
            } else {
                continue;
            }
        } else {
            continue;
        };

        // Convert back to image coordinates
        let abs_bounds = BoundingBox {
            x: face.x + bounds.x,
            y: face.y + bounds.y,
            width: bounds.width,
            height: bounds.height,
        };

        parts.push(DetectedPart {
            name: part_name.to_string(),
            bounds: abs_bounds,
            confidence: 0.7,
        });
    }

    // Add eye highlights if we found eyes
    let left_eye_bounds = parts.iter().find(|p| p.name == "left_eye").map(|p| p.bounds.clone());
    let right_eye_bounds = parts.iter().find(|p| p.name == "right_eye").map(|p| p.bounds.clone());

    if let (Some(left), Some(right)) = (left_eye_bounds, right_eye_bounds) {
        // Add highlights (small bright spots in upper-center of each eye)
        parts.push(DetectedPart {
            name: "left_eye_highlight".into(),
            bounds: BoundingBox {
                x: left.x + (left.width as f32 * 0.3) as i32,
                y: left.y + (left.height as f32 * 0.2) as i32,
                width: ((left.width as f32 * 0.3) as i32).max(1),
                height: ((left.height as f32 * 0.3) as i32).max(1),
            },
            confidence: 0.6,
        });

        parts.push(DetectedPart {
            name: "right_eye_highlight".into(),
            bounds: BoundingBox {
                x: right.x + (right.width as f32 * 0.3) as i32,
                y: right.y + (right.height as f32 * 0.2) as i32,
                width: ((right.width as f32 * 0.3) as i32).max(1),
                height: ((right.height as f32 * 0.3) as i32).max(1),
            },
            confidence: 0.6,
        });
    }

    // If we didn't find enough features, fall back to heuristic
    if parts.len() < 4 {
        return extract_features_from_face(face);
    }

    parts
}

fn contour_bounds_local(contour: &contours::Contour<i32>) -> BoundingBox {
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for point in &contour.points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }

    BoundingBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x + 1,
        height: max_y - min_y + 1,
    }
}

/// Extract facial features from detected face region.
fn extract_features_from_face(face: &BoundingBox) -> Vec<DetectedPart> {
    let x = face.x;
    let y = face.y;
    let w = face.width;
    let h = face.height;

    let eye_y = y + (h as f32 * EYE_Y_RATIO) as i32;
    let eye_h = ((h as f32 * EYE_H_RATIO) as i32).max(1);
    let brow_y = y + (h as f32 * BROW_Y_RATIO) as i32;
    let brow_h = ((h as f32 * BROW_H_RATIO) as i32).max(1);
    let mouth_y = y + (h as f32 * MOUTH_Y_RATIO) as i32;
    let mouth_h = ((h as f32 * MOUTH_H_RATIO) as i32).max(1);

    let part = |name: &str, x_ratio: f32, y: i32, w_ratio: f32, h: i32, conf: f32| -> DetectedPart {
        DetectedPart {
            name: name.into(),
            bounds: BoundingBox {
                x: x + (w as f32 * x_ratio) as i32,
                y,
                width: ((w as f32 * w_ratio) as i32).max(1),
                height: h,
            },
            confidence: conf,
        }
    };

    vec![
        part("left_eye", LEFT_EYE_X, eye_y, LEFT_EYE_W, eye_h, 0.6),
        part("right_eye", RIGHT_EYE_X, eye_y, RIGHT_EYE_W, eye_h, 0.6),
        part("mouth", MOUTH_X, mouth_y, MOUTH_W, mouth_h, 0.5),
        part("nose", NOSE_X, y + (h as f32 * NOSE_Y) as i32, NOSE_W, ((h as f32 * NOSE_H) as i32).max(1), 0.5),
        part("left_eyebrow", LEFT_BROW_X, brow_y, LEFT_BROW_W, brow_h, 0.4),
        part("right_eyebrow", RIGHT_BROW_X, brow_y, RIGHT_BROW_W, brow_h, 0.4),
        part("left_eye_highlight", LEFT_EYE_X + 0.03, eye_y + (eye_h as f32 * 0.2) as i32, 0.06, (eye_h as f32 * 0.3) as i32, 0.35),
        part("right_eye_highlight", RIGHT_EYE_X + 0.03, eye_y + (eye_h as f32 * 0.2) as i32, 0.06, (eye_h as f32 * 0.3) as i32, 0.35),
    ]
}

/// Detect face region using heuristic proportions (anime character layout).
fn detect_face_heuristic(image: &RgbaImage) -> Option<FaceDetection> {
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
        part("left_eye_highlight", LEFT_EYE_X + 0.03, eye_y + (eye_h as f32 * 0.2) as i32, 0.06, (eye_h as f32 * 0.3) as i32, 0.25),
        part("right_eye_highlight", RIGHT_EYE_X + 0.03, eye_y + (eye_h as f32 * 0.2) as i32, 0.06, (eye_h as f32 * 0.3) as i32, 0.25),
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
