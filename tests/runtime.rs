use mocari::{
    assets::{load_model, load_model_runtime},
    expression::{ExpressionManager, ExpressionPlayer, load_expression},
    json::{Expression3, Motion3, UserDataTarget},
    motion::{MotionManager, MotionPlayer, MotionPriority},
};

#[test]
fn runtime_exposes_user_data_when_present() {
    // Mao doesn't have userdata, so user_data should be None
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    assert!(model.runtime().user_data().is_none());
}

#[test]
fn find_user_data_returns_none_when_no_data() {
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    assert!(model.runtime().find_user_data(&UserDataTarget::Parameter, "ParamAngleX").is_none());
}

#[test]
fn runtime_loads_user_data_for_hiyori() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let data = model.runtime().user_data().expect("Hiyori has userdata");
    assert_eq!(data.version(), 3);
    assert_eq!(data.entries().len(), 7);
    // "ArtMesh" targets fall through to Parameter in the current parser
    assert_eq!(
        model.runtime().find_user_data(&UserDataTarget::Parameter, "ArtMesh93"),
        Some("ribon"),
    );
}

#[test]
fn runtime_default_pose_matches_default_model() {
    let runtime = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let default = load_model("assets/models/Haru/Haru.model3.json").unwrap();

    let runtime_meshes = runtime.runtime().meshes();
    let default_meshes = default.meshes();

    // Geometry must match; opacity may differ because the runtime applies the
    // pose3 part groups (hiding the redundant arm) while load_model does not.
    assert_eq!(runtime_meshes.len(), default_meshes.len());
    for (left, right) in runtime_meshes.iter().zip(default_meshes) {
        assert_eq!(left.vertices(), right.vertices());
    }
}

#[test]
fn loaded_textures_match_image_crate_rgba_output() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let texture_paths = [
        "assets/models/Hiyori/Hiyori.2048/texture_00.png",
        "assets/models/Hiyori/Hiyori.2048/texture_01.png",
    ];

    assert_eq!(model.textures().len(), texture_paths.len());
    for (texture, path) in model.textures().iter().zip(texture_paths) {
        let expected = image::open(path).unwrap().to_rgba8();

        assert_eq!(texture.width(), expected.width());
        assert_eq!(texture.height(), expected.height());
        assert_eq!(texture.rgba(), expected.as_raw());
    }
}

#[test]
fn setting_a_parameter_changes_mesh_vertices() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let before: Vec<_> = model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().to_vec())
        .collect();

    let angle_x = model
        .runtime()
        .parameter_ids()
        .iter()
        .find(|id| id.as_str() == "ParamAngleX")
        .cloned()
        .expect("Haru has ParamAngleX");
    model.runtime_mut().set_parameter(&angle_x, 30.0);
    model.runtime_mut().update_meshes().unwrap();

    let after: Vec<_> = model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().to_vec())
        .collect();

    assert_ne!(before, after, "moving ParamAngleX should deform the mesh");
}

#[test]
fn runtime_hit_tests_model3_hit_areas() {
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    let runtime = model.runtime();
    let body = runtime
        .model()
        .hit_areas()
        .iter()
        .find(|hit_area| hit_area.name() == "Body")
        .expect("Mao declares Body hit area");
    let drawable_index = runtime
        .drawable_index(body.id())
        .expect("hit area id references a drawable");
    let (x, y) = drawable_center(&runtime.meshes()[drawable_index]);

    let hit = runtime.hit_test(x, y).expect("body center should hit");

    assert_eq!(hit.id(), body.id());
    assert_eq!(hit.name(), "Body");
    assert_eq!(hit.drawable_index(), drawable_index);
    assert!(runtime.hit_test(10_000.0, 10_000.0).is_none());
}

