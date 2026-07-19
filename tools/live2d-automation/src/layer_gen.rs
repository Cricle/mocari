use crate::types::{z_order_for_part, BoundingBox, FaceDetection, Layer};
use image::{GrayImage, RgbaImage};
use imageproc::contours;
use imageproc::distance_transform::Norm;
use imageproc::edges::canny;
use imageproc::morphology::{close, dilate};

/// Generate body layers from the full image using the face detection as a guide.
///
/// Uses imageproc for contour-based segmentation instead of proportional rectangles.
pub fn generate_layers(image: &RgbaImage, face: &FaceDetection) -> Vec<Layer> {
    let _w = image.width() as i32;
    let h = image.height() as i32;
    let fb = &face.face_bounds;

    let mut layers = Vec::new();

    // Create foreground mask (non-transparent or non-background pixels)
    let mask = create_foreground_mask(image);

    // Find contours for body segmentation
    let contours = contours::find_contours::<i32>(&mask);

    // Group contours into body parts based on position relative to face
    let face_top = fb.y;
    let face_bottom = fb.y + fb.height;

    let mut hair_contours = Vec::new();
    let mut body_contours = Vec::new();
    let mut arm_contours = Vec::new();
    let mut leg_contours = Vec::new();

    for contour in &contours {
        let bounds = contour_bounds(contour);
        let center_y = bounds.y + bounds.height / 2;
        let center_x = bounds.x + bounds.width / 2;

        // Skip tiny contours
        if bounds.width < 10 || bounds.height < 10 {
            continue;
        }

        // Skip the face region itself
        if bounds.x >= fb.x
            && bounds.x + bounds.width <= fb.x + fb.width
            && bounds.y >= fb.y
            && bounds.y + bounds.height <= fb.y + fb.height
        {
            continue;
        }

        // Classify by position
        if center_y < face_top {
            hair_contours.push(bounds);
        } else if center_y > face_bottom && center_y < h - (h - face_bottom) / 3 {
            if center_x < fb.x || center_x > fb.x + fb.width {
                arm_contours.push(bounds);
            } else {
                body_contours.push(bounds);
            }
        } else if center_y >= h - (h - face_bottom) / 3 {
            leg_contours.push(bounds);
        }
    }

    // Generate layers from contours
    if !hair_contours.is_empty() {
        let hair_bounds = merge_bounds(&hair_contours);
        layers.push(make_layer_from_bounds(image, "back_hair", &hair_bounds));
        layers.push(make_layer_from_bounds(image, "front_hair", &hair_bounds));
    }

    if !body_contours.is_empty() {
        let body_bounds = merge_bounds(&body_contours);
        layers.push(make_layer_from_bounds(image, "body", &body_bounds));
    }

    if !arm_contours.is_empty() {
        // Split arms into left and right
        let mid_x = fb.x + fb.width / 2;
        let left_arms: Vec<_> = arm_contours.iter().filter(|b| b.x + b.width / 2 < mid_x).collect();
        let right_arms: Vec<_> = arm_contours.iter().filter(|b| b.x + b.width / 2 >= mid_x).collect();

        if !left_arms.is_empty() {
            let bounds = merge_bounds_ref(&left_arms);
            layers.push(make_layer_from_bounds(image, "left_arm", &bounds));
        }
        if !right_arms.is_empty() {
            let bounds = merge_bounds_ref(&right_arms);
            layers.push(make_layer_from_bounds(image, "right_arm", &bounds));
        }
    }

    if !leg_contours.is_empty() {
        let mid_x = fb.x + fb.width / 2;
        let left_legs: Vec<_> = leg_contours.iter().filter(|b| b.x + b.width / 2 < mid_x).collect();
        let right_legs: Vec<_> = leg_contours.iter().filter(|b| b.x + b.width / 2 >= mid_x).collect();

        if !left_legs.is_empty() {
            let bounds = merge_bounds_ref(&left_legs);
            layers.push(make_layer_from_bounds(image, "left_leg", &bounds));
        }
        if !right_legs.is_empty() {
            let bounds = merge_bounds_ref(&right_legs);
            layers.push(make_layer_from_bounds(image, "right_leg", &bounds));
        }
    }

    // Head: face region
    layers.push(make_layer(image, "head", fb.x, fb.y, fb.width, fb.height));

    layers.sort_by_key(|l| l.z_order);
    layers
}

/// Create a foreground mask from the image (non-transparent pixels).
fn create_foreground_mask(image: &RgbaImage) -> GrayImage {
    let (w, h) = image.dimensions();
    let mut mask = GrayImage::new(w, h);

    for y in 0..h {
        for x in 0..w {
            let pixel = image.get_pixel(x, y);
            // Consider pixel as foreground if alpha > 10 and not near-white background
            if pixel[3] > 10 && !(pixel[0] > 240 && pixel[1] > 240 && pixel[2] > 240) {
                mask.put_pixel(x, y, image::Luma([255]));
            }
        }
    }

    // Use Canny edge detection to refine boundaries
    let gray = image::imageops::grayscale(image);
    let edges = canny(&gray, 30.0, 80.0);

    // Combine mask with edges for better boundary detection
    for y in 0..h {
        for x in 0..w {
            let mask_val = mask.get_pixel(x, y)[0];
            let edge_val = edges.get_pixel(x, y)[0];
            if mask_val > 0 || edge_val > 0 {
                mask.put_pixel(x, y, image::Luma([255]));
            }
        }
    }

    // Clean up mask with morphological operations
    let mask = close(&mask, Norm::L1, 2);
    dilate(&mask, Norm::L1, 1)
}

/// Get bounding box from a contour.
fn contour_bounds(contour: &contours::Contour<i32>) -> BoundingBox {
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

/// Merge multiple bounding boxes into one.
fn merge_bounds(bounds: &[BoundingBox]) -> BoundingBox {
    let mut min_x = i32::MAX;
    let mut max_x = i32::MIN;
    let mut min_y = i32::MAX;
    let mut max_y = i32::MIN;

    for b in bounds {
        min_x = min_x.min(b.x);
        max_x = max_x.max(b.x + b.width);
        min_y = min_y.min(b.y);
        max_y = max_y.max(b.y + b.height);
    }

    BoundingBox {
        x: min_x,
        y: min_y,
        width: max_x - min_x,
        height: max_y - min_y,
    }
}

fn merge_bounds_ref(bounds: &[&BoundingBox]) -> BoundingBox {
    let owned: Vec<BoundingBox> = bounds.iter().map(|&b| b.clone()).collect();
    merge_bounds(&owned)
}

fn make_layer_from_bounds(image: &RgbaImage, name: &str, bounds: &BoundingBox) -> Layer {
    make_layer(image, name, bounds.x, bounds.y, bounds.width, bounds.height)
}

fn make_layer(image: &RgbaImage, name: &str, x: i32, y: i32, width: i32, height: i32) -> Layer {
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
        z_order: z_order_for_part(name),
        confidence: 0.5,
    }
}
