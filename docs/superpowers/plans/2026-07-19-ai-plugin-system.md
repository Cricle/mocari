# AI Plugin System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add optional AI integration — AI-assisted rigging and AI-driven runtime character control — as plugins with zero external dependencies.

**Architecture:** Two independent subsystems behind an `ai` feature flag: (1) `AiRigger` trait + `RiggedModel` data types for model creation, (2) `AiDriver` trait for per-frame parameter injection. RiggedModel converts to a live model via a new moc3 binary encoder + existing `load_model_from_bytes()`.

**Tech Stack:** Rust, existing mocari `src/moc3/` and `src/engine/` modules, `serde_json` for model3.json generation.

## Global Constraints

- Zero AI dependencies — mocari defines traits only, users implement them
- `ai` feature flag gates the module; no impact on builds without it
- `AiDriver` runs before engine tick, in registration order
- `load_rigged_model` goes through `load_model_from_bytes()` — no special rendering path
- moc3 encoder targets version V5_0_0 (version byte 5), little-endian
- All new types are `Send + Sync`

---

### Task 1: Feature Flag and Module Scaffolding

**Files:**
- Modify: `Cargo.toml:14-16`
- Modify: `src/lib.rs:40`
- Create: `src/ai/mod.rs`
- Create: `src/ai/error.rs`

**Interfaces:**
- Produces: `mocari::ai::RigError` enum

- [ ] **Step 1: Add `ai` feature to Cargo.toml**

In `Cargo.toml`, add to the `[features]` section:

```toml
[features]
ai = []
```

- [ ] **Step 2: Add module to lib.rs**

In `src/lib.rs`, add after the existing module declarations:

```rust
#[cfg(feature = "ai")]
pub mod ai;
```

- [ ] **Step 3: Create `src/ai/error.rs`**

```rust
use std::fmt;

/// Errors from AI rigging operations.
#[derive(Debug)]
pub enum RigError {
    /// Input image/PSD could not be decoded.
    InvalidInput(String),
    /// AI inference failed.
    InferenceFailed(String),
    /// Output data is invalid or incomplete.
    InvalidOutput(String),
}

impl fmt::Display for RigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(msg) => write!(f, "invalid input: {msg}"),
            Self::InferenceFailed(msg) => write!(f, "inference failed: {msg}"),
            Self::InvalidOutput(msg) => write!(f, "invalid output: {msg}"),
        }
    }
}

impl std::error::Error for RigError {}
```

- [ ] **Step 4: Create `src/ai/mod.rs`**

```rust
//! AI integration traits and types.
//!
//! This module defines traits for AI-assisted model rigging (`AiRigger`)
//! and AI-driven runtime character control (`AiDriver`). mocari provides
//! no implementations — users plug in their preferred AI backend.

mod driver;
mod error;
mod rigger;

pub use driver::AiDriver;
pub use error::RigError;
pub use rigger::{
    AiRigger, DeformerChild, DeformerType, InterpolationType, ParameterKeyframe,
    RiggedDeformer, RiggedMesh, RiggedModel, RiggedParameter,
};
```

- [ ] **Step 5: Create `src/ai/rigger.rs` (stub)**

