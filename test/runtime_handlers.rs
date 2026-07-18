use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use mocari::mcp::session::ModelSession;
use rmcp::model::CallToolResult;
use rmcp::model::JsonObject;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn empty_args() -> JsonObject {
    serde_json::json!({}).as_object().unwrap().clone()
}

fn args_with_model_id(model_id: &str) -> JsonObject {
    serde_json::json!({ "model_id": model_id })
        .as_object()
        .unwrap()
        .clone()
}

fn ren_model_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/models/Ren/Ren.model3.json")
        .display()
        .to_string()
}

fn ren_motion_path() -> String {
    "motions/mtn_01.motion3.json".to_string()
}

fn ren_expression_path() -> String {
    "expressions/exp_01.exp3.json".to_string()
}

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

async fn load_ren_model() -> (Arc<Mutex<ModelSession>>, String) {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({ "path": ren_model_path() })
        .as_object()
        .unwrap()
        .clone();
    let result = mocari::mcp::runtime::handle_load_model(&session, args)
        .await
        .expect("load should succeed");
    assert!(is_success(&result), "load_model failed: {}", extract_text(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    let model_id = parsed["model_id"].as_str().unwrap().to_string();
    (session, model_id)
}

// ===========================================================================
// Model lifecycle
// ===========================================================================

#[tokio::test]
async fn handle_load_model_success() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({ "path": ren_model_path() })
        .as_object()
        .unwrap()
        .clone();
    let result = mocari::mcp::runtime::handle_load_model(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(parsed["model_id"].as_str().is_some());
    assert!(parsed["parameter_count"].as_u64().unwrap() > 0);
    assert!(parsed["drawable_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn handle_load_model_nonexistent() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({ "path": "/nonexistent/model.model3.json" })
        .as_object()
        .unwrap()
        .clone();
    let result = mocari::mcp::runtime::handle_load_model(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn handle_list_models_shows_loaded() {
    let (session, model_id) = load_ren_model().await;
    let result = mocari::mcp::runtime::handle_list_models(&session, empty_args())
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["model_id"].as_str().unwrap(), model_id);
    assert!(parsed[0]["parameter_count"].as_u64().unwrap() > 0);
    assert!(parsed[0]["drawable_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn handle_list_models_empty() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_list_models(&session, empty_args())
        .await
        .unwrap();
    assert!(is_success(&result));
    assert_eq!(extract_text(&result), "[]");
}

#[tokio::test]
async fn handle_unload_model_success() {
    let (session, model_id) = load_ren_model().await;
    let args = args_with_model_id(&model_id);
    let result = mocari::mcp::runtime::handle_unload_model(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    // Verify list is empty after unload
    let list_result = mocari::mcp::runtime::handle_list_models(&session, empty_args())
        .await
        .unwrap();
    assert_eq!(extract_text(&list_result), "[]");
}

#[tokio::test]
async fn handle_unload_model_nonexistent() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = args_with_model_id("model_999");
    let result = mocari::mcp::runtime::handle_unload_model(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

// ===========================================================================
// Parameters
// ===========================================================================

#[tokio::test]
async fn handle_list_parameters_success() {
    let (session, model_id) = load_ren_model().await;
    let args = args_with_model_id(&model_id);
    let result = mocari::mcp::runtime::handle_list_parameters(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert!(!parsed.is_empty(), "should have parameters");
    // Each param must have id, min, max, default, current
    for p in &parsed {
        assert!(p["id"].as_str().is_some(), "param must have id");
        assert!(p["min"].is_number(), "param must have min");
        assert!(p["max"].is_number(), "param must have max");
        assert!(p["default"].is_number(), "param must have default");
        assert!(p["current"].is_number(), "param must have current");
    }
}

#[tokio::test]
async fn handle_set_parameter_success() {
    let (session, model_id) = load_ren_model().await;
    // First get a valid parameter ID
    let list_args = args_with_model_id(&model_id);
    let list_result = mocari::mcp::runtime::handle_list_parameters(&session, list_args)
        .await
        .unwrap();
    let params: Vec<serde_json::Value> = serde_json::from_str(extract_text(&list_result)).unwrap();
    let param_id = params[0]["id"].as_str().unwrap();

    let args = serde_json::json!({
        "model_id": model_id,
        "parameter_id": param_id,
        "value": 0.5
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_parameter(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["success"], true);
}

#[tokio::test]
async fn handle_set_parameter_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "parameter_id": "NonExistentParam",
        "value": 1.0
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_parameter(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn handle_get_parameter_success() {
    let (session, model_id) = load_ren_model().await;
    let list_args = args_with_model_id(&model_id);
    let list_result = mocari::mcp::runtime::handle_list_parameters(&session, list_args)
        .await
        .unwrap();
    let params: Vec<serde_json::Value> = serde_json::from_str(extract_text(&list_result)).unwrap();
    let param_id = params[0]["id"].as_str().unwrap();

    let args = serde_json::json!({
        "model_id": model_id,
        "parameter_id": param_id
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_get_parameter(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(parsed["value"].is_number());
    assert!(parsed["min"].is_number());
    assert!(parsed["max"].is_number());
    assert!(parsed["default"].is_number());
}

#[tokio::test]
async fn handle_get_parameter_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "parameter_id": "NonExistentParam"
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_get_parameter(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

// ===========================================================================
// Drawables
// ===========================================================================

#[tokio::test]
async fn handle_list_drawables_success() {
    let (session, model_id) = load_ren_model().await;
    let args = args_with_model_id(&model_id);
    let result = mocari::mcp::runtime::handle_list_drawables(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(text).unwrap();
    assert!(!parsed.is_empty(), "should have drawables");
    for d in &parsed {
        assert!(d["id"].as_str().is_some(), "drawable must have id");
        assert!(d["visible"].as_bool().is_some(), "drawable must have visible");
        assert!(d["opacity"].is_number(), "drawable must have opacity");
    }
}

#[tokio::test]
async fn handle_set_drawable_visible_success() {
    let (session, model_id) = load_ren_model().await;
    // Get a valid drawable ID
    let list_args = args_with_model_id(&model_id);
    let list_result = mocari::mcp::runtime::handle_list_drawables(&session, list_args)
        .await
        .unwrap();
    let drawables: Vec<serde_json::Value> =
        serde_json::from_str(extract_text(&list_result)).unwrap();
    let drawable_id = drawables[0]["id"].as_str().unwrap();

    let args = serde_json::json!({
        "model_id": model_id,
        "drawable_id": drawable_id,
        "visible": false
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_drawable_visible(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_set_drawable_visible_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "drawable_id": "NonExistentDrawable",
        "visible": true
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_drawable_visible(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn handle_set_drawable_color_multiply() {
    let (session, model_id) = load_ren_model().await;
    let list_args = args_with_model_id(&model_id);
    let list_result = mocari::mcp::runtime::handle_list_drawables(&session, list_args)
        .await
        .unwrap();
    let drawables: Vec<serde_json::Value> =
        serde_json::from_str(extract_text(&list_result)).unwrap();
    let drawable_id = drawables[0]["id"].as_str().unwrap();

    let args = serde_json::json!({
        "model_id": model_id,
        "drawable_id": drawable_id,
        "multiply": [1.0, 0.5, 0.5]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_drawable_color(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_set_drawable_color_screen() {
    let (session, model_id) = load_ren_model().await;
    let list_args = args_with_model_id(&model_id);
    let list_result = mocari::mcp::runtime::handle_list_drawables(&session, list_args)
        .await
        .unwrap();
    let drawables: Vec<serde_json::Value> =
        serde_json::from_str(extract_text(&list_result)).unwrap();
    let drawable_id = drawables[0]["id"].as_str().unwrap();

    let args = serde_json::json!({
        "model_id": model_id,
        "drawable_id": drawable_id,
        "screen": [0.1, 0.1, 0.2]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_drawable_color(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_set_drawable_color_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "drawable_id": "NonExistentDrawable",
        "multiply": [1.0, 1.0, 1.0]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_drawable_color(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

// ===========================================================================
// Motion / Expression
// ===========================================================================

#[tokio::test]
async fn handle_play_motion_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "path": ren_motion_path()
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_play_motion(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["active_count"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn handle_stop_motions_success() {
    let (session, model_id) = load_ren_model().await;
    // Play then stop
    let play_args = serde_json::json!({
        "model_id": model_id,
        "path": ren_motion_path()
    })
    .as_object()
    .unwrap()
    .clone();
    mocari::mcp::runtime::handle_play_motion(&session, play_args)
        .await
        .unwrap();

    let stop_args = args_with_model_id(&model_id);
    let result = mocari::mcp::runtime::handle_stop_motions(&session, stop_args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_play_expression_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "path": ren_expression_path()
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_play_expression(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_stop_expressions_success() {
    let (session, model_id) = load_ren_model().await;
    let play_args = serde_json::json!({
        "model_id": model_id,
        "path": ren_expression_path()
    })
    .as_object()
    .unwrap()
    .clone();
    mocari::mcp::runtime::handle_play_expression(&session, play_args)
        .await
        .unwrap();

    let stop_args = args_with_model_id(&model_id);
    let result = mocari::mcp::runtime::handle_stop_expressions(&session, stop_args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

// ===========================================================================
// Auto-systems
// ===========================================================================

#[tokio::test]
async fn handle_configure_eye_blink_enable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "enabled": true,
        "weight": 0.8
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_eye_blink(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_configure_eye_blink_disable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "enabled": false
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_eye_blink(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_configure_lip_sync_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "amplitude": 0.5,
        "weight": 1.0
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_lip_sync(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_configure_breath_enable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "enabled": true,
        "weight": 0.7
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_breath(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_configure_breath_disable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "enabled": false
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_breath(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_configure_mouse_tracker_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "x": 0.5,
        "y": -0.3,
        "weight": 1.0
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_mouse_tracker(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_configure_physics_disable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "enabled": false
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_physics(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
}

// ===========================================================================
// Tick / State
// ===========================================================================

#[tokio::test]
async fn handle_tick_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({
        "model_id": model_id,
        "delta_seconds": 0.016
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_tick(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["events"].as_array().is_some());
}

#[tokio::test]
async fn handle_get_state_success() {
    let (session, model_id) = load_ren_model().await;
    let args = args_with_model_id(&model_id);
    let result = mocari::mcp::runtime::handle_get_state(&session, args)
        .await
        .unwrap();
    assert!(is_success(&result));
    let text = extract_text(&result);
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert!(parsed["parameters"].as_array().unwrap().len() > 0);
    assert!(parsed["drawable_count"].as_u64().unwrap() > 0);
    assert!(parsed["has_eye_blink"].as_bool().is_some());
    assert!(parsed["has_lip_sync"].as_bool().is_some());
    assert!(parsed["has_breath"].as_bool().is_some());
    assert!(parsed["has_mouse_tracker"].as_bool().is_some());
    assert!(parsed["active_motions"].as_u64().is_some());
}

// ===========================================================================
// Error paths: missing model_id → transport error
// ===========================================================================

#[tokio::test]
async fn load_model_missing_path_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_load_model(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn unload_model_missing_id_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_unload_model(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_parameters_missing_id_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_list_parameters(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn set_parameter_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_set_parameter(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn get_parameter_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_get_parameter(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn list_drawables_missing_id_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_list_drawables(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn set_drawable_visible_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_set_drawable_visible(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn set_drawable_color_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_set_drawable_color(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn play_motion_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_play_motion(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn stop_motions_missing_id_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_stop_motions(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn play_expression_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_play_expression(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn stop_expressions_missing_id_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_stop_expressions(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn configure_eye_blink_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_configure_eye_blink(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn configure_lip_sync_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_configure_lip_sync(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn configure_breath_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_configure_breath(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn configure_mouse_tracker_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_configure_mouse_tracker(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn configure_physics_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_configure_physics(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn tick_missing_args_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_tick(&session, empty_args()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn get_state_missing_id_is_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_get_state(&session, empty_args()).await;
    assert!(result.is_err());
}

// ===========================================================================
// Error paths: invalid model_id → tool error (not transport error)
// ===========================================================================

#[tokio::test]
async fn unload_model_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = args_with_model_id("model_999");
    let result = mocari::mcp::runtime::handle_unload_model(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn list_parameters_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = args_with_model_id("model_999");
    let result = mocari::mcp::runtime::handle_list_parameters(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn set_parameter_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "parameter_id": "ParamX",
        "value": 1.0
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_parameter(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn get_parameter_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "parameter_id": "ParamX"
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_get_parameter(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn list_drawables_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = args_with_model_id("model_999");
    let result = mocari::mcp::runtime::handle_list_drawables(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn set_drawable_visible_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "drawable_id": "X",
        "visible": true
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_drawable_visible(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn set_drawable_color_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "drawable_id": "X",
        "multiply": [1.0, 1.0, 1.0]
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_set_drawable_color(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn play_motion_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "path": "x.motion3.json"
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_play_motion(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn stop_motions_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = args_with_model_id("model_999");
    let result = mocari::mcp::runtime::handle_stop_motions(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn play_expression_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "path": "x.exp3.json"
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_play_expression(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn stop_expressions_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = args_with_model_id("model_999");
    let result = mocari::mcp::runtime::handle_stop_expressions(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn configure_eye_blink_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "enabled": true
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_eye_blink(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn configure_lip_sync_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "amplitude": 0.5
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_lip_sync(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn configure_breath_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "enabled": true
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_breath(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn configure_mouse_tracker_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "x": 0.0,
        "y": 0.0
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_mouse_tracker(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn configure_physics_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "enabled": false
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_configure_physics(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn tick_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({
        "model_id": "model_999",
        "delta_seconds": 0.016
    })
    .as_object()
    .unwrap()
    .clone();
    let result = mocari::mcp::runtime::handle_tick(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn get_state_invalid_id_is_tool_error() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = args_with_model_id("model_999");
    let result = mocari::mcp::runtime::handle_get_state(&session, args)
        .await
        .unwrap();
    assert!(is_error(&result));
}
