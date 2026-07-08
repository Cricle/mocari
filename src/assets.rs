//! Convenience loaders for complete Cubism model folders.
//!
//! These helpers start from a `.model3.json` path and resolve the files it
//! references relative to that model file. Use [`crate::assets::load_model_runtime`]
//! when the application needs per-frame parameter, motion, or expression updates.
//! Use [`crate::assets::load_model`] for a static default-pose snapshot.

use std::{fmt, fs, path::Path};

use crate::{
    json::{Model3, Pose3},
    moc3::{
        Moc3ArtMeshKeyforms, Moc3ArtMeshes, Moc3CanvasInfo, Moc3Deformers, Moc3DrawOrderGroups,
        Moc3DrawableMesh, Moc3Glues, Moc3Ids, Moc3KeyformBindings, Moc3OffscreenInfo, Moc3Parts,
        build_moc3_drawable_meshes_for_default_pose_with_offscreen_state,
    },
    runtime::ModelRuntime,
};

#[derive(Debug, Clone)]
/// A model loaded in its default pose.
///
/// This type is useful for import tools, previews, tests, or renderers that only
/// need the initial mesh data. For animation and interactive parameter changes,
/// use [`RuntimeModel`] instead.
pub struct DefaultModel {
    model: Model3,
    canvas: Moc3CanvasInfo,
    meshes: Vec<Moc3DrawableMesh>,
    textures: Vec<DecodedTexture>,
}

impl DefaultModel {
    /// Returns the parsed `.model3.json` data.
    pub fn model(&self) -> &Model3 {
        &self.model
    }

    /// Returns the model canvas information parsed from the `.moc3` file.
    pub fn canvas(&self) -> Moc3CanvasInfo {
        self.canvas
    }

    /// Returns drawable meshes built from the model's default parameter values.
    pub fn meshes(&self) -> &[Moc3DrawableMesh] {
        &self.meshes
    }

    /// Returns decoded RGBA textures in the order referenced by the model file.
    pub fn textures(&self) -> &[DecodedTexture] {
        &self.textures
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// A PNG texture decoded into tightly packed RGBA8 pixels.
///
/// The pixel buffer is arranged row-major with four bytes per pixel:
/// red, green, blue, alpha.
pub struct DecodedTexture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl DecodedTexture {
    /// Creates a decoded texture from raw RGBA8 data.
    ///
    /// The constructor does not validate that `rgba.len() == width * height * 4`;
    /// callers that build textures manually should keep those values consistent.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba,
        }
    }

    /// Returns the texture width in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the texture height in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the raw RGBA8 pixel data.
    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

#[derive(Debug)]
/// Errors that can occur while loading a complete model from disk.
pub enum AssetLoadError {
    /// A referenced file could not be read.
    Io {
        /// Path of the file that failed to load.
        path: String,
        /// Original I/O error.
        source: std::io::Error,
    },
    /// A Cubism JSON file was invalid or unsupported.
    Json(crate::Error),
    /// The referenced `.moc3` file was invalid or unsupported.
    Moc3(crate::Error),
    /// A referenced texture could not be decoded.
    Image {
        /// Path of the image that failed to decode.
        path: String,
        /// Original image decoding error.
        source: image::ImageError,
    },
    /// The supplied model path has no parent directory for resolving assets.
    MissingParent {
        /// Path that did not have a parent directory.
        path: String,
    },
    /// Drawable meshes could not be built from the parsed model data.
    DrawableMeshes,
}

impl fmt::Display for AssetLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(formatter, "failed to read {path}: {source}"),
            Self::Json(error) => write!(formatter, "failed to parse model json: {error}"),
            Self::Moc3(error) => write!(formatter, "failed to parse moc3: {error}"),
            Self::Image { path, source } => write!(formatter, "failed to decode {path}: {source}"),
            Self::MissingParent { path } => write!(formatter, "path has no parent: {path}"),
            Self::DrawableMeshes => formatter.write_str("failed to build drawable meshes"),
        }
    }
}

impl std::error::Error for AssetLoadError {}

/// Loads a model as a static default-pose snapshot.
///
/// The `path` should point to a `.model3.json` file. Mocari reads the referenced
/// `.moc3` file and textures from the same model directory, then builds drawable
/// meshes using the model's default parameter values.
pub fn load_model(path: impl AsRef<Path>) -> Result<DefaultModel, AssetLoadError> {
    let parsed = parse_model(path)?;
    let mut meshes = build_moc3_drawable_meshes_for_default_pose_with_offscreen_state(
        &parsed.art_meshes,
        &parsed.art_mesh_keyforms,
        &parsed.deformers,
        &parsed.bindings,
        &parsed.ids,
        &parsed.offscreen,
    )
    .ok_or(AssetLoadError::DrawableMeshes)?;
    parsed
        .glues
        .apply(
            &mut meshes,
            &parsed.bindings,
            parsed.bindings.parameter_default_values(),
        )
        .ok_or(AssetLoadError::DrawableMeshes)?;

    Ok(DefaultModel {
        model: parsed.model,
        canvas: parsed.canvas,
        meshes,
        textures: parsed.textures,
    })
}

