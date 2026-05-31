use rusty_live2d::json::{
    Pose3, copy_pose_link_opacities, resolved_pose_fade_in_time, update_pose_group_opacities,
};

#[test]
fn parses_pose3_groups_and_links() {
    let pose = Pose3::from_json_str(
        r#"{
            "Type": "Live2D Pose",
            "FadeInTime": 0.5,
            "Groups": [
                [
                    { "Id": "PartArmLA", "Link": ["PartArmLB"] },
                    { "Id": "PartArmRA" }
                ]
            ]
        }"#,
    )
    .unwrap();

    assert_eq!(pose.kind(), "Live2D Pose");
    assert_eq!(pose.fade_in_time(), Some(0.5));
    assert_eq!(pose.groups().len(), 1);
    assert_eq!(pose.groups()[0][0].id(), "PartArmLA");
    assert_eq!(pose.groups()[0][0].links(), ["PartArmLB"]);
    assert_eq!(pose.groups()[0][1].id(), "PartArmRA");
    assert!(pose.groups()[0][1].links().is_empty());
}

#[test]
fn pose3_defaults_missing_groups_to_empty() {
    let pose = Pose3::from_json_str(
        r#"{
            "Type": "Live2D Pose"
        }"#,
    )
    .unwrap();

    assert!(pose.groups().is_empty());
}

#[test]
fn resolves_pose_fade_time_like_framework() {
    let pose = Pose3::from_json_str(
        r#"{
            "Type": "Live2D Pose",
            "FadeInTime": -1.0
        }"#,
    )
    .unwrap();
    let missing = Pose3::from_json_str(r#"{ "Type": "Live2D Pose" }"#).unwrap();

    assert_eq!(resolved_pose_fade_in_time(pose.fade_in_time()), 0.5);
    assert_eq!(pose.resolved_fade_in_time(), 0.5);
    assert_eq!(missing.resolved_fade_in_time(), 0.5);
}

#[test]
fn updates_pose_group_opacity_with_background_threshold() {
    let parameters = [0.0_f32, 1.0];
    let mut opacities = [1.0_f32, 0.0];

    update_pose_group_opacities(&parameters, &mut opacities, 0.1, 0.5).unwrap();

    assert!((opacities[0] - 0.8125).abs() < 0.00001);
    assert!((opacities[1] - 0.2).abs() < 0.00001);
}

#[test]
fn pose_group_falls_back_to_first_part_when_no_parameter_visible() {
    let parameters = [0.0_f32, 0.0];
    let mut opacities = [0.25_f32, 0.5];

    update_pose_group_opacities(&parameters, &mut opacities, -0.2, 0.5).unwrap();

    assert_eq!(opacities, [1.0, 0.0]);
}

#[test]
fn copies_pose_link_opacities() {
    let mut opacities = [0.35, 0.0, 1.0];

    copy_pose_link_opacities(&mut opacities, 0, &[1, 2]).unwrap();

    assert_eq!(opacities, [0.35, 0.35, 0.35]);
}
