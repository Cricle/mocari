use rusty_live2d::{
    assets::{load_model, load_model_runtime},
    json::Motion3,
    motion::MotionPlayer,
};

#[test]
fn runtime_default_pose_matches_default_model() {
    let runtime = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let default = load_model("assets/models/Haru/Haru.model3.json").unwrap();

    let runtime_meshes = runtime.runtime().meshes();
    let default_meshes = default.meshes();

    assert_eq!(runtime_meshes.len(), default_meshes.len());
    for (left, right) in runtime_meshes.iter().zip(default_meshes) {
        assert_eq!(left.vertices(), right.vertices());
        assert_eq!(left.opacity(), right.opacity());
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
fn motion_player_drives_a_parameter_over_time() {
    let mut model = load_model_runtime("assets/models/Haru/Haru.model3.json").unwrap();
    let motion =
        rusty_live2d::motion::load_motion("assets/models/Haru/motions/haru_g_idle.motion3.json")
            .unwrap();

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