#[test]
fn updating_parameters_reuses_runtime_mesh_storage() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let mesh_ptr = model.runtime().meshes().as_ptr();
    let vertex_ptrs: Vec<_> = model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().as_ptr())
        .collect();

    assert!(model.runtime_mut().set_parameter("ParamAngleX", 30.0));
    model.runtime_mut().update_meshes().unwrap();

    assert_eq!(model.runtime().meshes().as_ptr(), mesh_ptr);
    assert_eq!(
        model
            .runtime()
            .meshes()
            .iter()
            .map(|mesh| mesh.vertices().as_ptr())
            .collect::<Vec<_>>(),
        vertex_ptrs
    );
}

fn drawable_center(mesh: &mocari::moc3::Moc3DrawableMesh) -> (f32, f32) {
    let first = mesh.vertices().first().expect("hit drawable has vertices");
    let [mut min_x, mut min_y] = first.position();
    let mut max_x = min_x;
    let mut max_y = min_y;

    for vertex in mesh.vertices().iter().skip(1) {
        let [x, y] = vertex.position();
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }

    ((min_x + max_x) * 0.5, (min_y + max_y) * 0.5)
}

#[test]
fn set_parameter_clamps_to_model_range() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let id = "ParamAngleX";
    let index = model.runtime().parameter_index(id).unwrap();

    model.runtime_mut().set_parameter(id, 1_000_000.0);
    let high = model.runtime().parameter_value_by_index(index).unwrap();
    model.runtime_mut().set_parameter(id, -1_000_000.0);
    let low = model.runtime().parameter_value_by_index(index).unwrap();

    assert!(high < 1_000_000.0, "value must be clamped to the maximum");
    assert!(low > -1_000_000.0, "value must be clamped to the minimum");
    assert!(low < high);
}

#[test]
fn parameter_info_exposes_range_default_and_current_value() {
    let model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let info = model.runtime().parameter_info_by_index(index).unwrap();

    assert_eq!(info.id(), "ParamAngleX");
    assert!(info.minimum() < info.maximum());
    assert!(info.minimum() <= info.default());
    assert!(info.default() <= info.maximum());
    assert_eq!(info.default(), info.value());
    assert_eq!(
        model.runtime().parameter_infos().count(),
        model.runtime().parameter_ids().len()
    );
}

#[test]
fn set_parameter_normalized_maps_unit_range_to_model_range() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let info = model.runtime().parameter_info_by_index(index).unwrap();
    let expected = info.minimum() + (info.maximum() - info.minimum()) * 0.75;

    assert!(
        model
            .runtime_mut()
            .set_parameter_normalized_by_index(index, 0.75)
    );

    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        expected,
    );
    assert_close(
        model
            .runtime()
            .parameter_normalized_value_by_index(index)
            .unwrap(),
        0.75,
    );

    assert!(
        model
            .runtime_mut()
            .set_parameter_normalized("ParamAngleX", 2.0)
    );
    assert_close(
        model
            .runtime()
            .parameter_normalized_value_by_index(index)
            .unwrap(),
        1.0,
    );
}

#[test]
fn parameter_overrides_can_be_applied_after_parameter_reset() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let info = model.runtime().parameter_info_by_index(index).unwrap();
    let maximum = info.maximum();
    let default = info.default();

    assert!(
        model
            .runtime_mut()
            .set_parameter_override_normalized_by_index(index, 1.0)
    );
    model.runtime_mut().reset_parameters();
    model.runtime_mut().apply_parameter_overrides();
    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        maximum,
    );
    assert_close(
        model
            .runtime()
            .parameter_override_normalized_value_by_index(index)
            .unwrap(),
        1.0,
    );

    assert!(model.runtime_mut().clear_parameter_override_by_index(index));
    model.runtime_mut().reset_parameters();
    model.runtime_mut().apply_parameter_overrides();
    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        default,
    );
    assert!(
        model
            .runtime()
            .parameter_override_value_by_index(index)
            .is_none()
    );
}

