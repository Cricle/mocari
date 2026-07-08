use crate::moc3::{Moc3DrawableMesh, Moc3DrawableVertex};

#[derive(Debug, Copy, Clone, PartialEq)]
/// A renderer-ready Live2D drawable vertex.
///
/// The layout is two `f32` position values, two `f32` UV values, one `f32`
/// opacity value, three `f32` multiply-color channels, and three `f32`
/// screen-color channels.
pub struct DrawableVertex {
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
}

impl DrawableVertex {
    /// Size in bytes of one encoded vertex.
    pub const STRIDE: usize = 44;

    /// Creates a vertex with default multiply and screen colors.
    pub fn new(position: [f32; 2], uv: [f32; 2], opacity: f32) -> Self {
        Self::with_colors(position, uv, opacity, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0])
    }

    /// Creates a vertex with explicit multiply and screen colors.
    pub fn with_colors(
        position: [f32; 2],
        uv: [f32; 2],
        opacity: f32,
        multiply: [f32; 3],
        screen: [f32; 3],
    ) -> Self {
        Self {
            position,
            uv,
            opacity,
            multiply,
            screen,
        }
    }

    /// Returns the model-space vertex position.
    pub fn position(&self) -> [f32; 2] {
        self.position
    }

    /// Returns the texture coordinate.
    pub fn uv(&self) -> [f32; 2] {
        self.uv
    }

    /// Returns the drawable opacity copied onto this vertex.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    /// Returns the drawable multiply color copied onto this vertex.
    pub fn multiply(&self) -> [f32; 3] {
        self.multiply
    }

    /// Returns the drawable screen color copied onto this vertex.
    pub fn screen(&self) -> [f32; 3] {
        self.screen
    }
}

/// Converts all vertices from a drawable mesh into [`DrawableVertex`] values.
pub fn vertices_from_drawable(mesh: &Moc3DrawableMesh) -> Vec<DrawableVertex> {
    mesh.vertices()
        .iter()
        .map(|vertex| {
            vertex_from_drawable_vertex(
                vertex,
                mesh.opacity(),
                mesh.multiply_color(),
                mesh.screen_color(),
            )
        })
        .collect()
}

/// Encodes a drawable mesh's vertices into native-endian `f32` bytes.
///
/// The output buffer is cleared before new bytes are appended.
pub fn encode_vertices_from_drawable(mesh: &Moc3DrawableMesh, bytes: &mut Vec<u8>) {
    bytes.clear();
    bytes.reserve(mesh.vertices().len() * DrawableVertex::STRIDE);
    for vertex in mesh.vertices() {
        encode_vertex(
            vertex.position(),
            vertex.uv(),
            mesh.opacity(),
            mesh.multiply_color(),
            mesh.screen_color(),
            bytes,
        );
    }
}

/// Converts one raw MOC3 drawable vertex into a renderer-ready vertex.
pub fn vertex_from_drawable_vertex(
    vertex: &Moc3DrawableVertex,
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
) -> DrawableVertex {
    DrawableVertex::with_colors(vertex.position(), vertex.uv(), opacity, multiply, screen)
}

/// Encodes vertices into native-endian `f32` bytes.
pub fn encode_vertices(vertices: &[DrawableVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * DrawableVertex::STRIDE);
    for vertex in vertices {
        encode_vertex(
            vertex.position,
            vertex.uv,
            vertex.opacity,
            vertex.multiply,
            vertex.screen,
            &mut bytes,
        );
    }

    bytes
}

fn encode_vertex(
    position: [f32; 2],
    uv: [f32; 2],
    opacity: f32,
    multiply: [f32; 3],
    screen: [f32; 3],
    bytes: &mut Vec<u8>,
) {
    bytes.extend_from_slice(&position[0].to_ne_bytes());
    bytes.extend_from_slice(&position[1].to_ne_bytes());
    bytes.extend_from_slice(&uv[0].to_ne_bytes());
    bytes.extend_from_slice(&uv[1].to_ne_bytes());
    bytes.extend_from_slice(&opacity.to_ne_bytes());
    for channel in multiply {
        bytes.extend_from_slice(&channel.to_ne_bytes());
    }
    for channel in screen {
        bytes.extend_from_slice(&channel.to_ne_bytes());
    }
}

/// Encodes `u16` mesh indices into native-endian bytes.
pub fn encode_indices(indices: &[u16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(indices.len() * 2);
    for index in indices {
        bytes.extend_from_slice(&index.to_ne_bytes());
    }

    bytes
}
