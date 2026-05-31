use serde::Deserialize;

use crate::{Error, Result};

const FORMAT: &str = "exp3.json";

#[derive(Debug, Clone, PartialEq)]
pub struct Expression3 {
    kind: String,
    fade_in_time: Option<f32>,
    fade_out_time: Option<f32>,
    parameters: Vec<ExpressionParameter>,
}

impl Expression3 {
    pub fn from_json_str(source: &str) -> Result<Self> {
        let raw: RawExpression3 =
            serde_json::from_str(source).map_err(|error| Error::InvalidJson {
                format: FORMAT,
                message: error.to_string(),
            })?;

        Ok(Self {
            kind: raw.kind,
            fade_in_time: raw.fade_in_time,
            fade_out_time: raw.fade_out_time,
            parameters: raw.parameters,
        })
    }

    pub fn kind(&self) -> &str {
        &self.kind
    }

    pub fn fade_in_time(&self) -> Option<f32> {
        self.fade_in_time
    }

    pub fn fade_out_time(&self) -> Option<f32> {
        self.fade_out_time
    }

    pub fn parameters(&self) -> &[ExpressionParameter] {
        &self.parameters
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct RawExpression3 {
    #[serde(rename = "Type")]
    kind: String,
    #[serde(rename = "FadeInTime", default)]
    fade_in_time: Option<f32>,
    #[serde(rename = "FadeOutTime", default)]
    fade_out_time: Option<f32>,
    #[serde(rename = "Parameters", default)]
    parameters: Vec<ExpressionParameter>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ExpressionParameter {
    #[serde(rename = "Id")]
    id: String,
    #[serde(rename = "Value")]
    value: f32,
    #[serde(rename = "Blend", default)]
    blend: ExpressionBlend,
}

impl ExpressionParameter {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn value(&self) -> f32 {
        self.value
    }

    pub fn blend(&self) -> ExpressionBlend {
        self.blend
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Deserialize, Default)]
pub enum ExpressionBlend {
    #[serde(rename = "Add")]
    Add,
    #[serde(rename = "Multiply")]
    Multiply,
    #[default]
    #[serde(rename = "Overwrite")]
    Overwrite,
}