/// Loads a model into a mutable runtime.
///
/// This is the main entry point for interactive applications. The returned
/// [`RuntimeModel`] keeps decoded textures next to a [`ModelRuntime`], so a render
/// loop can update parameters, apply motions and expressions, call
/// [`ModelRuntime::update_meshes`], and draw the resulting meshes.
pub fn load_model_runtime(path: impl AsRef<Path>) -> Result<RuntimeModel, AssetLoadError> {
    let path = path.as_ref();
    let model_dir = path.parent().map(Path::to_path_buf);
    let parsed = parse_model(path)?;
    let runtime = ModelRuntime::new(
        parsed.model,
        parsed.canvas,
        parsed.art_meshes,
        parsed.art_mesh_keyforms,
        parsed.deformers,
        parsed.bindings,
        parsed.ids,
        parsed.offscreen,
        parsed.glues,
        parsed.parts,
        parsed.draw_order_groups,
        parsed.pose,
    )
    .ok_or(AssetLoadError::DrawableMeshes)?;

    Ok(RuntimeModel {
        runtime,
        textures: parsed.textures,
        model_dir,
    })
}

#[derive(Debug, Clone)]
/// A loaded model with mutable runtime state and decoded textures.
pub struct RuntimeModel {
    runtime: ModelRuntime,
    textures: Vec<DecodedTexture>,
    model_dir: Option<std::path::PathBuf>,
}

impl RuntimeModel {
    /// Returns the immutable runtime state.
    pub fn runtime(&self) -> &ModelRuntime {
        &self.runtime
    }

    /// Returns the mutable runtime state.
    ///
    /// Use this in a frame loop to edit parameters, apply motions or expressions,
    /// and rebuild drawable meshes.
    pub fn runtime_mut(&mut self) -> &mut ModelRuntime {
        &mut self.runtime
    }

    /// Returns decoded textures in the order used by drawable texture indices.
    pub fn textures(&self) -> &[DecodedTexture] {
        &self.textures
    }

    /// Returns the directory that contained the loaded `.model3.json` file.
    pub fn model_dir(&self) -> Option<&Path> {
        self.model_dir.as_deref()
    }
}

struct ParsedModel {
    model: Model3,
    canvas: Moc3CanvasInfo,
    art_meshes: Moc3ArtMeshes,
    art_mesh_keyforms: Moc3ArtMeshKeyforms,
    deformers: Moc3Deformers,
    bindings: Moc3KeyformBindings,
    ids: Moc3Ids,
    offscreen: Moc3OffscreenInfo,
    glues: Moc3Glues,
    parts: Moc3Parts,
    draw_order_groups: Option<Moc3DrawOrderGroups>,
    pose: Option<Pose3>,
    textures: Vec<DecodedTexture>,
}

fn parse_model(path: impl AsRef<Path>) -> Result<ParsedModel, AssetLoadError> {
    let path = path.as_ref();
    let model_source = read_text(path)?;
    let model = Model3::from_json_str(&model_source).map_err(AssetLoadError::Json)?;
    let model_dir = path.parent().ok_or_else(|| AssetLoadError::MissingParent {
        path: path.display().to_string(),
    })?;
    let moc_path = model_dir.join(model.moc());
    let moc = read_bytes(&moc_path)?;

    let art_meshes = Moc3ArtMeshes::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let art_mesh_keyforms = Moc3ArtMeshKeyforms::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let deformers = Moc3Deformers::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let bindings = Moc3KeyformBindings::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let ids = Moc3Ids::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let offscreen = Moc3OffscreenInfo::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let glues = Moc3Glues::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let parts = Moc3Parts::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let canvas = Moc3CanvasInfo::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let draw_order_groups = Moc3DrawOrderGroups::parse(&moc).map_err(AssetLoadError::Moc3)?;
    let pose = match model.pose() {
        Some(pose_file) => {
            let pose_source = read_text(&model_dir.join(pose_file))?;
            Some(Pose3::from_json_str(&pose_source).map_err(AssetLoadError::Json)?)
        }
        None => None,
    };
    let textures = model
        .textures()
        .iter()
        .map(|texture| decode_texture(model_dir.join(texture)))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(ParsedModel {
        model,
        canvas,
        art_meshes,
        art_mesh_keyforms,
        deformers,
        bindings,
        ids,
        offscreen,
        glues,
        parts,
        draw_order_groups,
        pose,
        textures,
    })
}

fn read_text(path: &Path) -> Result<String, AssetLoadError> {
    fs::read_to_string(path).map_err(|source| AssetLoadError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, AssetLoadError> {
    fs::read(path).map_err(|source| AssetLoadError::Io {
        path: path.display().to_string(),
        source,
    })
}

fn decode_texture(path: impl AsRef<Path>) -> Result<DecodedTexture, AssetLoadError> {
    let path = path.as_ref();
    let image = image::open(path).map_err(|source| AssetLoadError::Image {
        path: path.display().to_string(),
        source,
    })?;
    let rgba = image.to_rgba8();
    Ok(DecodedTexture::new(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}
