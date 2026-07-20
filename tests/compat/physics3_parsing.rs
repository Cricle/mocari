//! Physics3.json parsing and computation compatibility tests.

use mocari::json::Physics3;
use std::fs;
use std::path::Path;

fn collect_physics_files() -> Vec<(String, String)> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/models");
    let mut files = Vec::new();
    #[allow(clippy::collapsible_if)]
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let model_path = entry.path();
            if let Some(model_name) = model_path.file_name().and_then(|n| n.to_str()) {
                let physics_path = model_path.join(format!("{}.physics3.json", model_name));
                if physics_path.exists() {
                    if let Ok(content) = fs::read_to_string(&physics_path) {
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
fn all_physics3_files_parse_successfully() {
    let files = collect_physics_files();
    assert!(!files.is_empty(), "No physics3.json files found in assets/models");

    for (name, content) in files {
        let result = Physics3::from_json_str(&content);
        assert!(
            result.is_ok(),
            "Failed to parse {}.physics3.json: {:?}",
            name,
            result.err()
        );
    }
}

#[test]
fn physics3_meta_fields_are_valid() {
    for (name, content) in collect_physics_files() {
        let physics = Physics3::from_json_str(&content).unwrap();
        let meta = physics.meta();

        assert!(meta.physics_setting_count() > 0, "{}: no physics settings", name);
        // Total counts are unsigned, so >= 0 check is redundant
        assert!(meta.total_input_count() < 10000, "{}: suspiciously high input count", name);
        assert!(meta.total_output_count() < 10000, "{}: suspiciously high output count", name);
        assert!(meta.vertex_count() < 10000, "{}: suspiciously high vertex count", name);
    }
}

#[test]
fn physics3_settings_have_valid_structure() {
    for (name, content) in collect_physics_files() {
        let physics = Physics3::from_json_str(&content).unwrap();

        for (i, setting) in physics.settings().iter().enumerate() {
            assert!(!setting.id().is_empty(), "{}: setting {} has empty id", name, i);
            // Some settings may have no inputs or outputs (rare but valid)
            assert!(!setting.vertices().is_empty(), "{}: setting {} has no vertices", name, i);

            // Normalization should be valid
            let norm = setting.normalization();
            assert!(norm.position().minimum() <= norm.position().maximum(),
                    "{}: setting {} position normalization invalid", name, i);
            assert!(norm.angle().minimum() <= norm.angle().maximum(),
                    "{}: setting {} angle normalization invalid", name, i);
        }
    }
}

#[test]
fn physics3_all_parameters_referenced_exist() {
    // This test verifies that all parameter IDs referenced in physics3.json
    // would theoretically exist in a model (we can't verify without loading .moc3)
    for (name, content) in collect_physics_files() {
        let physics = Physics3::from_json_str(&content).unwrap();

        for setting in physics.settings() {
            for input in setting.inputs() {
                assert!(!input.source().id().is_empty(),
                        "{}: input has empty parameter id", name);
            }
            for output in setting.outputs() {
                assert!(!output.destination().id().is_empty(),
                        "{}: output has empty parameter id", name);
            }
        }
    }
}