```rust
use super::RigError;

/// Complete model data produced by an AI rigger.
pub struct RiggedModel {
    pub textures: Vec<Vec<u8>>,
    pub meshes: Vec<RiggedMesh>,
    pub parameters: Vec<RiggedParameter>,
    pub deformers: Vec<RiggedDeformer>,
    pub physics: Option<Vec<u8>>,
    pub motions: Vec<(String, Vec<u8>)>,
    pub expressions: Vec<(String, Vec<u8>)>,
}

pub struct RiggedMesh {
    pub texture_index: usize,
    pub vertices: Vec<[f32; 2]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u16>,
    pub opacity: f32,
}

pub struct RiggedParameter {
    pub id: String,
    pub min: f32,
    pub max: f32,
    pub default: f32,
    pub keyframes: Vec<ParameterKeyframe>,
}

pub struct ParameterKeyframe {
    pub time: f32,
    pub value: f32,
    pub interpolation: InterpolationType,
}

pub enum InterpolationType {
    Linear,
    Bezier([f32; 4]),
    Stepped,
}

pub struct RiggedDeformer {
    pub id: String,
    pub deformer_type: DeformerType,
    pub children: Vec<DeformerChild>,
    pub origin: [f32; 2],
}

pub enum DeformerType {
    Rotation { angle_range: [f32; 2] },
    Warp { vertex_count: usize },
}

pub enum DeformerChild {
    Mesh(usize),
    Deformer(usize),
}

/// Trait for AI-assisted model rigging.
pub trait AiRigger: Send + Sync {
    fn rig_from_image(&self, image: &[u8]) -> Result<RiggedModel, RigError>;
    fn rig_from_psd(&self, psd: &[u8]) -> Result<RiggedModel, RigError>;
    fn rig_from_description(&self, prompt: &str) -> Result<RiggedModel, RigError>;
}
```

- [ ] **Step 6: Create `src/ai/driver.rs`**

```rust
use crate::runtime::RuntimeModel;

/// A per-frame AI driver that injects parameter changes.
///
/// Multiple drivers can be registered simultaneously.
/// They run in registration order, before the engine's tick.
pub trait AiDriver: Send + Sync {
    /// Called every frame before engine tick.
    /// Use `model.set_parameter()` to drive the character.
    fn update(&mut self, delta: f32, model: &mut RuntimeModel);
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check --features ai -p mocari`
Expected: PASS (no errors)

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml src/lib.rs src/ai/
git commit -m "feat: add ai module scaffolding with AiRigger and AiDriver traits"
```

---

### Task 2: moc3 Binary Encoder

**Files:**
- Create: `src/moc3/encode.rs`
- Modify: `src/moc3/mod.rs`

**Interfaces:**
- Consumes: `RiggedModel` from `src/ai/rigger.rs`
- Produces: `encode_moc3(rigged: &RiggedModel) -> Result<Vec<u8>>`

The moc3 binary format (V5_0_0, little-endian):
```
[Header: 64 bytes]
[Section Offsets: 4 * N bytes pointing to each section]
[Count Info section]
[IDs section: UTF-8 strings, null-terminated]
[Canvas Info section]
[Parts section]
[Deformers section]
[Art Meshes section]
[Parameters section]
[Keyform Bindings section]
[Draw Order Groups section]
[... remaining sections]
```

- [ ] **Step 1: Create `src/moc3/encode.rs` with header and offset encoding**

```rust
use crate::ai::{RiggedModel, DeformerType, DeformerChild};
use crate::{Error, Result};

const HEADER_SIZE: usize = 64;
const SECTION_COUNT: usize = 19;

struct Encoder {
    buf: Vec<u8>,
}