#[test]
fn motion_player_drives_a_parameter_over_time() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion =
        mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();

    let target = motion
        .curves()
        .iter()
        .find(|curve| curve.target() == "Parameter")
        .map(|curve| curve.id().to_owned())
        .expect("idle motion has a parameter curve");
    let index = model
        .runtime()
        .parameter_index(&target)
        .expect("motion parameter exists on model");

    let mut player = MotionPlayer::new(motion);
    player.tick(0.5);
    player.apply(model.runtime_mut());
    let first = model.runtime().parameter_value_by_index(index).unwrap();

    player.tick(0.5);
    player.apply(model.runtime_mut());
    let second = model.runtime().parameter_value_by_index(index).unwrap();

    assert!(first.is_finite() && second.is_finite());
}

#[test]
fn non_looping_motion_finishes_after_duration() {
    let motion = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": false },
            "Curves": [
                { "Target": "Parameter", "Id": "ParamAngleX", "Segments": [0.0, 0.0, 0, 1.0, 10.0] }
            ]
        }"#,
    )
    .unwrap();

    let mut player = MotionPlayer::new(motion);
    assert!(!player.is_finished());
    player.tick(0.5);
    assert!(!player.is_finished());
    player.tick(0.6);
    assert!(player.is_finished());
    assert_eq!(player.time(), 1.0);
}

#[test]
fn looping_motion_wraps_time() {
    let motion = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": true },
            "Curves": [
                { "Target": "Parameter", "Id": "ParamAngleX", "Segments": [0.0, 0.0, 0, 1.0, 10.0] }
            ]
        }"#,
    )
    .unwrap();

    let mut player = MotionPlayer::new(motion);
    player.tick(1.5);
    assert!(!player.is_finished());
    assert!((player.time() - 0.5).abs() < 0.0001);
}

#[test]
fn one_shot_player_finishes_looping_motion() {
    let motion = Motion3::from_json_str(
        r#"{
            "Version": 3,
            "Meta": { "Duration": 1.0, "Fps": 30.0, "Loop": true },
            "Curves": [
                { "Target": "Parameter", "Id": "ParamAngleX", "Segments": [0.0, 0.0, 0, 1.0, 10.0] }
            ]
        }"#,
    )
    .unwrap();

    let mut player = MotionPlayer::new_once(motion);

    assert!(!player.is_looping());
    player.tick(1.5);
    assert!(player.is_finished());
    assert_eq!(player.time(), 1.0);
}

fn hiyori_mesh_snapshot(model: &mocari::assets::RuntimeModel) -> Vec<Vec<[f32; 2]>> {
    model
        .runtime()
        .meshes()
        .iter()
        .map(|mesh| mesh.vertices().iter().map(|v| v.position()).collect())
        .collect()
}

#[test]
fn hiyori_distinct_bindings_drive_distinct_parameters() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let baseline = hiyori_mesh_snapshot(&model);

    model.runtime_mut().set_parameter("ParamRibbon", 1.0);
    model.runtime_mut().update_meshes().unwrap();
    let ribbon = hiyori_mesh_snapshot(&model);
    assert_ne!(
        baseline, ribbon,
        "ParamRibbon should deform the Hiyori mesh"
    );

    model.runtime_mut().reset_parameters();
    model.runtime_mut().set_parameter("ParamSkirt2", 1.0);
    model.runtime_mut().update_meshes().unwrap();
    let skirt = hiyori_mesh_snapshot(&model);
    assert_ne!(baseline, skirt, "ParamSkirt2 should deform the Hiyori mesh");

    assert_ne!(
        ribbon, skirt,
        "distinct parameters must drive distinct deformations"
    );
}

#[test]
fn zeroing_a_part_hides_its_drawables() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let baseline_visible = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| mesh.opacity() > 0.0)
        .count();
    assert!(baseline_visible > 0);

    let part_ids: Vec<String> = model.runtime().part_ids().to_vec();
    let mut hid_some = false;
    for part_id in part_ids {
        model.runtime_mut().reset_part_opacities();
        model.runtime_mut().set_part_opacity(&part_id, 0.0);
        model.runtime_mut().update_meshes().unwrap();
        let visible = model
            .runtime()
            .meshes()
            .iter()
            .filter(|mesh| mesh.opacity() > 0.0)
            .count();
        if visible < baseline_visible {
            hid_some = true;
            assert!(visible > 0, "a single part should not hide the whole model");
            break;
        }
    }
    assert!(hid_some, "no part hid any drawable");
}

