use serde::{Deserialize, Serialize};

/// Configuration for the Live2D automation pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Face detection settings
    pub face_detection: FaceDetectionConfig,
    /// Mesh generation settings
    pub mesh: MeshConfig,
    /// Motion generation settings
    pub motion: MotionConfig,
    /// Export settings
    pub export: ExportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceDetectionConfig {
    /// Minimum face size ratio (relative to image size)
    pub min_face_ratio: f32,
    /// Maximum face size ratio
    pub max_face_ratio: f32,
    /// Skin color detection thresholds
    pub skin_r_min: u8,
    pub skin_g_min: u8,
    pub skin_b_min: u8,
    /// Canny edge detection thresholds
    pub canny_low: f32,
    pub canny_high: f32,
    /// Adaptive threshold block size
    pub adaptive_block_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshConfig {
    /// Default mesh density (vertices per pixel)
    pub default_density: f32,
    /// Eye mesh density
    pub eye_density: f32,
    /// Mouth mesh density
    pub mouth_density: f32,
    /// Minimum triangle area
    pub min_triangle_area: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionConfig {
    /// Motion FPS
    pub fps: f32,
    /// Enable breathing motion
    pub breathing: bool,
    /// Enable blink motion
    pub blink: bool,
    /// Enable sway motion
    pub sway: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportConfig {
    /// Export texture format (png, webp)
    pub texture_format: TextureFormat,
    /// Texture quality (0-100)
    pub texture_quality: u8,
    /// Export physics
    pub export_physics: bool,
    /// Export deformers
    pub export_deformers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TextureFormat {
    Png,
    WebP,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            face_detection: FaceDetectionConfig {
                min_face_ratio: 0.05,
                max_face_ratio: 0.6,
                skin_r_min: 95,
                skin_g_min: 40,
                skin_b_min: 20,
                canny_low: 50.0,
                canny_high: 100.0,
                adaptive_block_size: 15,
            },
            mesh: MeshConfig {
                default_density: 0.02,
                eye_density: 0.03,
                mouth_density: 0.025,
                min_triangle_area: 100.0,
            },
            motion: MotionConfig {
                fps: 30.0,
                breathing: true,
                blink: true,
                sway: true,
            },
            export: ExportConfig {
                texture_format: TextureFormat::Png,
                texture_quality: 90,
                export_physics: true,
                export_deformers: true,
            },
        }
    }
}

impl PipelineConfig {
    /// Load configuration from file.
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file.
    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
