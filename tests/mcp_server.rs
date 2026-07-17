#![cfg(feature = "mcp")]
#![forbid(unsafe_code)]

use std::sync::Arc;
use tokio::sync::Mutex;

use mocari::mcp::{MocariMcpServer, ModelSession};
use mocari::mcp::session::SessionError;
use rmcp::handler::server::ServerHandler;
use rmcp::model::CallToolResult;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_args() -> rmcp::model::JsonObject {
    serde_json::json!({}).as_object().unwrap().clone()
}

fn make_session() -> Arc<Mutex<ModelSession>> {
    Arc::new(Mutex::new(ModelSession::new()))
}

// ---------------------------------------------------------------------------
// Server info
// ---------------------------------------------------------------------------

#[test]
fn test_server_info_name() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    assert_eq!(info.server_info.name, "mocari-mcp");
}

#[test]
fn test_server_info_has_instructions() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    let instructions = info.instructions.unwrap_or_default();
    assert!(
        instructions.contains("Live2D"),
        "instructions should mention Live2D, got: {instructions}"
    );
    assert!(
        instructions.contains("load_model"),
        "instructions should mention load_model, got: {instructions}"
    );
}

#[test]
fn test_server_info_has_tools_capability() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    assert!(
        info.capabilities.tools.is_some(),
        "server must declare tools capability"
    );
}

// ---------------------------------------------------------------------------
// ModelSession management
// ---------------------------------------------------------------------------

#[test]
fn test_session_load_nonexistent_path() {
    let mut session = ModelSession::new();
    let result = session.load_model("/nonexistent/path/to/model.model3.json");
    assert!(result.is_err(), "loading nonexistent file should fail");
}

#[test]
fn test_session_unload_nonexistent_model() {
    let mut session = ModelSession::new();
    assert!(
        !session.unload_model("model_999"),
        "unloading nonexistent model should return false"
    );
}

#[test]
fn test_session_list_empty() {
    let session = ModelSession::new();
    assert!(session.models.is_empty(), "new session should have no models");
    assert!(session.list_models().is_empty(), "list_models should be empty");
}

#[test]
fn test_session_with_model_nonexistent() {
    let session = ModelSession::new();
    let result = session.with_model("model_999", |_| 42);
    assert!(result.is_err(), "with_model on missing ID should fail");
    match result.unwrap_err() {
        SessionError::ModelNotFound(id) => assert_eq!(id, "model_999"),
        other => panic!("expected ModelNotFound, got: {other}"),
    }
}

#[test]
fn test_session_with_model_mut_nonexistent() {
    let mut session = ModelSession::new();
    let result = session.with_model_mut("model_999", |_| 42);
    assert!(result.is_err(), "with_model_mut on missing ID should fail");
}

// ---------------------------------------------------------------------------
// Runtime handler: list_models (empty session)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_list_models_empty() {
    let session = make_session();
    let result = mocari::mcp::runtime::handle_list_models(&session, empty_args()).await;
    let call_result = result.expect("list_models should succeed");
    assert_eq!(call_result.is_error, Some(false));
    let text = extract_text(&call_result);
    assert_eq!(text, "[]", "empty session should list zero models");
}

// ---------------------------------------------------------------------------
// Runtime handler: unload_model (nonexistent)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_unload_model_nonexistent() {
    let session = make_session();
    let args = serde_json::json!({ "model_id": "model_999" })
        .as_object()
        .unwrap()
        .clone();
    let result = mocari::mcp::runtime::handle_unload_model(&session, args).await;
    let call_result = result.expect("should not be a transport error");
    assert_eq!(
        call_result.is_error,
        Some(true),
        "unloading nonexistent model should be an error result"
    );
}

// ---------------------------------------------------------------------------
// Runtime handler: missing required args
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_list_parameters_missing_model_id() {
    let session = make_session();
    let result = mocari::mcp::runtime::handle_list_parameters(&session, empty_args()).await;
    // Returns Err(ErrorData) when required arg is missing
    assert!(
        result.is_err(),
        "list_parameters without model_id should return error"
    );
}