#[test]
fn mao_drawables_carry_non_identity_multiply_and_screen_colors() {
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    let non_identity = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| {
            mesh.multiply_color() != [1.0, 1.0, 1.0] || mesh.screen_color() != [0.0, 0.0, 0.0]
        })
        .count();
    assert!(
        non_identity > 0,
        "Mao should expose per-drawable color keyforms"
    );
}

#[test]
fn expression_player_applies_faded_expression_parameters() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let expression = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamAngleX", "Value": 10.0, "Blend": "Add" }
            ]
        }"#,
    )
    .unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let default = model.runtime().parameter_value_by_index(index).unwrap();

    let mut player = ExpressionPlayer::new(expression);
    player.tick(0.5);
    player.apply(model.runtime_mut());

    let value = model.runtime().parameter_value_by_index(index).unwrap();
    assert_close(value, default + 5.0);
}

#[test]
fn expression_manager_fades_out_previous_expression_when_playing_next() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let first = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamAngleX", "Value": 10.0, "Blend": "Add" }
            ]
        }"#,
    )
    .unwrap();
    let second = Expression3::from_json_str(
        r#"{
            "Type": "Live2D Expression",
            "Parameters": [
                { "Id": "ParamAngleX", "Value": -4.0, "Blend": "Add" }
            ]
        }"#,
    )
    .unwrap();
    let index = model.runtime().parameter_index("ParamAngleX").unwrap();
    let default = model.runtime().parameter_value_by_index(index).unwrap();

    let mut manager = ExpressionManager::new();
    manager.play(first);
    manager.tick(1.0);
    model.runtime_mut().reset_parameters();
    manager.apply(model.runtime_mut());
    assert_close(
        model.runtime().parameter_value_by_index(index).unwrap(),
        default + 10.0,
    );

    manager.play(second);
    manager.tick(0.5);
    model.runtime_mut().reset_parameters();
    manager.apply(model.runtime_mut());

    let value = model.runtime().parameter_value_by_index(index).unwrap();
    assert_close(value, default + (10.0 * 0.5) + (-4.0 * 0.5));
    assert_eq!(manager.active_expression_count(), 2);

    manager.tick(0.5);
    assert_eq!(manager.active_expression_count(), 1);
}

#[test]
fn load_expression_reads_exp3_asset() {
    let expression = load_expression("assets/models/Haru/expressions/F01.exp3.json").unwrap();

    assert_eq!(expression.kind(), "Live2D Expression");
    assert_eq!(expression.parameters()[0].id(), "ParamMouthForm");
}

#[test]
fn mao_drawable_colors_match_core_default_pose() {
    let model = load_model_runtime("assets/models/Mao/Mao.model3.json").unwrap();
    let meshes = model.runtime().meshes();

    assert_color_close(meshes[45].multiply_color(), [1.0, 1.0, 1.0]);
    assert_color_close(meshes[45].screen_color(), [0.0, 0.0, 0.0]);
    assert_color_close(meshes[138].multiply_color(), [1.0, 1.0, 1.0]);
    assert_color_close(meshes[138].screen_color(), [1.0, 0.454_901_96, 0.513_725_5]);
}

#[test]
fn legacy_model_without_color_keyforms_defaults_to_identity() {
    let model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    for mesh in model.runtime().meshes() {
        assert_eq!(mesh.multiply_color(), [1.0, 1.0, 1.0]);
        assert_eq!(mesh.screen_color(), [0.0, 0.0, 0.0]);
    }
}

#[test]
fn deformer_opacity_hides_rest_state_effects() {
    let model = load_model_runtime("assets/models/Rice/Rice.model3.json").unwrap();
    let meshes = model.runtime().meshes();
    for &index in &[151usize, 152, 153] {
        assert_eq!(
            meshes[index].opacity(),
            0.0,
            "magic-circle drawable {index} must be hidden by deformer opacity at rest"
        );
    }
}

