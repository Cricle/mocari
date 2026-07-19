use super::rigger::RiggedModel;

/// Configuration for generated model3.json file paths.
#[derive(Clone)]
pub struct ModelJsonConfig {
    /// Path to the `.moc3` file. Default: `"model.moc3"`.
    pub moc_file: String,
    /// Base directory for texture paths. Default: `""` (current dir).
    pub texture_dir: String,
    /// Base directory for motion paths. Default: `""`.
    pub motion_dir: String,
    /// Base directory for expression paths. Default: `""`.
    pub expression_dir: String,
}

impl Default for ModelJsonConfig {
    fn default() -> Self {
        Self {
            moc_file: "model.moc3".into(),
            texture_dir: String::new(),
            motion_dir: String::new(),
            expression_dir: String::new(),
        }
    }
}

/// Generates a `model3.json` string for a rigged model.
///
/// The generated JSON includes file references for the moc3 binary, textures,
/// motions, and expressions. File paths are constructed from `config` defaults
/// unless overridden.
pub fn generate_model_json(model: &RiggedModel, config: &ModelJsonConfig) -> String {
    let mut json = String::from("{\n  \"Version\": 3,\n  \"FileReferences\": {\n");

    // Moc file.
    json.push_str(&format!("    \"Moc\": \"{}\",\n", escape_json(&config.moc_file)));

    // Textures.
    json.push_str("    \"Textures\": [\n");
    for i in 0..model.textures.len() {
        let path = if config.texture_dir.is_empty() {
            format!("texture_{i}.png")
        } else {
            format!("{}/texture_{}.png", config.texture_dir, i)
        };
        let comma = if i < model.textures.len() - 1 { "," } else { "" };
        json.push_str(&format!("      \"{}\"{}\n", escape_json(&path), comma));
    }
    json.push_str("    ]");

    // Motions.
    if !model.motions.is_empty() {
        json.push_str(",\n    \"Motions\": {\n");
        json.push_str("      \"Idle\": [\n");
        for (i, (name, _)) in model.motions.iter().enumerate() {
            let path = if config.motion_dir.is_empty() {
                format!("{name}.motion3.json")
            } else {
                format!("{}/{name}.motion3.json", config.motion_dir)
            };
            let comma = if i < model.motions.len() - 1 { "," } else { "" };
            json.push_str(&format!(
                "        {{ \"File\": \"{}\" }}{}\n",
                escape_json(&path),
                comma
            ));
        }
        json.push_str("      ]\n    }");
    }

    // Expressions.
    if !model.expressions.is_empty() {
        json.push_str(",\n    \"Expressions\": [\n");
        for (i, (name, _)) in model.expressions.iter().enumerate() {
            let path = if config.expression_dir.is_empty() {
                format!("{name}.exp3.json")
            } else {
                format!("{}/{name}.exp3.json", config.expression_dir)
            };
            let comma = if i < model.expressions.len() - 1 { "," } else { "" };
            json.push_str(&format!(
                "      {{ \"Name\": \"{}\", \"File\": \"{}\" }}{}\n",
                escape_json(name),
                escape_json(&path),
                comma
            ));
        }
        json.push_str("    ]");
    }

    json.push_str("\n  }\n}\n");
    json
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_valid_json_with_single_mesh() {
        let model = RiggedModel {
            textures: vec![vec![0u8; 4]],
            meshes: vec![super::super::rigger::RiggedMesh {
                texture_index: 0,
                vertices: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
                indices: vec![0, 1, 2],
                opacity: 1.0,
            }],
            parameters: vec![],
            deformers: vec![],
            physics: None,
            motions: vec![],
            expressions: vec![],
        };
        let json = generate_model_json(&model, &ModelJsonConfig::default());
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["Version"], 3);
        assert_eq!(parsed["FileReferences"]["Moc"], "model.moc3");
        assert_eq!(parsed["FileReferences"]["Textures"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn includes_motions_and_expressions() {
        let model = RiggedModel {
            textures: vec![],
            meshes: vec![],
            parameters: vec![],
            deformers: vec![],
            physics: None,
            motions: vec![("idle".into(), vec![])],
            expressions: vec![("happy".into(), vec![])],
        };
        let json = generate_model_json(&model, &ModelJsonConfig::default());
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed["FileReferences"]["Motions"].is_object());
        assert!(parsed["FileReferences"]["Expressions"].is_array());
    }
}
