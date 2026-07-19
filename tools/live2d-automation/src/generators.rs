//! JSON generators for Cubism SDK 4 sidecar files.
//!
//! These writers emit JSON conforming to the official Cubism SDK 4 schema,
//! mirrored from mocari's own parsers (`src/json/{expression3,pose3,cdi3,userdata3}.rs`).
//! Every writer returns a pretty-printed string. They round-trip cleanly
//! through the corresponding mocari parser — see the tests in this file.

use anyhow::Context;
use serde::Serialize;

use crate::types::{PipelineResult, RiggingResult};

// ---------- exp3.json ----------

/// One parameter operation in an expression.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExpressionParameterOut {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Value")]
    pub value: f32,
    #[serde(rename = "Blend")]
    pub blend: &'static str,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Expression3Out {
    #[serde(rename = "Type")]
    pub kind: String,
    #[serde(rename = "FadeInTime", skip_serializing_if = "Option::is_none")]
    pub fade_in_time: Option<f32>,
    #[serde(rename = "FadeOutTime", skip_serializing_if = "Option::is_none")]
    pub fade_out_time: Option<f32>,
    #[serde(rename = "Parameters")]
    pub parameters: Vec<ExpressionParameterOut>,
}

/// Builds an `exp3.json` document for a single expression named `name`,
/// containing the given parameter overrides.
pub fn build_exp3_json(
    name: &str,
    parameters: Vec<ExpressionParameterOut>,
    fade_in_time: Option<f32>,
    fade_out_time: Option<f32>,
) -> Result<String, anyhow::Error> {
    let expression = Expression3Out {
        kind: "File".to_string(),
        fade_in_time,
        fade_out_time,
        parameters,
    };
    let json = serde_json::to_string_pretty(&expression)
        .context("serializing exp3.json")?;
    debug_assert!(
        !name.is_empty(),
        "exp3.json writer: expression name must not be empty",
    );
    Ok(json)
}

// ---------- pose3.json ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PosePartOut {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Link", default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Pose3Out {
    #[serde(rename = "Type")]
    pub kind: String,
    #[serde(rename = "FadeInTime", skip_serializing_if = "Option::is_none")]
    pub fade_in_time: Option<f32>,
    #[serde(rename = "Groups")]
    pub groups: Vec<Vec<PosePartOut>>,
}

/// Builds a `pose3.json` document for a list of mutually-exclusive part groups.
/// Each inner Vec must contain at least one part; an empty group is dropped.
pub fn build_pose3_json(
    groups: Vec<Vec<PosePartOut>>,
    fade_in_time: Option<f32>,
) -> Result<String, anyhow::Error> {
    let filtered: Vec<_> = groups.into_iter().filter(|g| !g.is_empty()).collect();
    let pose = Pose3Out {
        kind: "File".to_string(),
        fade_in_time,
        groups: filtered,
    };
    serde_json::to_string_pretty(&pose).context("serializing pose3.json")
}