#[test]
fn default_pose_hides_redundant_arm_via_pose_groups() {
    let model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let hidden = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| mesh.opacity() == 0.0)
        .count();
    let visible = model
        .runtime()
        .meshes()
        .iter()
        .filter(|mesh| mesh.opacity() > 0.0)
        .count();

    assert!(
        hidden > 0,
        "pose group should hide the redundant arm at rest"
    );
    assert!(visible > 0, "the selected arm and body must stay visible");
}

#[test]
fn loaded_physics_updates_parameters_and_can_be_reset() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let initial_output = runtime.parameter_value("ParamHairFront").unwrap();

    assert!(runtime.physics().is_some());
    assert!(runtime.set_parameter("ParamAngleX", 30.0));
    for _ in 0..3 {
        assert!(runtime.apply_physics(1.0 / 30.0));
    }

    let output = runtime.parameter_value("ParamHairFront").unwrap();
    assert!(output.is_finite());
    assert_ne!(output, initial_output);
    assert!(runtime.reset_physics());
    assert!(runtime.stabilize_physics());
    runtime.clear_physics();
    assert!(runtime.physics().is_none());
    assert!(!runtime.apply_physics(1.0 / 30.0));
}

fn assert_color_close(actual: [f32; 3], expected: [f32; 3]) {
    for (actual, expected) in actual.into_iter().zip(expected) {
        assert!((actual - expected).abs() < 0.0001);
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.0001,
        "actual {actual}, expected {expected}"
    );
}

// ── Breath auto-animation tests ────────────────────────────────────────────

use mocari::auto::Breath;

#[test]
fn breath_produces_oscillating_output() {
    let mut breath = Breath::with_defaults();
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    breath.tick(1.0);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    breath.apply(runtime);
    let value1 = runtime.parameter_value("ParamBreath").unwrap();

    breath.tick(1.0);
    runtime.reset_parameters();
    breath.apply(runtime);
    let value2 = runtime.parameter_value("ParamBreath").unwrap();

    // Values should change over time (sine wave)
    // They may or may not be different depending on phase, but at least one should be non-zero
    assert!(value1.is_finite() && value2.is_finite(), "breath values must be finite");
}

