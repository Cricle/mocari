use rmcp::model::CallToolResult;

fn extract_text(result: &CallToolResult) -> &str {
    result.content.first().and_then(|c| c.as_text()).map(|t| t.text.as_str()).unwrap_or("")
}

fn is_success(result: &CallToolResult) -> bool {
    result.is_error == Some(false)
}

fn is_error(result: &CallToolResult) -> bool {
    result.is_error == Some(true)
}

// ===========================================================================
// create_model_json
// ===========================================================================

#[tokio::test]
async fn create_model_json_success() {
    let args = serde_json::json!({"name":"T","moc_path":"t.moc3","textures":["t.png"]})
        .as_object().unwrap().clone();
    let result = mocari::mcp::creator::handle_create_model_json(args).await.unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["FileReferences"]["Moc"], "t.moc3");
}

#[tokio::test]
async fn create_model_json_with_motions_and_expressions() {
    let args = serde_json::json!({
        "name":"F","moc_path":"f.moc3","textures":["t.png"],
        "motions":{"Idle":[{"File":"i.motion3.json"}]},
        "expressions":[{"Name":"h","File":"h.exp3.json"}]
    }).as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_model_json(args).await.unwrap())
    ).unwrap();
    assert!(parsed["FileReferences"]["Motions"]["Idle"].as_array().is_some());
}

#[tokio::test]
async fn create_model_json_missing_name_is_error() {
    let args = serde_json::json!({"moc_path":"x.moc3","textures":["t.png"]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_model_json(args).await.is_err());
}

#[tokio::test]
async fn create_model_json_missing_textures_is_error() {
    let args = serde_json::json!({"name":"x","moc_path":"x.moc3"}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_model_json(args).await.is_err());
}

#[tokio::test]
async fn create_model_json_missing_moc_path_is_error() {
    let args = serde_json::json!({"name":"x","textures":["t.png"]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_model_json(args).await.is_err());
}

// ===========================================================================
// create_motion_json
// ===========================================================================

#[tokio::test]
async fn create_motion_json_success() {
    let args = serde_json::json!({"duration":2.0,"fps":30.0,"loop":true,"curves":[]})
        .as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_motion_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["Duration"], 2.0);
    assert_eq!(parsed["Meta"]["Loop"], true);
}

#[tokio::test]
async fn create_motion_json_default_loop_false() {
    let args = serde_json::json!({"duration":1.0,"fps":30.0,"curves":[]}).as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_motion_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["Meta"]["Loop"], false);
}

#[tokio::test]
async fn create_motion_json_missing_duration_is_error() {
    let args = serde_json::json!({"fps":30.0,"curves":[]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_motion_json(args).await.is_err());
}

#[tokio::test]
async fn create_motion_json_missing_fps_is_error() {
    let args = serde_json::json!({"duration":1.0,"curves":[]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_motion_json(args).await.is_err());
}

#[tokio::test]
async fn create_motion_json_missing_curves_is_error() {
    let args = serde_json::json!({"duration":1.0,"fps":30.0}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_motion_json(args).await.is_err());
}

// ===========================================================================
// create_expression_json
// ===========================================================================

#[tokio::test]
async fn create_expression_json_success() {
    let args = serde_json::json!({"fade_in":0.3,"fade_out":0.5,"parameters":[{"id":"P","value":0.0}]})
        .as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_expression_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["Type"], "Additive");
    assert_eq!(parsed["FadeInTime"], 0.3);
    assert_eq!(parsed["Parameters"][0]["id"], "P");
}

#[tokio::test]
async fn create_expression_json_defaults() {
    let args = serde_json::json!({"parameters":[{"id":"P","value":1.0}]}).as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_expression_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["FadeInTime"], 0.5);
    assert_eq!(parsed["FadeOutTime"], 0.5);
}

#[tokio::test]
async fn create_expression_json_missing_parameters_is_error() {
    let args = serde_json::json!({"fade_in":0.3}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_expression_json(args).await.is_err());
}

// ===========================================================================
// create_physics_json
// ===========================================================================

