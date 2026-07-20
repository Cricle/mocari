//! Complete runtime integration tests - load, update, render cycle.

use mocari::assets::load_model_runtime;
use std::path::Path;

const TEST_MODELS: &[&str] = &["Haru", "Hiyori", "Mao", "Mark", "Natori", "Ren", "Rice", "Wanko"];

#[test]
fn all_models_load_successfully() {
    for &model_name in TEST_MODELS {
        let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);
        let result = load_model_runtime(&path);
        assert!(
            result.is_ok(),
            "{} failed to load: {:?}",
            model_name,
            result.err()
        );
    }
}

#[test]
fn loaded_models_have_valid_state() {
    for &model_name in TEST_MODELS {
        let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);
        let model = load_model_runtime(&path).unwrap();
        let runtime = model.runtime();

        assert!(!runtime.parameter_ids().is_empty(), "{}: no parameters", model_name);
        assert!(!runtime.drawable_ids().is_empty(), "{}: no drawables", model_name);
        assert!(!runtime.meshes().is_empty(), "{}: no meshes", model_name);
    }
}

#[test]
fn runtime_parameters_can_be_set() {
    for &model_name in TEST_MODELS {
        let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);
        let mut model = load_model_runtime(&path).unwrap();
        let runtime = model.runtime_mut();

        // Try to set common parameters
        let params_to_test = ["ParamAngleX", "ParamAngleY", "ParamEyeBallX", "ParamEyeBallY"];
        let mut found_any = false;

        for param in params_to_test {
            if runtime.set_parameter(param, 10.0) {
                found_any = true;
                let value = runtime.parameter_value(param).unwrap();
                // Parameter may be clamped to valid range, just verify it's finite
                assert!(value.is_finite(),
                       "{}: parameter {} not finite after set", model_name, param);
            }
        }

        // If no common parameters found, just verify we can set any parameter
        if !found_any && !runtime.parameter_ids().is_empty() {
            runtime.set_parameter_by_index(0, 5.0);
            assert!(runtime.parameter_value_by_index(0).unwrap().is_finite(),
                   "{}: first parameter not settable", model_name);
        }
    }
}

#[test]
fn runtime_meshes_update_after_parameter_change() {
    for &model_name in TEST_MODELS {
        let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);
        let mut model = load_model_runtime(&path).unwrap();
        let runtime = model.runtime_mut();

        // Record initial state
        let _initial_dirty = runtime.is_dirty();

        // Change a parameter
        runtime.set_parameter_by_index(0, 5.0);

        // Verify state changed
        assert!(runtime.is_dirty(), "{}: not dirty after parameter change", model_name);

        // Update meshes
        let result = runtime.update_meshes();
        assert!(result.is_some(), "{}: update_meshes failed", model_name);
        assert!(!runtime.is_dirty(), "{}: still dirty after update_meshes", model_name);
    }
}

#[test]
fn runtime_maintains_mesh_count() {
    for &model_name in TEST_MODELS {
        let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);
        let mut model = load_model_runtime(&path).unwrap();
        let runtime = model.runtime_mut();

        let initial_mesh_count = runtime.meshes().len();

        // Update several times
        for i in 0..10 {
            runtime.set_parameter_by_index(0, i as f32);
            runtime.update_meshes();
            assert_eq!(runtime.meshes().len(), initial_mesh_count,
                      "{}: mesh count changed at iteration {}", model_name, i);
        }
    }
}

#[test]
fn runtime_handles_normalized_parameters() {
    for &model_name in TEST_MODELS {
        let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);
        let mut model = load_model_runtime(&path).unwrap();
        let runtime = model.runtime_mut();

        // Set normalized value (0.0-1.0)
        if runtime.parameter_ids().len() > 0 {
            runtime.set_parameter_normalized_by_index(0, 0.5);
            let value = runtime.parameter_value_by_index(0).unwrap();
            assert!(value.is_finite(), "{}: normalized value not finite", model_name);
        }
    }
}

#[test]
fn runtime_hit_test_works() {
    for &model_name in TEST_MODELS {
        let path = format!("assets/models/{}/{}.model3.json", model_name, model_name);
        let model = load_model_runtime(&path).unwrap();
        let runtime = model.runtime();

        // Test canvas center - hit_test returns Option<HitAreaInfo>
        let center_hit = runtime.hit_test(0.0, 0.0);
        // Can be Some or None, just shouldn't panic
        if let Some(hit) = center_hit {
            assert!(!hit.id().is_empty(), "{}: hit has empty id", model_name);
            assert!(!hit.name().is_empty(), "{}: hit has empty name", model_name);
        }
    }
}
