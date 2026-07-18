use rmcp::model::CallToolResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn extract_text(result: &CallToolResult) -> &str {
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("")
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
    let args = serde_json::json!({
        "name": "TestChar",
        "moc_path": "TestChar.moc3",
        "textures": ["tex1.png", "tex2.png"]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_model_json(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["FileReferences"]["Moc"], "TestChar.moc3");
    assert_eq!(parsed["FileReferences"]["Textures"][0], "tex1.png");
    assert_eq!(parsed["FileReferences"]["Textures"][1], "tex2.png");
}

#[tokio::test]
async fn create_model_json_with_motions_and_expressions() {
    let args = serde_json::json!({
        "name": "Full",
        "moc_path": "full.moc3",
        "textures": ["t.png"],
        "motions": { "Idle": [{ "File": "idle.motion3.json" }] },
        "expressions": [{ "Name": "happy", "File": "happy.exp3.json" }]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_model_json(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert!(parsed["FileReferences"]["Motions"]["Idle"].as_array().is_some());
    assert!(parsed["FileReferences"]["Expressions"].as_array().is_some());
}

#[tokio::test]
async fn create_model_json_missing_name_is_error() {
    let args = serde_json::json!({
        "moc_path": "x.moc3",
        "textures": ["t.png"]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_model_json(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_model_json_missing_textures_is_error() {
    let args = serde_json::json!({
        "name": "x",
        "moc_path": "x.moc3"
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_model_json(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_model_json_missing_moc_path_is_error() {
    let args = serde_json::json!({
        "name": "x",
        "textures": ["t.png"]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_model_json(args).await;
    assert!(result.is_err());
}

// ===========================================================================
// create_motion_json
// ===========================================================================

#[tokio::test]
async fn create_motion_json_success() {
    let args = serde_json::json!({
        "duration": 2.0,
        "fps": 30.0,
        "loop": true,
        "curves": []
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_motion_json(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["Duration"], 2.0);
    assert_eq!(parsed["Meta"]["Fps"], 30.0);
    assert_eq!(parsed["Meta"]["Loop"], true);
    assert_eq!(parsed["Meta"]["CurveCount"], 0);
}

#[tokio::test]
async fn create_motion_json_default_loop_false() {
    let args = serde_json::json!({
        "duration": 1.0,
        "fps": 30.0,
        "curves": []
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_motion_json(args)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["Meta"]["Loop"], false);
}

#[tokio::test]
async fn create_motion_json_missing_duration_is_error() {
    let args = serde_json::json!({
        "fps": 30.0,
        "curves": []
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_motion_json(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_motion_json_missing_fps_is_error() {
    let args = serde_json::json!({
        "duration": 1.0,
        "curves": []
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_motion_json(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_motion_json_missing_curves_is_error() {
    let args = serde_json::json!({
        "duration": 1.0,
        "fps": 30.0
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_motion_json(args).await;
    assert!(result.is_err());
}

// ===========================================================================
// create_expression_json
// ===========================================================================

#[tokio::test]
async fn create_expression_json_success() {
    let args = serde_json::json!({
        "fade_in": 0.3,
        "fade_out": 0.5,
        "parameters": [
            { "id": "ParamEyeLOpen", "value": 0.0 }
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_expression_json(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["Type"], "Additive");
    assert_eq!(parsed["FadeInTime"], 0.3);
    assert_eq!(parsed["FadeOutTime"], 0.5);
    assert_eq!(parsed["Parameters"][0]["id"], "ParamEyeLOpen");
    assert_eq!(parsed["Parameters"][0]["value"], 0.0);
}

#[tokio::test]
async fn create_expression_json_defaults() {
    let args = serde_json::json!({
        "parameters": [{ "id": "ParamMouthOpenY", "value": 1.0 }]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_expression_json(args)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["FadeInTime"], 0.5, "default fade_in should be 0.5");
    assert_eq!(parsed["FadeOutTime"], 0.5, "default fade_out should be 0.5");
}

#[tokio::test]
async fn create_expression_json_missing_parameters_is_error() {
    let args = serde_json::json!({
        "fade_in": 0.3
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_expression_json(args).await;
    assert!(result.is_err());
}

// ===========================================================================
// create_physics_json
// ===========================================================================

#[tokio::test]
async fn create_physics_json_success() {
    let args = serde_json::json!({
        "settings": { "gravity": [0.0, -1.0] },
        "physics_info": [
            { "id": "Hair", "input": [], "output": [], "vertices": [] }
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_physics_json(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["PhysicsSettingCount"], 1);
    assert!(parsed["PhysicsSettings"].as_array().is_some());
}

#[tokio::test]
async fn create_physics_json_missing_settings_is_error() {
    let args = serde_json::json!({
        "physics_info": []
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_physics_json(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_physics_json_missing_physics_info_is_error() {
    let args = serde_json::json!({
        "settings": {}
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_physics_json(args).await;
    assert!(result.is_err());
}

// ===========================================================================
// create_pose_json
// ===========================================================================

#[tokio::test]
async fn create_pose_json_success() {
    let args = serde_json::json!({
        "fade_in_time": 0.8,
        "groups": [
            [{ "id": "PartArmL", "link": [] }]
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_pose_json(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["Type"], "Live2D Pose");
    assert_eq!(parsed["FadeInTime"], 0.8);
    assert!(parsed["Groups"].as_array().is_some());
}

#[tokio::test]
async fn create_pose_json_default_fade_in() {
    let args = serde_json::json!({
        "groups": [[{ "id": "X", "link": [] }]]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_pose_json(args)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["FadeInTime"], 0.5);
}

#[tokio::test]
async fn create_pose_json_missing_groups_is_error() {
    let args = serde_json::json!({
        "fade_in_time": 0.5
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_pose_json(args).await;
    assert!(result.is_err());
}

// ===========================================================================
// create_userdata_json
// ===========================================================================

#[tokio::test]
async fn create_userdata_json_success() {
    let args = serde_json::json!({
        "entries": [
            { "target": "ArtMesh", "id": "HairFront", "value": "bangs" }
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_userdata_json(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["UserDataCount"], 1);
    assert!(parsed["UserData"].as_array().is_some());
}

#[tokio::test]
async fn create_userdata_json_missing_entries_is_error() {
    let args = serde_json::json!({}).as_object().unwrap().clone();
    let result = mocari::mcp::creator::handle_create_userdata_json(args).await;
    assert!(result.is_err());
}

// ===========================================================================
// create_simple_moc3 (not implemented)
// ===========================================================================

#[tokio::test]
async fn create_simple_moc3_returns_tool_error() {
    let args = serde_json::json!({
        "name": "test",
        "width": 1024.0,
        "height": 1024.0,
        "parameters": [{ "id": "ParamX" }],
        "meshes": [{ "id": "Mesh1" }]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_simple_moc3(args)
        .await
        .unwrap();
    assert!(is_error(&result), "create_simple_moc3 should return tool error");
}

#[tokio::test]
async fn create_simple_moc3_missing_args_is_transport_error() {
    let args = serde_json::json!({
        "name": "test"
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_simple_moc3(args).await;
    assert!(result.is_err());
}

// ===========================================================================
// create_model_bundle
// ===========================================================================

#[tokio::test]
async fn create_model_bundle_success() {
    let args = serde_json::json!({
        "name": "MyChar",
        "description": "A test character",
        "meshes": [{ "id": "Body" }],
        "parameters": [{ "id": "ParamX" }],
        "motions": [],
        "expressions": []
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_model_bundle(args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();

    // model_json should be a valid object with correct moc reference
    let model_json = parsed["model_json"]
        .as_object()
        .expect("model_json should be an object");
    assert_eq!(model_json["Version"], 3);
    let moc_ref = model_json["FileReferences"]["Moc"].as_str().unwrap();
    assert_eq!(moc_ref, "MyChar.moc3");

    // files array should list 3 files
    let files = parsed["files"].as_array().expect("files should be an array");
    assert_eq!(files.len(), 3);
    assert!(files.iter().any(|f| f.as_str().unwrap().contains("model3.json")));
    assert!(files.iter().any(|f| f.as_str().unwrap().contains(".png")));
    assert!(files.iter().any(|f| f.as_str().unwrap().contains(".moc3")));
}

#[tokio::test]
async fn create_model_bundle_with_motions_and_expressions() {
    let args = serde_json::json!({
        "name": "Full",
        "meshes": [{ "id": "Body" }],
        "parameters": [{ "id": "ParamX" }],
        "motions": [{ "File": "idle.motion3.json" }],
        "expressions": [{ "Name": "happy", "File": "happy.exp3.json" }]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_model_bundle(args)
        .await
        .unwrap();
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    let model_json = parsed["model_json"].as_object().unwrap();
    assert!(model_json["FileReferences"]["Motions"][""].as_array().is_some());
    assert!(model_json["FileReferences"]["Expressions"].as_array().is_some());
}

#[tokio::test]
async fn create_model_bundle_missing_name_is_error() {
    let args = serde_json::json!({
        "meshes": [{ "id": "X" }],
        "parameters": [{ "id": "P" }]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_model_bundle(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_model_bundle_missing_meshes_is_error() {
    let args = serde_json::json!({
        "name": "X",
        "parameters": [{ "id": "P" }]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_model_bundle(args).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn create_model_bundle_missing_parameters_is_error() {
    let args = serde_json::json!({
        "name": "X",
        "meshes": [{ "id": "M" }]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::creator::handle_create_model_bundle(args).await;
    assert!(result.is_err());
}