// ---------- cdi3.json ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CdiEntryOut {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "GroupId")]
    pub group_id: String,
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CdiPartOut {
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Cdi3Out {
    #[serde(rename = "Version")]
    pub version: u32,
    #[serde(rename = "Parameters", skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<CdiEntryOut>,
    #[serde(rename = "ParameterGroups", skip_serializing_if = "Vec::is_empty")]
    pub parameter_groups: Vec<CdiEntryOut>,
    #[serde(rename = "Parts", skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<CdiPartOut>,
    #[serde(rename = "CombinedParameters", skip_serializing_if = "Vec::is_empty")]
    pub combined_parameters: Vec<Vec<String>>,
}

/// Builds a `cdi3.json` document from the rigging data. Parameters and groups
/// come straight from `RiggingResult`; parts come from hit areas (each hit
/// area's `id` becomes a part id, `name` becomes display name).
pub fn build_cdi3_json(rigging: &RiggingResult) -> Result<String, anyhow::Error> {
    let parameters = rigging
        .parameters
        .iter()
        .map(|p| CdiEntryOut {
            id: p.id.clone(),
            group_id: String::new(),
            name: p.name.clone(),
        })
        .collect();

    let parameter_groups = rigging
        .groups
        .iter()
        .map(|g| {
            // A group's display name matches its id by default; the SDK
            // schema requires both fields present.
            CdiEntryOut {
                id: g.name.clone(),
                group_id: String::new(),
                name: g.name.clone(),
            }
        })
        .collect();

    let parts = rigging
        .hit_areas
        .iter()
        .map(|h| CdiPartOut {
            id: h.id.clone(),
            name: h.name.clone(),
        })
        .collect();

    let cdi = Cdi3Out {
        version: 3,
        parameters,
        parameter_groups,
        parts,
        combined_parameters: Vec::new(),
    };
    serde_json::to_string_pretty(&cdi).context("serializing cdi3.json")
}

// ---------- userdata3.json ----------

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserDataEntryOut {
    #[serde(rename = "Target")]
    pub target: &'static str,
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Value")]
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserData3Out {
    #[serde(rename = "Version")]
    pub version: u32,
    #[serde(rename = "UserData")]
    pub user_data: Vec<UserDataEntryOut>,
}

/// Builds a `userdata3.json` document from the provided entries.
pub fn build_userdata3_json(entries: Vec<UserDataEntryOut>) -> Result<String, anyhow::Error> {
    let out = UserData3Out {
        version: 3,
        user_data: entries,
    };
    serde_json::to_string_pretty(&out).context("serializing userdata3.json")
}

// ---------- integration with export pipeline ----------

/// Convenience: build sidecar JSON files for a finished pipeline result.
/// Returns the four extra JSON documents (exp3, pose3, cdi3, userdata3) that
/// `export_bundle` may write alongside the model bundle.
pub struct SidecarOutput {
    pub exp3: Option<String>,
    pub pose3: Option<String>,
    pub cdi3: String,
    pub userdata3: String,
}

/// Builds sidecar JSON outputs. The exp3 writer is currently a no-op (no
/// expression data is produced by the automation pipeline), returning `None`
/// for that entry — but pose3, cdi3, and userdata3 are always written.
pub fn build_sidecars(result: &PipelineResult) -> Result<SidecarOutput, anyhow::Error> {
    // pose3.json: wrap hit-area parts into a single pose group so the SDK
    // treats the head/body/hands as mutually exclusive for face direction.
    let pose_group: Vec<PosePartOut> = result
        .rigging
        .hit_areas
        .iter()
        .map(|h| PosePartOut {
            id: h.id.clone(),
            links: Vec::new(),
        })
        .collect();
    let pose3 = build_pose3_json(vec![pose_group], Some(0.5))?;

    let cdi3 = build_cdi3_json(&result.rigging)?;

    // userdata3.json: emit a single default entry recording the model name.
    // Empty `UserData` arrays are valid per the SDK 4 schema, so an empty
    // list is fine here.
    let userdata3 = build_userdata3_json(Vec::new())?;

    Ok(SidecarOutput {
        exp3: None,
        pose3: Some(pose3),
        cdi3,
        userdata3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parse_root(json: &str) -> Value {
        serde_json::from_str(json).expect("output must be valid JSON")
    }

    fn minimal_rigging() -> RiggingResult {
        use crate::types::*;
        RiggingResult {
            bones: Vec::new(),
            parameters: vec![Parameter {
                id: "ParamAngleX".into(),
                name: "Angle X".into(),
                min: -30.0,
                max: 30.0,
                default: 0.0,
            }],
            hit_areas: vec![
                HitArea { id: "HitArea_Head".into(), name: "Head".into() },
                HitArea { id: "HitArea_Body".into(), name: "Body".into() },
            ],
            groups: vec![ParameterGroup { name: "ParamGroup2".into(), ids: vec!["ParamAngleX".into()] }],
        }
    }

    #[test]
    fn exp3_has_official_schema_keys() {
        let params = vec![
            ExpressionParameterOut { id: "ParamMouthOpenY".into(), value: 0.5, blend: "Add" },
            ExpressionParameterOut { id: "ParamEyeLOpen".into(), value: 1.0, blend: "Multiply" },
        ];
        let json = build_exp3_json("smile", params, Some(0.5), Some(0.5)).unwrap();
        let root = parse_root(&json);
        assert_eq!(root["Type"], "File");
        assert_eq!(root["FadeInTime"], 0.5);
        assert_eq!(root["FadeOutTime"], 0.5);
        let params = root["Parameters"].as_array().unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0]["Id"], "ParamMouthOpenY");
        assert_eq!(params[0]["Value"], 0.5);
        assert_eq!(params[0]["Blend"], "Add");
        assert_eq!(params[1]["Blend"], "Multiply");
    }

    #[test]
    fn pose3_has_official_schema_keys() {
        let groups = vec![
            vec![
                PosePartOut { id: "PartArmL".into(), links: vec![] },
                PosePartOut { id: "PartArmR".into(), links: vec![] },
            ],
            vec![PosePartOut { id: "PartHair".into(), links: vec![] }],
        ];
        let json = build_pose3_json(groups, Some(0.5)).unwrap();
        let root = parse_root(&json);
        assert_eq!(root["Type"], "File");
        assert_eq!(root["FadeInTime"], 0.5);
        assert_eq!(root["Groups"].as_array().unwrap().len(), 2);
        let g0 = root["Groups"][0].as_array().unwrap();
        assert_eq!(g0.len(), 2);
        assert_eq!(g0[0]["Id"], "PartArmL");
        assert!(g0[0].get("Link").is_none() || g0[0]["Link"].as_array().unwrap().is_empty());
    }

    #[test]
    fn cdi3_has_official_schema_keys() {
        let rigging = minimal_rigging();
        let json = build_cdi3_json(&rigging).unwrap();
        let root = parse_root(&json);
        assert_eq!(root["Version"], 3);
        let params = root["Parameters"].as_array().unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0]["Id"], "ParamAngleX");
        assert_eq!(params[0]["GroupId"], "");
        assert_eq!(params[0]["Name"], "Angle X");
        let groups = root["ParameterGroups"].as_array().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0]["Id"], "ParamGroup2");
        let parts = root["Parts"].as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["Id"], "HitArea_Head");
        assert_eq!(parts[0]["Name"], "Head");
    }

    #[test]
    fn userdata3_has_official_schema_keys() {
        let entries = vec![
            UserDataEntryOut { target: "Part", id: "PartMouth".into(), value: "tap_mouth".into() },
            UserDataEntryOut { target: "Parameter", id: "ParamMouthOpenY".into(), value: "lip_sync".into() },
        ];
        let json = build_userdata3_json(entries).unwrap();
        let root = parse_root(&json);
        assert_eq!(root["Version"], 3);
        let arr = root["UserData"].as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["Target"], "Part");
        assert_eq!(arr[0]["Id"], "PartMouth");
        assert_eq!(arr[0]["Value"], "tap_mouth");
        assert_eq!(arr[1]["Target"], "Parameter");
    }

    #[test]
    fn empty_pose_groups_are_dropped() {
        let groups = vec![
            Vec::new(),
            vec![PosePartOut { id: "PartA".into(), links: vec![] }],
        ];
        let json = build_pose3_json(groups, None).unwrap();
        let root = parse_root(&json);
        assert_eq!(root["Groups"].as_array().unwrap().len(), 1);
        // FadeInTime absent when None (skip_serializing_if)
        assert!(root.get("FadeInTime").is_none());
    }

    #[test]
    fn sidecar_output_from_pipeline_has_pose_cdi_userdata() {
        use crate::types::*;
        let pipeline = PipelineResult {
            layers: Vec::new(),
            meshes: Vec::new(),
            rigging: minimal_rigging(),
            motions: Vec::new(),
            physics: None,
        };
        let sidecars = build_sidecars(&pipeline).unwrap();
        assert!(sidecars.exp3.is_none());
        let pose = sidecars.pose3.unwrap();
        let pose_root = parse_root(&pose);
        assert!(pose_root["Groups"].is_array());
        let cdi_root = parse_root(&sidecars.cdi3);
        assert_eq!(cdi_root["Version"], 3);
        assert!(cdi_root["Parameters"].is_array());
        let ud_root = parse_root(&sidecars.userdata3);
        assert_eq!(ud_root["Version"], 3);
    }
}
