use crate::types::ArtMesh;
use rand::Rng;
use spade::{DelaunayTriangulation, Point2, Triangulation};

const MIN_TRIANGLE_AREA: f32 = 100.0;

/// Generate a Delaunay-triangulated mesh for a layer.
pub fn generate_mesh_for_layer(width: f32, height: f32, density: f32) -> ArtMesh {
    let nx = (3.max((width * density) as i32)) as usize;
    let ny = (3.max((height * density) as i32)) as usize;

    let mut points = generate_grid_points(width, height, nx, ny);
    points.extend(generate_edge_points(width, height));

    match triangulate(&points) {
        Some((verts, uvs, indices)) => ArtMesh {
            vertices: verts,
            uvs,
            indices,
        },
        None => simple_grid_mesh(width, height),
    }
}

fn generate_grid_points(width: f32, height: f32, nx: usize, ny: usize) -> Vec<[f32; 2]> {
    let mut rng = rand::thread_rng();
    let mut points = Vec::with_capacity(nx * ny);
    for j in 0..ny {
        for i in 0..nx {
            let x = if nx > 1 {
                width * (i as f32) / ((nx - 1) as f32)
            } else {
                width * 0.5
            };
            let y = if ny > 1 {
                height * (j as f32) / ((ny - 1) as f32)
            } else {
                height * 0.5
            };
            let noise_x: f32 = rng.gen_range(-2.0..2.0);
            let noise_y: f32 = rng.gen_range(-2.0..2.0);
            points.push([
                (x + noise_x).clamp(0.0, width),
                (y + noise_y).clamp(0.0, height),
            ]);
        }
    }
    points
}

fn generate_edge_points(width: f32, height: f32) -> Vec<[f32; 2]> {
    let mut points = Vec::new();
    for i in 0..5 {
        points.push([width * (i as f32) / 4.0, 0.0]);
        points.push([width * (i as f32) / 4.0, height]);
    }
    for i in 0..3 {
        points.push([0.0, height * (i as f32) / 2.0]);
        points.push([width, height * (i as f32) / 2.0]);
    }
    points
}

fn triangle_area(p1: [f32; 2], p2: [f32; 2], p3: [f32; 2]) -> f32 {
    0.5 * ((p2[0] - p1[0]) * (p3[1] - p1[1]) - (p3[0] - p1[0]) * (p2[1] - p1[1])).abs()
}

type TriResult = (Vec<[f32; 2]>, Vec<[f32; 2]>, Vec<u16>);
fn triangulate(points: &[[f32; 2]]) -> Option<TriResult> {
    if points.len() < 3 {
        return None;
    }

    let spade_points: Vec<Point2<f64>> = points
        .iter()
        .map(|p| Point2::new(p[0] as f64, p[1] as f64))
        .collect();

    let triangulation: DelaunayTriangulation<Point2<f64>> =
        DelaunayTriangulation::bulk_load(spade_points).ok()?;

    let width = points.iter().map(|p| p[0]).fold(0.0f32, f32::max);
    let height = points.iter().map(|p| p[1]).fold(0.0f32, f32::max);

    let mut vertices = Vec::new();
    let mut uvs = Vec::new();
    for p in points {
        vertices.push(*p);
        uvs.push([
            if width > 0.0 { p[0] / width } else { 0.0 },
            if height > 0.0 { p[1] / height } else { 0.0 },
        ]);
    }

    let mut indices = Vec::new();
    for face in triangulation.inner_faces() {
        let v = face.vertices();
        let pos0 = v[0].position();
        let pos1 = v[1].position();
        let pos2 = v[2].position();
        let p1 = [pos0.x as f32, pos0.y as f32];
        let p2 = [pos1.x as f32, pos1.y as f32];
        let p3 = [pos2.x as f32, pos2.y as f32];

        if triangle_area(p1, p2, p3) < MIN_TRIANGLE_AREA {
            continue;
        }

        let i1 = find_point_index(points, p1);
        let i2 = find_point_index(points, p2);
        let i3 = find_point_index(points, p3);

        if let (Some(a), Some(b), Some(c)) = (i1, i2, i3) {
            indices.push(a as u16);
            indices.push(b as u16);
            indices.push(c as u16);
        }
    }

    if indices.is_empty() {
        return None;
    }

    Some((vertices, uvs, indices))
}

fn find_point_index(points: &[[f32; 2]], target: [f32; 2]) -> Option<usize> {
    points.iter().position(|p| {
        (p[0] - target[0]).abs() < 0.01 && (p[1] - target[1]).abs() < 0.01
    })
}

fn simple_grid_mesh(width: f32, height: f32) -> ArtMesh {
    let nx = 4;
    let ny = 4;
    let mut vertices = Vec::new();
    let mut uvs = Vec::new();

    for j in 0..ny {
        for i in 0..nx {
            let x = width * (i as f32) / ((nx - 1) as f32);
            let y = height * (j as f32) / ((ny - 1) as f32);
            vertices.push([x, y]);
            uvs.push([
                if width > 0.0 { x / width } else { 0.0 },
                if height > 0.0 { y / height } else { 0.0 },
            ]);
        }
    }

    let mut indices = Vec::new();
    for j in 0..(ny - 1) {
        for i in 0..(nx - 1) {
            let tl = (j * nx + i) as u16;
            let tr = tl + 1;
            let bl = ((j + 1) * nx + i) as u16;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, tr, bl]);
            indices.extend_from_slice(&[tr, br, bl]);
        }
    }

    ArtMesh {
        vertices,
        uvs,
        indices,
    }
}
