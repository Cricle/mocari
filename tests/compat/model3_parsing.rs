//! Model3.json parsing and structure validation tests.

use mocari::json::Model3;
use std::fs;
use std::path::Path;

fn collect_model3_files() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models");
    let mut files = Vec::new();
    #[allow(clippy::collapsible_if)]
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let model_path = entry.path();
            if let Some(model_name) = model_path.file_name().and_then(|n| n.to_str()) {
                let json_path = model_path.join(format!("{}.model3.json", model_name));
                if json_path.exists() {
                    if let Ok(content) = fs::read_to_string(&json_path) {
                        files.push((model_name.to_string(), content));
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

#[test]
fn all_model3_files_parse_successfully() {
    let files = collect_model3_files();
    assert!(!files.is_empty(), "No model3.json files found");

    for (name, content) in files {
        let result = Model3::from_json_str(&content);
        assert!(
            result.is_ok(),
            "Failed to parse {}.model3.json: {:?}",
            name,
            result.err()
        );
    }
}

#[test]
fn model3_version_is_valid() {
    for (name, content) in collect_model3_files() {
        let model = Model3::from_json_str(&content).unwrap();
        let version = model.version();
        assert!(version >= 3, "{}: version {} < 3", name, version);
        assert!(version <= 5, "{}: version {} > 5 (future version?)", name, version);
    }
}

#[test]
fn model3_file_references_are_valid() {
    for (name, content) in collect_model3_files() {
        let model = Model3::from_json_str(&content).unwrap();

        // Moc3 file must be specified
        assert!(!model.moc().is_empty(), "{}: moc file not specified", name);
        assert!(model.moc().ends_with(".moc3"), "{}: moc file doesn't end with .moc3", name);

        // Textures must be specified
        assert!(!model.textures().is_empty(), "{}: no textures", name);
        for (i, tex) in model.textures().iter().enumerate() {
            assert!(tex.ends_with(".png"), "{}: texture {} is not PNG", name, i);
        }

        // Optional physics file
        if let Some(physics) = model.physics() {
            assert!(physics.ends_with(".physics3.json"), "{}: physics file wrong format", name);
        }

        // Optional pose file
        if let Some(pose) = model.pose() {
            assert!(pose.ends_with(".pose3.json"), "{}: pose file wrong format", name);
        }
    }
}

#[test]
fn model3_groups_are_valid() {
    for (name, content) in collect_model3_files() {
        let model = Model3::from_json_str(&content).unwrap();

        let groups = model.groups();
        if !groups.is_empty() {
            for group in groups {
                assert!(!group.name().is_empty(), "{}: group has empty name", name);
                // Some groups can be empty (like LipSync placeholders)
                // Just verify the structure is valid
            }
        }
    }
}

#[test]
fn model3_hit_areas_are_valid() {
    for (name, content) in collect_model3_files() {
        let model = Model3::from_json_str(&content).unwrap();

        let hit_areas = model.hit_areas();
        if !hit_areas.is_empty() {
            for area in hit_areas {
                assert!(!area.id().is_empty(), "{}: hit area has empty id", name);
                assert!(!area.name().is_empty(), "{}: hit area has empty name", name);
            }
        }
    }
}