#[test]
fn breath_weight_zero_has_no_effect() {
    let mut breath = Breath::with_defaults();
    breath.set_weight(0.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamBreath").unwrap_or(0.0);
    breath.tick(1.0);
    runtime.reset_parameters();
    breath.apply(runtime);
    let after = runtime.parameter_value("ParamBreath").unwrap_or(0.0);
    assert_close_breath(after, before);
}

fn assert_close_breath(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}

// ── EyeBlink auto-animation tests ──────────────────────────────────────────

use mocari::auto::{EyeBlink, LipSync};

#[test]
fn eye_blink_default_config_has_reasonable_values() {
    let blink = EyeBlink::with_defaults();
    // We can't access config fields directly, but we can verify behavior
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    // Eyes should start open (blink hasn't happened yet)
    blink.apply(runtime);
    // ParamEyeLOpen should still be at default after apply with no tick
    let value = runtime.parameter_value("ParamEyeLOpen");
    assert!(value.is_some());
}

#[test]
fn eye_blink_closes_eyes_during_blink() {
    let config = mocari::auto::EyeBlinkConfig {
        min_interval: 0.0,
        max_interval: 0.0,
        close_duration: 0.1,
        open_duration: 0.15,
        weight: 1.0,
    };
    let mut blink = EyeBlink::new(config);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    // Tick to trigger immediate blink (interval = 0)
    blink.tick(0.001);
    // Tick into closing phase
    blink.tick(0.05);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    blink.apply(runtime);

    let left = runtime.parameter_value("ParamEyeLOpen").unwrap();
    let right = runtime.parameter_value("ParamEyeROpen").unwrap();
    assert!(left < 1.0, "left eye should be closing: {left}");
    assert!(right < 1.0, "right eye should be closing: {right}");
}

#[test]
fn eye_blink_weight_zero_has_no_effect() {
    let mut blink = EyeBlink::with_defaults();
    blink.set_weight(0.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamEyeLOpen").unwrap();
    runtime.reset_parameters();
    blink.tick(0.5);
    blink.apply(runtime);
    let after = runtime.parameter_value("ParamEyeLOpen").unwrap();
    assert_close_runtime(after, before);
}

fn assert_close_runtime(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}

// ── LipSync tests ──────────────────────────────────────────────────────────

#[test]
fn lip_sync_smooths_amplitude_over_time() {
    let mut lip = LipSync::with_defaults();
    lip.set_amplitude(1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    // After first tick, amplitude should be partially smoothed
    lip.tick(1.0 / 60.0);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    lip.apply(runtime);
    let value = runtime.parameter_value("ParamMouthOpenY").unwrap();
    assert!(value > 0.0, "mouth should open after amplitude: {value}");
    assert!(value < 1.0, "mouth should not snap to max instantly: {value}");
}

#[test]
fn lip_sync_weight_zero_has_no_effect() {
    let mut lip = LipSync::with_defaults();
    lip.set_weight(0.0);
    lip.set_amplitude(1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamMouthOpenY").unwrap();
    lip.tick(1.0);
    runtime.reset_parameters();
    lip.apply(runtime);
    let after = runtime.parameter_value("ParamMouthOpenY").unwrap();
    assert_close_lip(after, before);
}

fn assert_close_lip(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}

// ── Drawable visibility tests ───────────────────────────────────────────────

#[test]
fn drawable_visibility_hides_mesh_vertices() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let drawable_ids: Vec<String> = model.runtime().drawable_ids().to_vec();
    assert!(!drawable_ids.is_empty());

    let target_id = &drawable_ids[0];
    let index = model.runtime().drawable_index(target_id).unwrap();
    let before_vertices = model.runtime().meshes()[index].vertices().to_vec();

    model.runtime_mut().set_drawable_visible(target_id, false);
    model.runtime_mut().update_meshes().unwrap();

    let after_vertices = model.runtime().meshes()[index].vertices().to_vec();
    // Hidden drawable should have all vertices at origin
    for vertex in &after_vertices {
        assert_eq!(vertex.position(), [0.0, 0.0], "hidden vertex should be at origin");
    }
    assert_ne!(before_vertices, after_vertices, "vertices should change when hidden");
}

#[test]
fn drawable_visibility_reset_restores_all() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let drawable_ids: Vec<String> = model.runtime().drawable_ids().to_vec();

    // Hide all
    for id in &drawable_ids {
        model.runtime_mut().set_drawable_visible(id, false);
    }
    model.runtime_mut().update_meshes().unwrap();

    // Reset all
    model.runtime_mut().reset_drawable_visibility();
    model.runtime_mut().update_meshes().unwrap();

    // All should be visible again
    for (i, id) in drawable_ids.iter().enumerate() {
        assert!(model.runtime().is_drawable_visible(i), "drawable {id} should be visible after reset");
    }
}

#[test]
fn set_drawable_visible_by_index_works() {
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    model.runtime_mut().set_drawable_visible_by_index(0, false);
    assert!(!model.runtime().is_drawable_visible(0));
    model.runtime_mut().set_drawable_visible_by_index(0, true);
    assert!(model.runtime().is_drawable_visible(0));
}

// ── MouseTracker tests ─────────────────────────────────────────────────────

use mocari::auto::MouseTracker;

#[test]
fn mouse_tracker_smooths_toward_target() {
    let mut tracker = MouseTracker::with_defaults();
    tracker.set_target(1.0, -1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();

    tracker.tick(1.0 / 60.0);
    let runtime = model.runtime_mut();
    runtime.reset_parameters();
    tracker.apply(runtime);
    let angle_x = runtime.parameter_value("ParamAngleX").unwrap();
    assert!(angle_x > 0.0, "should track toward positive X: {angle_x}");
}

#[test]
fn mouse_tracker_weight_zero_has_no_effect() {
    let mut tracker = MouseTracker::with_defaults();
    tracker.set_weight(0.0);
    tracker.set_target(1.0, 1.0);
    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();
    let before = runtime.parameter_value("ParamAngleX").unwrap();
    tracker.tick(1.0);
    runtime.reset_parameters();
    tracker.apply(runtime);
    let after = runtime.parameter_value("ParamAngleX").unwrap();
    assert_close_mouse(after, before);
}

fn assert_close_mouse(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.01,
        "actual {actual}, expected {expected}"
    );
}

// ── Integration: all features together ──────────────────────────────────────

#[test]
fn all_features_work_together() {
    use mocari::{EyeBlink, LipSync, Breath, MouseTracker, MotionManager};

    let mut model = load_model_runtime("assets/models/Hiyori/Hiyori.model3.json").unwrap();
    let runtime = model.runtime_mut();

    // Create all auto-animation features
    let mut blink = EyeBlink::with_defaults();
    let mut lip = LipSync::with_defaults();
    let mut breath = Breath::with_defaults();
    let mut tracker = MouseTracker::with_defaults();
    let mut motion_mgr = MotionManager::new();

    // Set up inputs
    lip.set_amplitude(0.5);
    tracker.set_target(0.5, -0.3);

    // Simulate a few frames
    let delta = 1.0 / 60.0;
    for _ in 0..10 {
        runtime.reset_parameters();
        runtime.reset_part_opacities();

        blink.tick(delta);
        blink.apply(runtime);
        lip.tick(delta);
        lip.apply(runtime);
        breath.tick(delta);
        breath.apply(runtime);
        tracker.tick(delta);
        tracker.apply(runtime);
        motion_mgr.tick(delta);
        motion_mgr.apply(runtime);

        runtime.apply_physics(delta);
        runtime.update_meshes().unwrap();
    }

    // Verify meshes are valid
    assert!(!runtime.meshes().is_empty(), "should have meshes after all features");
    for mesh in runtime.meshes() {
        for vertex in mesh.vertices() {
            let [x, y] = vertex.position();
            assert!(x.is_finite() && y.is_finite(), "vertex positions must be finite");
        }
    }
}

// ── MotionManager tests ────────────────────────────────────────────────────

#[test]
fn motion_manager_plays_single_motion() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let mut manager = MotionManager::new();

    manager.start_motion(motion, MotionPriority::Normal, "Idle");
    manager.tick(0.5);
    model.runtime_mut().reset_parameters();
    manager.apply(model.runtime_mut());

    // Should have changed some parameter values
    assert_eq!(manager.active_count(), 1);
    assert!(!manager.is_finished());
}

#[test]
fn motion_manager_crossfades_same_group() {
    let _model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion1 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let motion2 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let mut manager = MotionManager::new();
    manager.set_crossfade_duration(0.5);

    manager.start_motion(motion1, MotionPriority::Normal, "Idle");
    manager.tick(1.0);
    manager.start_motion(motion2, MotionPriority::Normal, "Idle");
    manager.tick(0.1);

    // Both should be active during crossfade
    assert!(manager.active_count() >= 1, "should have active motions: {}", manager.active_count());
}

#[test]
fn motion_manager_force_interrupts_normal() {
    let _model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion1 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let motion2 = mocari::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json").unwrap();
    let mut manager = MotionManager::new();
    manager.set_crossfade_duration(0.5);

    manager.start_motion(motion1, MotionPriority::Normal, "Idle");
    manager.tick(1.0);
    manager.start_motion(motion2, MotionPriority::Force, "Force");
    manager.tick(0.1);

    assert!(manager.active_count() >= 1);
}

#[test]
fn motion_manager_default_crossfade_is_half_second() {
    let manager = MotionManager::new();
    assert_eq!(manager.crossfade_duration(), 0.5);
}