impl Encoder {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_u32(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_bytes(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    fn write_null_terminated_string(&mut self, s: &str) {
        self.write_bytes(s.as_bytes());
        self.write_u8(0);
    }

    fn pad_to_alignment(&mut self, align: usize) {
        while self.buf.len() % align != 0 {
            self.buf.push(0);
        }
    }

    fn offset(&self) -> u32 {
        self.buf.len() as u32
    }
}

/// Encodes a `RiggedModel` into a moc3 V5_0_0 binary.
pub fn encode_moc3(rigged: &RiggedModel) -> Result<Vec<u8>> {
    let mut enc = Encoder::new();

    // Pass 1: compute section offsets
    // We need to know the layout before writing, so we compute sizes first.

    // Header is always 64 bytes, offsets table follows
    let header_and_offsets_size = HEADER_SIZE + SECTION_COUNT * 4;

    // Count info section
    let count_info_size = 35 * 4; // V5_0_0 has 35 count fields

    // IDs section: null-terminated strings
    let ids_start = header_and_offsets_size + count_info_size;
    let mut ids_bytes = Vec::new();
    for m in &rigged.meshes {
        ids_bytes.extend_from_slice(m.texture_index.to_string().as_bytes());
        ids_bytes.push(0);
    }
    for p in &rigged.parameters {
        ids_bytes.extend_from_slice(p.id.as_bytes());
        ids_bytes.push(0);
    }
    for d in &rigged.deformers {
        ids_bytes.extend_from_slice(d.id.as_bytes());
        ids_bytes.push(0);
    }
    // Part ID
    ids_bytes.extend_from_slice(b"Part0");
    ids_bytes.push(0);
    let ids_size = ids_bytes.len();

    // Canvas info: 5 floats (ppu, origin_x, origin_y, width, height) + 1 bool = 24 bytes
    let canvas_size = 24;

    // Parts section: 1 part, ~24 bytes
    let parts_count = 1u32;
    let parts_size = parts_count as usize * 24;

    // Deformers section
    let rotation_deformer_count = rigged.deformers.iter()
        .filter(|d| matches!(d.deformer_type, DeformerType::Rotation { .. }))
        .count() as u32;
    let warp_deformer_count = rigged.deformers.iter()
        .filter(|d| matches!(d.deformer_type, DeformerType::Warp { .. }))
        .count() as u32;
    let deformer_count = rotation_deformer_count + warp_deformer_count;
    let deformers_size = (rotation_deformer_count as usize * 48)
        + (warp_deformer_count as usize * 56);

    // Art meshes section
    let art_mesh_count = rigged.meshes.len() as u32;
    let art_meshes_size = art_mesh_count as usize * 80;

    // Parameters section
    let param_count = rigged.parameters.len() as u32;
    let parameters_size = param_count as usize * 16;

    // Keyform bindings
    let keyform_binding_count = param_count;
    let keyform_bindings_size = keyform_binding_count as usize * 8;

    // Draw order groups: 1 group with all meshes
    let draw_order_groups_size = 8;
    let draw_order_group_objects_size = art_mesh_count as usize * 4;

    // Compute section offsets
    let mut offset = header_and_offsets_size as u32;

    let count_info_offset = offset;
    offset += count_info_size as u32;

    let ids_offset = offset;
    offset += ids_size as u32;

    let canvas_info_offset = offset;
    offset += canvas_size as u32;

    let parts_offset = offset;
    offset += parts_size as u32;

    let deformers_offset = offset;
    offset += deformers_size as u32;

    let art_meshes_offset = offset;
    offset += art_meshes_size as u32;

    let parameters_offset = offset;
    offset += parameters_size as u32;

    let keyform_bindings_offset = offset;
    offset += keyform_bindings_size as u32;

    let draw_order_groups_offset = offset;
    offset += draw_order_groups_size as u32;

    let draw_order_group_objects_offset = offset;
    // Remaining sections are zero-size for minimal encoder
    let section_offsets = [
        count_info_offset,
        ids_offset,
        canvas_info_offset,
        parts_offset,
        deformers_offset,
        0, // rotation deformers (included in deformers)
        0, // warp deformers (included in deformers)
        art_meshes_offset,
        parameters_offset,
        0, // part keyforms
        0, // warp deformer keyforms
        0, // rotation deformer keyforms
        0, // art mesh keyforms
        0, // keyform positions
        0, // parameter binding indices
        keyform_bindings_offset,
        0, // parameter bindings
        0, // keys
        draw_order_groups_offset,
        draw_order_group_objects_offset,
        0, // glue
        0, // glue info
        0, // glue keyforms
        0, // keyform multiply colors
        0, // keyform screen colors
    ];

    // --- Write actual binary ---

    // Header (64 bytes)
    enc.write_bytes(b"MOC3");           // magic
    enc.write_u8(5);                     // version V5_0_0
    enc.write_u8(0);                     // little-endian
    enc.write_u8(0);                     // reserved
    enc.write_u8(0);                     // reserved
    // Remaining 56 bytes: reserved zeros
    for _ in 4..HEADER_SIZE {
        enc.write_u8(0);
    }

    // Section offsets table
    for &off in &section_offsets {
        enc.write_u32(off);
    }

    // Count info section (35 u32 fields for V5_0_0)
    let counts = [
        parts_count,          // 0: parts
        deformer_count,       // 1: deformers
        warp_deformer_count,  // 2: warp deformers
        rotation_deformer_count, // 3: rotation deformers
        art_mesh_count,       // 4: art meshes
        param_count,          // 5: parameters
        0u32,                 // 6: part keyforms
        0u32,                 // 7: warp deformer keyforms
        0u32,                 // 8: rotation deformer keyforms
        0u32,                 // 9: art mesh keyforms
        0u32,                 // 10: keyform positions
        0u32,                 // 11: parameter binding indices
        keyform_binding_count, // 12: keyform bindings
        0u32,                 // 13: parameter bindings
        0u32,                 // 14: keys
        0u32,                 // 15: uvs
        0u32,                 // 16: position indices
        0u32,                 // 17: drawable masks
        1u32,                 // 18: draw order groups
        art_mesh_count,       // 19: draw order group objects
        0u32,                 // 20: glue
        0u32,                 // 21: glue info
        0u32,                 // 22: glue keyforms
        0u32,                 // 23: keyform multiply colors
        0u32,                 // 24: keyform screen colors
        0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, 0u32, // 25-34: reserved
    ];
    for &c in &counts {
        enc.write_u32(c);
    }

    // IDs section
    enc.write_bytes(&ids_bytes);

    // Canvas info section
    enc.pad_to_alignment(4);
    enc.write_f32(1.0);   // pixels per unit
    enc.write_f32(0.0);   // origin x
    enc.write_f32(0.0);   // origin y
    enc.write_f32(2.0);   // width
    enc.write_f32(2.0);   // height
    enc.write_u32(0);     // reverse y = false

    // Parts section (1 part)
    enc.pad_to_alignment(4);
    enc.write_i32(-1);    // parent part index (none)
    enc.write_f32(1.0);   // opacity
    enc.write_u32(0);     // part type
    for _ in 0..4 {
        enc.write_u32(0); // reserved
    }

    // Deformers section
    enc.pad_to_alignment(4);
    for d in &rigged.deformers {
        match &d.deformer_type {
            DeformerType::Rotation { angle_range } => {
                enc.write_i32(-1); // parent deformer index
                let child_count = d.children.len() as u32;
                enc.write_u32(child_count);
                enc.write_f32(d.origin[0]);
                enc.write_f32(d.origin[1]);
                enc.write_f32(angle_range[0]);
                enc.write_f32(angle_range[1]);
                for child in &d.children {
                    match child {
                        DeformerChild::Mesh(i) => {
                            enc.write_u32(0); // type: mesh
                            enc.write_u32(*i as u32);
                        }
                        DeformerChild::Deformer(i) => {
                            enc.write_u32(1); // type: deformer
                            enc.write_u32(*i as u32);
                        }
                    }
                }
            }
            DeformerType::Warp { vertex_count } => {
                enc.write_i32(-1); // parent deformer index
                let child_count = d.children.len() as u32;
                enc.write_u32(child_count);
                enc.write_f32(d.origin[0]);
                enc.write_f32(d.origin[1]);
                enc.write_u32(*vertex_count as u32);
                for child in &d.children {
                    match child {
                        DeformerChild::Mesh(i) => {
                            enc.write_u32(0);
                            enc.write_u32(*i as u32);
                        }
                        DeformerChild::Deformer(i) => {
                            enc.write_u32(1);
                            enc.write_u32(*i as u32);
                        }
                    }
                }
            }
        }
    }

    // Art meshes section
    enc.pad_to_alignment(4);
    for m in &rigged.meshes {
        enc.write_i32(m.texture_index as i32);
        enc.write_u32(0x03); // flags: visible + double-sided
        enc.write_f32(m.opacity);
        let vert_count = m.vertices.len() as u32;
        let idx_count = m.indices.len() as u32;
        enc.write_u32(vert_count);
        enc.write_u32(idx_count);
        for v in &m.vertices {
            enc.write_f32(v[0]);
            enc.write_f32(v[1]);
        }
        for uv in &m.uvs {
            enc.write_f32(uv[0]);
            enc.write_f32(uv[1]);
        }
        for idx in &m.indices {
            enc.write_u16(*idx);
        }
    }

    // Parameters section
    enc.pad_to_alignment(4);
    for p in &rigged.parameters {
        enc.write_f32(p.default);
        enc.write_f32(p.min);
        enc.write_f32(p.max);
        let keyframe_count = p.keyframes.len() as u32;
        enc.write_u32(keyframe_count);
    }

    // Keyform bindings
    enc.pad_to_alignment(4);
    for _ in &rigged.parameters {
        enc.write_u32(0); // band index
        enc.write_u32(1); // keyform count
    }

    // Draw order groups (1 group with all meshes)
    enc.pad_to_alignment(4);
    enc.write_u32(0); // part index
    enc.write_u32(art_mesh_count);

    // Draw order group objects
    for i in 0..art_mesh_count {
        enc.write_u32(i); // mesh index
    }

    Ok(enc.buf)
}

trait WriteExt {
    fn write_i32(&mut self, v: i32);
}

impl WriteExt for Encoder {
    fn write_i32(&mut self, v: i32) {
        self.write_u32(v as u32);
    }
}
```

- [ ] **Step 2: Add encode module to `src/moc3/mod.rs`**

Add to `src/moc3/mod.rs`:

```rust
mod encode;

pub use encode::encode_moc3;
```

- [ ] **Step 3: Add round-trip test**

In `src/moc3/encode.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{RiggedModel, RiggedMesh, RiggedParameter, ParameterKeyframe, InterpolationType};

    fn simple_model() -> RiggedModel {
        RiggedModel {
            textures: vec![vec![0u8; 8]], // dummy
            meshes: vec![RiggedMesh {
                texture_index: 0,
                vertices: vec![[-0.5, -0.5], [0.5, -0.5], [0.5, 0.5], [-0.5, 0.5]],
                uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
                indices: vec![0, 1, 2, 0, 2, 3],
                opacity: 1.0,
            }],
            parameters: vec![RiggedParameter {
                id: "ParamAngleX".into(),
                min: -30.0,
                max: 30.0,
                default: 0.0,
                keyframes: vec![],
            }],
            deformers: vec![],
            physics: None,
            motions: vec![],
            expressions: vec![],
        }
    }

    #[test]
    fn encode_produces_valid_header() {
        let rigged = simple_model();
        let bytes = encode_moc3(&rigged).unwrap();
        assert!(bytes.len() >= 64);
        assert_eq!(&bytes[0..4], b"MOC3");
        assert_eq!(bytes[4], 5); // V5_0_0
        assert_eq!(bytes[5], 0); // little-endian
    }

    #[test]
    fn encode_produces_valid_offsets() {
        let rigged = simple_model();
        let bytes = encode_moc3(&rigged).unwrap();
        // First offset at byte 64 should be > 64
        let first_offset = u32::from_le_bytes([bytes[64], bytes[65], bytes[66], bytes[67]]);
        assert!(first_offset as usize > HEADER_SIZE);
    }

    #[test]
    fn encode_produces_non_empty_output() {
        let rigged = simple_model();
        let bytes = encode_moc3(&rigged).unwrap();
        assert!(bytes.len() > 100);
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --features ai -p mocari moc3::encode`
Expected: 3 tests pass

- [ ] **Step 5: Commit**

```bash
git add src/moc3/encode.rs src/moc3/mod.rs
git commit -m "feat: add moc3 binary encoder for RiggedModel"
```

---

### Task 3: model3.json Generator

**Files:**
- Create: `src/ai/model_json.rs`
- Modify: `src/ai/mod.rs`

**Interfaces:**
- Consumes: `RiggedModel` types
- Produces: `generate_model_json(rigged: &RiggedModel, moc_filename: &str) -> String`

- [ ] **Step 1: Create `src/ai/model_json.rs`**

```rust
use serde_json::{json, Value};
use super::rigger::RiggedModel;

/// Generates a model3.json string from a RiggedModel.
pub fn generate_model_json(rigged: &RiggedModel, moc_filename: &str) -> String {
    let textures: Vec<Value> = (0..rigged.textures.len())
        .map(|i| Value::String(format!("texture_{i}.png")))
        .collect();

    let parameters: Vec<Value> = rigged.parameters.iter().map(|p| {
        json!({
            "Id": p.id,
            "Min": p.min,
            "Max": p.max,
            "Default": p.default
        })
    }).collect();

    let motions = if rigged.motions.is_empty() {
        None
    } else {
        let entries: Vec<Value> = rigged.motions.iter().map(|(name, _)| {
            json!({
                "File": format!("{name}.motion3.json")
            })
        }).collect();
        Some(json!({ "Idle": entries }))
    };

    let expressions: Vec<Value> = rigged.expressions.iter().map(|(name, _)| {
        json!({
            "Name": name,
            "File": format!("{name}.exp3.json")
        })
    }).collect();

    let mut file_references = json!({
        "Moc": moc_filename,
        "Textures": textures,
        "Parameters": parameters
    });

    if let Some(m) = motions {
        file_references["Motions"] = m;
    }
    if !expressions.is_empty() {
        file_references["Expressions"] = json!(expressions);
    }

    let model = json!({
        "Version": 3,
        "FileReferences": file_references,
        "Groups": [
            {
                "Target": "Parameter",
                "Name": "LipSync",
                "Ids": []
            },
            {
                "Target": "Parameter",
                "Name": "EyeBlink",
                "Ids": []
            }
        ]
    });

    serde_json::to_string_pretty(&model).unwrap()
}
```

- [ ] **Step 2: Add to `src/ai/mod.rs`**

Add to `src/ai/mod.rs`:

```rust
mod model_json;

pub(crate) use model_json::generate_model_json;
```

- [ ] **Step 3: Add test**

In `src/ai/model_json.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::{RiggedMesh, RiggedParameter, InterpolationType, ParameterKeyframe};

    #[test]
    fn generates_valid_json() {
        let rigged = RiggedModel {
            textures: vec![vec![0u8; 4]],
            meshes: vec![RiggedMesh {
                texture_index: 0,
                vertices: vec![[0.0, 0.0]],
                uvs: vec![[0.0, 0.0]],
                indices: vec![0],
                opacity: 1.0,
            }],
            parameters: vec![RiggedParameter {
                id: "ParamAngleX".into(),
                min: -30.0,
                max: 30.0,
                default: 0.0,
                keyframes: vec![],
            }],
            deformers: vec![],
            physics: None,
            motions: vec![],
            expressions: vec![],
        };

        let json = generate_model_json(&rigged, "model.moc3");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["Version"], 3);
        assert_eq!(parsed["FileReferences"]["Moc"], "model.moc3");
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --features ai -p mocari ai::model_json`
Expected: 1 test passes

- [ ] **Step 5: Commit**

```bash
git add src/ai/model_json.rs src/ai/mod.rs
git commit -m "feat: add model3.json generator from RiggedModel"
```

---

### Task 4: Engine Integration — load_rigged_model

**Files:**
- Modify: `src/engine/mod.rs:515-600` (add `load_rigged_model` near `load_model_from_bytes`)

**Interfaces:**
- Consumes: `encode_moc3` from `src/moc3/encode.rs`, `generate_model_json` from `src/ai/model_json.rs`
- Produces: `Live2dEngine::load_rigged_model(&mut self, rigged: &RiggedModel) -> Result<ModelHandle, EngineError>`

- [ ] **Step 1: Add `load_rigged_model` method to engine**

In `src/engine/mod.rs`, add after the `load_model_from_bytes` method:

```rust
    /// Loads a model from AI-generated rigging data.
    ///
    /// Converts the `RiggedModel` to moc3 binary and model3.json internally,
    /// then loads through the standard pipeline.
    #[cfg(feature = "ai")]
    pub fn load_rigged_model(
        &mut self,
        rigged: &crate::ai::RiggedModel,
    ) -> Result<ModelHandle, EngineError> {
        let moc3_bytes = crate::moc3::encode_moc3(rigged)
            .map_err(|e| EngineError::ModelLoad(e.to_string()))?;
        let model_json = crate::ai::generate_model_json(rigged, "model.moc3");
        let tex_refs: Vec<&[u8]> = rigged.textures.iter().map(|t| t.as_slice()).collect();
        self.load_model_from_bytes(&model_json, &moc3_bytes, &tex_refs)
    }
```

- [ ] **Step 2: Verify compilation**

Run: `cargo check --features ai -p mocari`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/engine/mod.rs
git commit -m "feat: add engine.load_rigged_model() for AI-generated models"
```

---

### Task 5: Engine Integration — AI Drivers

**Files:**
- Modify: `src/engine/mod.rs` (struct fields, `add_driver`, `remove_driver`, `tick`)

**Interfaces:**
- Consumes: `AiDriver` from `src/ai/driver.rs`
- Produces: `Live2dEngine::add_driver()`, `Live2dEngine::remove_driver()`

- [ ] **Step 1: Add `drivers` field to engine struct**

In `src/engine/mod.rs`, in the `Live2dEngine` struct (around line 78):

```rust
pub struct Live2dEngine {
    ctx: WgpuContext,
    renderer: WgpuLive2dRenderer,
    models: Vec<LoadedModel>,
    plugins: Vec<Box<dyn Live2dPlugin>>,
    #[cfg(feature = "ai")]
    drivers: Vec<Box<dyn crate::ai::AiDriver>>,
    frame_callbacks: Vec<FrameCallback>,
    render_callbacks: Vec<RenderCallback>,
    clear_color: Option<wgpu::Color>,
    last_delta: f32,
    needs_redraw: bool,
    msaa_view: wgpu::TextureView,
}
```

- [ ] **Step 2: Initialize `drivers` in constructors**

In the `new()` constructor (around line 104), add:

```rust
            #[cfg(feature = "ai")]
            drivers: Vec::new(),
```

In the `from_wgpu()` constructor (around line 134), add:

```rust
            #[cfg(feature = "ai")]
            drivers: Vec::new(),
```

- [ ] **Step 3: Add `add_driver` and `remove_driver` methods**

```rust
    /// Registers an AI driver that runs every frame before tick.
    #[cfg(feature = "ai")]
    pub fn add_driver(&mut self, driver: Box<dyn crate::ai::AiDriver>) {
        self.drivers.push(driver);
    }

    /// Removes an AI driver by index.
    #[cfg(feature = "ai")]
    pub fn remove_driver(&mut self, index: usize) {
        if index < self.drivers.len() {
            self.drivers.remove(index);
        }
    }
```

- [ ] **Step 4: Update `tick()` to run drivers**

In the `tick()` method (around line 408), add driver execution before the existing model tick:

```rust
    pub fn tick(&mut self, delta: f32) {
        self.last_delta = delta;

        // AI drivers run first — they set parameters before animation tick
        #[cfg(feature = "ai")]
        for model in &mut self.models {
            for driver in &mut self.drivers {
                driver.update(delta, &mut model.runtime);
            }
        }

        for model in &mut self.models {
            // ... existing tick logic
```

- [ ] **Step 5: Verify compilation**

Run: `cargo check --features ai -p mocari`
Expected: PASS

- [ ] **Step 6: Verify compilation without ai feature**

Run: `cargo check -p mocari`
Expected: PASS (no drivers field, no ai imports)

- [ ] **Step 7: Commit**

```bash
git add src/engine/mod.rs
git commit -m "feat: add AI driver support to engine (add_driver, remove_driver)"
```

---

### Task 6: Tests and Clippy

**Files:**
- Modify: `src/ai/driver.rs` (add mock driver test)
- Modify: `src/moc3/encode.rs` (verify tests)
- Modify: `src/ai/model_json.rs` (verify tests)

**Interfaces:**
- All types and methods from Tasks 1-5

- [ ] **Step 1: Add mock driver test**

In `src/ai/driver.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::RuntimeModel;

    struct MockDriver {
        call_count: usize,
    }

    impl MockDriver {
        fn new() -> Self {
            Self { call_count: 0 }
        }
    }

    impl AiDriver for MockDriver {
        fn update(&mut self, _delta: f32, _model: &mut RuntimeModel) {
            self.call_count += 1;
        }
    }

    #[test]
    fn mock_driver_implements_trait() {
        let driver = MockDriver::new();
        let boxed: Box<dyn AiDriver> = Box::new(driver);
        assert_eq!(boxed.call_count, 0); // can't access after boxing, just verify it compiles
    }
}
```

- [ ] **Step 2: Run all ai tests**

Run: `cargo test --features ai -p mocari`
Expected: All tests pass

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --features ai -p mocari -- -D warnings`
Expected: No warnings

- [ ] **Step 4: Fix any clippy warnings**

Address any warnings found.

- [ ] **Step 5: Commit**

```bash
git add src/ai/driver.rs src/moc3/encode.rs src/ai/model_json.rs
git commit -m "test: add AI driver mock test, fix clippy warnings"
```

---

### Task 7: Web Compatibility

**Files:**
- Modify: `src/ai/driver.rs` (remove `Send + Sync` requirement for wasm)

**Interfaces:**
- On wasm32, `AiDriver` should not require `Send + Sync` since wasm is single-threaded.

- [ ] **Step 1: Make AiDriver bounds platform-conditional**

In `src/ai/driver.rs`:

```rust
use crate::runtime::RuntimeModel;

/// A per-frame AI driver that injects parameter changes.
#[cfg(not(target_arch = "wasm32"))]
pub trait AiDriver: Send + Sync {
    fn update(&mut self, delta: f32, model: &mut RuntimeModel);
}

#[cfg(target_arch = "wasm32")]
pub trait AiDriver {
    fn update(&mut self, delta: f32, model: &mut RuntimeModel);
}
```

- [ ] **Step 2: Update engine drivers field for wasm**

In `src/engine/mod.rs`, the `drivers` field needs adjustment:

```rust
    #[cfg(all(feature = "ai", not(target_arch = "wasm32")))]
    drivers: Vec<Box<dyn crate::ai::AiDriver>>,
    #[cfg(all(feature = "ai", target_arch = "wasm32"))]
    drivers: Vec<Box<dyn crate::ai::AiDriver>>,
```

Actually, since both produce the same field, just keep it as:

```rust
    #[cfg(feature = "ai")]
    drivers: Vec<Box<dyn crate::ai::AiDriver>>,
```

The trait bound difference is handled by the conditional trait definition.

- [ ] **Step 3: Verify native compilation**

Run: `cargo check --features ai -p mocari`
Expected: PASS

- [ ] **Step 4: Verify wasm compilation**

Run: `cargo check --target wasm32-unknown-unknown --features ai,web -p mocari`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/ai/driver.rs src/engine/mod.rs
git commit -m "feat: make AiDriver trait wasm-compatible (no Send+Sync on wasm32)"
```