#[tokio::test]
async fn test_handle_set_parameter_missing_args() {
    let session = make_session();
    let result = mocari::mcp::runtime::handle_set_parameter(&session, empty_args()).await;
    assert!(
        result.is_err(),
        "set_parameter without args should return error"
    );
}

#[tokio::test]
async fn test_handle_tick_missing_model_id() {
    let session = make_session();
    let result = mocari::mcp::runtime::handle_tick(&session, empty_args()).await;
    assert!(
        result.is_err(),
        "tick without model_id should return error"
    );
}

// ---------------------------------------------------------------------------
// Creator handler: create_model_json (success)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_model_json_success() {
    let args = serde_json::json!({
        "name": "test_model",
        "moc_path": "test.moc3",
        "textures": ["tex1.png", "tex2.png"],
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_model_json(args).await;
    let call_result = result.expect("create_model_json should succeed");
    assert_eq!(call_result.is_error, Some(false));

    let text = extract_text(&call_result);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("result should be valid JSON");
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["FileReferences"]["Moc"], "test.moc3");
    assert_eq!(parsed["FileReferences"]["Textures"][0], "tex1.png");
    assert_eq!(parsed["FileReferences"]["Textures"][1], "tex2.png");
}

#[tokio::test]
async fn test_handle_create_model_json_missing_name() {
    let args = serde_json::json!({
        "moc_path": "test.moc3",
        "textures": ["tex.png"],
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_model_json(args).await;
    assert!(
        result.is_err(),
        "create_model_json without 'name' should fail"
    );
}

#[tokio::test]
async fn test_handle_create_model_json_missing_textures() {
    let args = serde_json::json!({
        "name": "test",
        "moc_path": "test.moc3",
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_model_json(args).await;
    assert!(
        result.is_err(),
        "create_model_json without 'textures' should fail"
    );
}

// ---------------------------------------------------------------------------
// Creator handler: create_motion_json
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_motion_json_success() {
    let args = serde_json::json!({
        "duration": 2.0,
        "fps": 30.0,
        "loop": true,
        "curves": []
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_motion_json(args).await;
    let call_result = result.expect("create_motion_json should succeed");
    assert_eq!(call_result.is_error, Some(false));

    let text = extract_text(&call_result);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("result should be valid JSON");
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["Duration"], 2.0);
    assert_eq!(parsed["Meta"]["Fps"], 30.0);
    assert_eq!(parsed["Meta"]["Loop"], true);
}

#[tokio::test]
async fn test_handle_create_motion_json_missing_duration() {
    let args = serde_json::json!({
        "fps": 30.0,
        "curves": []
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_motion_json(args).await;
    assert!(
        result.is_err(),
        "create_motion_json without 'duration' should fail"
    );
}

// ---------------------------------------------------------------------------
// Creator handler: create_expression_json
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_expression_json_success() {
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

    let result = mocari::mcp::creator::handle_create_expression_json(args).await;
    let call_result = result.expect("create_expression_json should succeed");
    assert_eq!(call_result.is_error, Some(false));

    let text = extract_text(&call_result);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("result should be valid JSON");
    assert_eq!(parsed["Type"], "Additive");
    assert_eq!(parsed["FadeInTime"], 0.3);
    assert_eq!(parsed["FadeOutTime"], 0.5);
    assert_eq!(parsed["Parameters"][0]["id"], "ParamEyeLOpen");
}

#[tokio::test]
async fn test_handle_create_expression_json_defaults() {
    let args = serde_json::json!({
        "parameters": [{ "id": "ParamMouthOpenY", "value": 1.0 }]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_expression_json(args).await;
    let call_result = result.expect("should succeed with only required args");
    let text = extract_text(&call_result);
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["FadeInTime"], 0.5, "default fade_in should be 0.5");
    assert_eq!(parsed["FadeOutTime"], 0.5, "default fade_out should be 0.5");
}

// ---------------------------------------------------------------------------
// Creator handler: create_physics_json
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_physics_json_success() {
    let args = serde_json::json!({
        "settings": { "gravity": [0.0, -1.0] },
        "physics_info": [{ "id": "Hair", "input": [], "output": [] }]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_physics_json(args).await;
    let call_result = result.expect("create_physics_json should succeed");
    assert_eq!(call_result.is_error, Some(false));

    let text = extract_text(&call_result);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("result should be valid JSON");
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["PhysicsSettingCount"], 1);
}

// ---------------------------------------------------------------------------
// Creator handler: create_pose_json
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_pose_json_success() {
    let args = serde_json::json!({
        "fade_in_time": 0.8,
        "groups": [
            [{ "id": "PartArmL", "link": [] }]
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_pose_json(args).await;
    let call_result = result.expect("create_pose_json should succeed");
    assert_eq!(call_result.is_error, Some(false));

    let text = extract_text(&call_result);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("result should be valid JSON");
    assert_eq!(parsed["Type"], "Live2D Pose");
    assert_eq!(parsed["FadeInTime"], 0.8);
}

// ---------------------------------------------------------------------------
// Creator handler: create_userdata_json
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_userdata_json_success() {
    let args = serde_json::json!({
        "entries": [
            { "target": "ArtMesh", "id": "HairFront", "value": "bangs" }
        ]
    })
    .as_object()
    .unwrap()
    .clone();

    let result = mocari::mcp::creator::handle_create_userdata_json(args).await;
    let call_result = result.expect("create_userdata_json should succeed");
    assert_eq!(call_result.is_error, Some(false));

    let text = extract_text(&call_result);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("result should be valid JSON");
    assert_eq!(parsed["Version"], 3);
    assert_eq!(parsed["Meta"]["UserDataCount"], 1);
}

// ---------------------------------------------------------------------------
// Creator handler: create_simple_moc3 (not yet implemented)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_simple_moc3_returns_error() {
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

    let result = mocari::mcp::creator::handle_create_simple_moc3(args).await;
    let call_result = result.expect("should return a CallToolResult, not transport error");
    assert_eq!(
        call_result.is_error,
        Some(true),
        "create_simple_moc3 should indicate not-implemented error"
    );
}

// ---------------------------------------------------------------------------
// Creator handler: create_model_bundle
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_create_model_bundle_success() {
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

    let result = mocari::mcp::creator::handle_create_model_bundle(args).await;
    let call_result = result.expect("create_model_bundle should succeed");
    assert_eq!(call_result.is_error, Some(false));

    let text = extract_text(&call_result);
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("result should be valid JSON");
    // The bundle should reference files with the character name
    let model_json_str = parsed["model_json"].as_str().expect("model_json should be a string");
    assert!(
        model_json_str.contains("MyChar.moc3"),
        "model JSON should reference the moc3 file"
    );
    let files = parsed["files"].as_array().expect("files should be an array");
    assert_eq!(files.len(), 3, "bundle should contain 3 files");
}

// ---------------------------------------------------------------------------
// Runtime handler: load_model with nonexistent path
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_handle_load_model_nonexistent() {
    let session = make_session();
    let args = serde_json::json!({ "path": "/nonexistent/model.model3.json" })
        .as_object()
        .unwrap()
        .clone();
    let result = mocari::mcp::runtime::handle_load_model(&session, args).await;
    let call_result = result.expect("should return CallToolResult, not transport error");
    assert_eq!(
        call_result.is_error,
        Some(true),
        "loading nonexistent path should be an error result"
    );
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extract the text content from a CallToolResult containing a single text block.
fn extract_text(result: &CallToolResult) -> &str {
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("")
}
