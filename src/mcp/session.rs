use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};

use crate::assets::{self, RuntimeModel};
use crate::auto::{Breath, EyeBlink, LipSync, MouseTracker};
use crate::expression::ExpressionManager;
use crate::motion::MotionManager;

/// A loaded Live2D model with its runtime state and auto-animation systems.
#[derive(Debug, Clone)]
pub struct LoadedModel {
    /// The loaded model with runtime state and textures.
    pub model: RuntimeModel,
    /// Motion playback manager.
    pub motion_manager: MotionManager,
    /// Expression playback manager.
    pub expression_manager: ExpressionManager,
    /// Automatic eye blinking, if the model has EyeBlink groups.
    pub eye_blink: Option<EyeBlink>,
    /// Automatic lip sync driven by external audio amplitude.
    pub lip_sync: Option<LipSync>,
    /// Subtle breathing animation.
    pub breath: Option<Breath>,
    /// Cursor/face tracking.
    pub mouse_tracker: Option<MouseTracker>,
    /// Directory containing the `.model3.json` file.
    pub base_path: PathBuf,
}

/// Errors returned by [`ModelSession`] operations.
#[derive(Debug, Clone)]
pub enum SessionError {
    /// No model with the given ID exists in the session.
    ModelNotFound(String),
    /// The requested file path does not exist on disk.
    FileNotFound(String),
    /// An error occurred while loading the model.
    LoadError(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelNotFound(id) => write!(f, "model not found: {id}"),
            Self::FileNotFound(path) => write!(f, "file not found: {path}"),
            Self::LoadError(msg) => write!(f, "load error: {msg}"),
        }
    }
}

impl std::error::Error for SessionError {}

/// Manages a collection of loaded Live2D models.
///
/// Each model is assigned a string ID (`model_1`, `model_2`, ...) on load.
/// Use [`with_model`](Self::with_model) / [`with_model_mut`](Self::with_model_mut)
/// to access a model's state by ID.
#[derive(Debug, Clone)]
pub struct ModelSession {
    /// Map from model ID to loaded model. Public so MCP tool handlers can access directly.
    pub models: HashMap<String, LoadedModel>,
    next_id: u64,
}

impl ModelSession {
    /// Creates an empty session with no loaded models.
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            next_id: 0,
        }
    }

    /// Loads a Live2D model from a `.model3.json` file path.
    ///
    /// Returns the assigned model ID (e.g. `"model_1"`).
    pub fn load_model(&mut self, path: &str) -> Result<String, SessionError> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(SessionError::FileNotFound(path.display().to_string()));
        }

        let rt_model = assets::load_model_runtime(path)
            .map_err(|e| SessionError::LoadError(e.to_string()))?;

        // Build auto-animation configs from model data
        let eye_blink = Some(EyeBlink::new(
            rt_model.runtime().eye_blink_config_from_model(),
        ));
        let lip_sync = Some(LipSync::new(
            rt_model.runtime().lip_sync_config_from_model(),
        ));

        self.next_id += 1;
        let id = format!("model_{}", self.next_id);
        let base_path = path.parent().unwrap_or(Path::new(".")).to_path_buf();

        self.models.insert(
            id.clone(),
            LoadedModel {
                model: rt_model,
                motion_manager: MotionManager::new(),
                expression_manager: ExpressionManager::new(),
                eye_blink,
                lip_sync,
                breath: Some(Breath::new(Default::default())),
                mouse_tracker: None,
                base_path,
            },
        );

        Ok(id)
    }

    /// Removes a model from the session. Returns `true` if the model existed.
    pub fn unload_model(&mut self, id: &str) -> bool {
        self.models.remove(id).is_some()
    }

    /// Lists all loaded models as `(id, base_path)` tuples.
    pub fn list_models(&self) -> Vec<(&str, &Path)> {
        self.models
            .iter()
            .map(|(id, m)| (id.as_str(), m.base_path.as_path()))
            .collect()
    }

    /// Calls `f` with an immutable reference to the model identified by `id`.
    pub fn with_model<R>(
        &self,
        id: &str,
        f: impl FnOnce(&LoadedModel) -> R,
    ) -> Result<R, SessionError> {
        self.models
            .get(id)
            .map(f)
            .ok_or_else(|| SessionError::ModelNotFound(id.to_string()))
    }

    /// Calls `f` with a mutable reference to the model identified by `id`.
    pub fn with_model_mut<R>(
        &mut self,
        id: &str,
        f: impl FnOnce(&mut LoadedModel) -> R,
    ) -> Result<R, SessionError> {
        self.models
            .get_mut(id)
            .map(f)
            .ok_or_else(|| SessionError::ModelNotFound(id.to_string()))
    }
}