#[tokio::test]
async fn create_physics_json_success() {
    let args = serde_json::json!({"settings":{"gravity":[0.0,-1.0]},"physics_info":[{"id":"H","input":[],"output":[],"vertices":[]}]})
        .as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_physics_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["PhysicsSettingCount"], 1);
}

#[tokio::test]
async fn create_physics_json_missing_settings_is_error() {
    let args = serde_json::json!({"physics_info":[]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_physics_json(args).await.is_err());
}

#[tokio::test]
async fn create_physics_json_missing_physics_info_is_error() {
    let args = serde_json::json!({"settings":{}}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_physics_json(args).await.is_err());
}

// ===========================================================================
// create_pose_json
// ===========================================================================

#[tokio::test]
async fn create_pose_json_success() {
    let args = serde_json::json!({"fade_in_time":0.8,"groups":[[{"id":"P","link":[]}]]})
        .as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_pose_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["Type"], "Live2D Pose");
    assert_eq!(parsed["FadeInTime"], 0.8);
}

#[tokio::test]
async fn create_pose_json_default_fade_in() {
    let args = serde_json::json!({"groups":[[{"id":"X","link":[]}]]}).as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_pose_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["FadeInTime"], 0.5);
}

#[tokio::test]
async fn create_pose_json_missing_groups_is_error() {
    let args = serde_json::json!({"fade_in_time":0.5}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_pose_json(args).await.is_err());
}

// ===========================================================================
// create_userdata_json
// ===========================================================================

#[tokio::test]
async fn create_userdata_json_success() {
    let args = serde_json::json!({"entries":[{"target":"ArtMesh","id":"H","value":"b"}]})
        .as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_userdata_json(args).await.unwrap())
    ).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["UserDataCount"], 1);
}

#[tokio::test]
async fn create_userdata_json_missing_entries_is_error() {
    let args = serde_json::json!({}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_userdata_json(args).await.is_err());
}

// ===========================================================================
// create_simple_moc3
// ===========================================================================

#[tokio::test]
async fn create_simple_moc3_returns_tool_error() {
    let args = serde_json::json!({"name":"t","width":1024.0,"height":1024.0,"parameters":[{"id":"P"}],"meshes":[{"id":"M"}]})
        .as_object().unwrap().clone();
    assert!(is_error(&mocari::mcp::creator::handle_create_simple_moc3(args).await.unwrap()));
}

#[tokio::test]
async fn create_simple_moc3_missing_args_is_transport_error() {
    let args = serde_json::json!({"name":"t"}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_simple_moc3(args).await.is_err());
}

// ===========================================================================
// create_model_bundle
// ===========================================================================

#[tokio::test]
async fn create_model_bundle_success() {
    let args = serde_json::json!({"name":"My","meshes":[{"id":"B"}],"parameters":[{"id":"P"}],"motions":[],"expressions":[]})
        .as_object().unwrap().clone();
    let parsed: serde_json::Value = serde_json::from_str(
        extract_text(&mocari::mcp::creator::handle_create_model_bundle(args).await.unwrap())
    ).unwrap();
    let model_json = parsed["model_json"].as_object().unwrap();
    assert_eq!(model_json["FileReferences"]["Moc"].as_str().unwrap(), "My.moc3");
    assert_eq!(parsed["files"].as_array().unwrap().len(), 3);
}

#[tokio::test]
async fn create_model_bundle_missing_name_is_error() {
    let args = serde_json::json!({"meshes":[{"id":"X"}],"parameters":[{"id":"P"}]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_model_bundle(args).await.is_err());
}

#[tokio::test]
async fn create_model_bundle_missing_meshes_is_error() {
    let args = serde_json::json!({"name":"X","parameters":[{"id":"P"}]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_model_bundle(args).await.is_err());
}

#[tokio::test]
async fn create_model_bundle_missing_parameters_is_error() {
    let args = serde_json::json!({"name":"X","meshes":[{"id":"M"}]}).as_object().unwrap().clone();
    assert!(mocari::mcp::creator::handle_create_model_bundle(args).await.is_err());
}
