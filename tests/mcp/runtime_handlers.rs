use std::sync::Arc;

use tokio::sync::Mutex;

use mocari::mcp::session::ModelSession;

use super::helpers::{
    empty_args, args_with_model_id, ren_model_path, extract_text, is_success, is_error,
    load_ren_model,
};

fn ren_motion_path() -> String {
    "motions/mtn_01.motion3.json".to_string()
}

fn ren_expression_path() -> String {
    "expressions/exp_01.exp3.json".to_string()
}

// ===========================================================================
// Model lifecycle
// ===========================================================================

#[tokio::test]
async fn handle_load_model_success() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({ "path": ren_model_path() }).as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_load_model(&session, args).await.unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert!(parsed["model_id"].as_str().is_some());
    assert!(parsed["parameter_count"].as_u64().unwrap() > 0);
    assert!(parsed["drawable_count"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn handle_load_model_nonexistent() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let args = serde_json::json!({ "path": "/nonexistent/model.model3.json" }).as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_load_model(&session, args).await.unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn handle_list_models_shows_loaded() {
    let (session, model_id) = load_ren_model().await;
    let result = mocari::mcp::runtime::handle_list_models(&session, empty_args()).await.unwrap();
    assert!(is_success(&result));
    let parsed: Vec<serde_json::Value> = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed.len(), 1);
    assert_eq!(parsed[0]["model_id"].as_str().unwrap(), model_id);
}

#[tokio::test]
async fn handle_list_models_empty() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_list_models(&session, empty_args()).await.unwrap();
    assert!(is_success(&result));
    assert_eq!(extract_text(&result), "[]");
}

#[tokio::test]
async fn handle_unload_model_success() {
    let (session, model_id) = load_ren_model().await;
    let result = mocari::mcp::runtime::handle_unload_model(&session, args_with_model_id(&model_id)).await.unwrap();
    assert!(is_success(&result));
    let list = mocari::mcp::runtime::handle_list_models(&session, empty_args()).await.unwrap();
    assert_eq!(extract_text(&list), "[]");
}

#[tokio::test]
async fn handle_unload_model_nonexistent() {
    let session = Arc::new(Mutex::new(ModelSession::new()));
    let result = mocari::mcp::runtime::handle_unload_model(&session, args_with_model_id("model_999")).await.unwrap();
    assert!(is_error(&result));
}

// ===========================================================================
// Parameters
// ===========================================================================

#[tokio::test]
async fn handle_list_parameters_success() {
    let (session, model_id) = load_ren_model().await;
    let result = mocari::mcp::runtime::handle_list_parameters(&session, args_with_model_id(&model_id)).await.unwrap();
    assert!(is_success(&result));
    let parsed: Vec<serde_json::Value> = serde_json::from_str(extract_text(&result)).unwrap();
    assert!(!parsed.is_empty());
    for p in &parsed {
        assert!(p["id"].as_str().is_some());
        assert!(p["min"].is_number());
        assert!(p["max"].is_number());
        assert!(p["default"].is_number());
        assert!(p["current"].is_number());
    }
}

#[tokio::test]
async fn handle_set_parameter_success() {
    let (session, model_id) = load_ren_model().await;
    let params: Vec<serde_json::Value> = serde_json::from_str(
        extract_text(&mocari::mcp::runtime::handle_list_parameters(&session, args_with_model_id(&model_id)).await.unwrap())
    ).unwrap();
    let param_id = params[0]["id"].as_str().unwrap();
    let args = serde_json::json!({ "model_id": model_id, "parameter_id": param_id, "value": 0.5 })
        .as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_set_parameter(&session, args).await.unwrap();
    assert!(is_success(&result));
}

#[tokio::test]
async fn handle_set_parameter_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "parameter_id": "NonExistent", "value": 1.0 })
        .as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_set_parameter(&session, args).await.unwrap();
    assert!(is_error(&result));
}

#[tokio::test]
async fn handle_get_parameter_success() {
    let (session, model_id) = load_ren_model().await;
    let params: Vec<serde_json::Value> = serde_json::from_str(
        extract_text(&mocari::mcp::runtime::handle_list_parameters(&session, args_with_model_id(&model_id)).await.unwrap())
    ).unwrap();
    let param_id = params[0]["id"].as_str().unwrap();
    let args = serde_json::json!({ "model_id": model_id, "parameter_id": param_id })
        .as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_get_parameter(&session, args).await.unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert!(parsed["value"].is_number());
    assert!(parsed["min"].is_number());
}

#[tokio::test]
async fn handle_get_parameter_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "parameter_id": "NonExistent" })
        .as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_get_parameter(&session, args).await.unwrap();
    assert!(is_error(&result));
}

// ===========================================================================
// Drawables
// ===========================================================================

#[tokio::test]
async fn handle_list_drawables_success() {
    let (session, model_id) = load_ren_model().await;
    let result = mocari::mcp::runtime::handle_list_drawables(&session, args_with_model_id(&model_id)).await.unwrap();
    assert!(is_success(&result));
    let parsed: Vec<serde_json::Value> = serde_json::from_str(extract_text(&result)).unwrap();
    assert!(!parsed.is_empty());
    for d in &parsed {
        assert!(d["id"].as_str().is_some());
        assert!(d["visible"].as_bool().is_some());
        assert!(d["opacity"].is_number());
    }
}

#[tokio::test]
async fn handle_set_drawable_visible_success() {
    let (session, model_id) = load_ren_model().await;
    let drawables: Vec<serde_json::Value> = serde_json::from_str(
        extract_text(&mocari::mcp::runtime::handle_list_drawables(&session, args_with_model_id(&model_id)).await.unwrap())
    ).unwrap();
    let drawable_id = drawables[0]["id"].as_str().unwrap();
    let args = serde_json::json!({ "model_id": model_id, "drawable_id": drawable_id, "visible": false })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_set_drawable_visible(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_set_drawable_visible_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "drawable_id": "NonExistent", "visible": true })
        .as_object().unwrap().clone();
    assert!(is_error(&mocari::mcp::runtime::handle_set_drawable_visible(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_set_drawable_color_multiply() {
    let (session, model_id) = load_ren_model().await;
    let drawables: Vec<serde_json::Value> = serde_json::from_str(
        extract_text(&mocari::mcp::runtime::handle_list_drawables(&session, args_with_model_id(&model_id)).await.unwrap())
    ).unwrap();
    let drawable_id = drawables[0]["id"].as_str().unwrap();
    let args = serde_json::json!({ "model_id": model_id, "drawable_id": drawable_id, "multiply": [1.0, 0.5, 0.5] })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_set_drawable_color(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_set_drawable_color_screen() {
    let (session, model_id) = load_ren_model().await;
    let drawables: Vec<serde_json::Value> = serde_json::from_str(
        extract_text(&mocari::mcp::runtime::handle_list_drawables(&session, args_with_model_id(&model_id)).await.unwrap())
    ).unwrap();
    let drawable_id = drawables[0]["id"].as_str().unwrap();
    let args = serde_json::json!({ "model_id": model_id, "drawable_id": drawable_id, "screen": [0.1, 0.1, 0.2] })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_set_drawable_color(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_set_drawable_color_invalid_id() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "drawable_id": "NonExistent", "multiply": [1.0, 1.0, 1.0] })
        .as_object().unwrap().clone();
    assert!(is_error(&mocari::mcp::runtime::handle_set_drawable_color(&session, args).await.unwrap()));
}

// ===========================================================================
// Motion / Expression
// ===========================================================================

#[tokio::test]
async fn handle_play_motion_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "path": ren_motion_path() })
        .as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_play_motion(&session, args).await.unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["active_count"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn handle_stop_motions_success() {
    let (session, model_id) = load_ren_model().await;
    let play_args = serde_json::json!({ "model_id": model_id, "path": ren_motion_path() })
        .as_object().unwrap().clone();
    mocari::mcp::runtime::handle_play_motion(&session, play_args).await.unwrap();
    assert!(is_success(&mocari::mcp::runtime::handle_stop_motions(&session, args_with_model_id(&model_id)).await.unwrap()));
}

#[tokio::test]
async fn handle_play_expression_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "path": ren_expression_path() })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_play_expression(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_stop_expressions_success() {
    let (session, model_id) = load_ren_model().await;
    let play_args = serde_json::json!({ "model_id": model_id, "path": ren_expression_path() })
        .as_object().unwrap().clone();
    mocari::mcp::runtime::handle_play_expression(&session, play_args).await.unwrap();
    assert!(is_success(&mocari::mcp::runtime::handle_stop_expressions(&session, args_with_model_id(&model_id)).await.unwrap()));
}

// ===========================================================================
// Auto-systems
// ===========================================================================

#[tokio::test]
async fn handle_configure_eye_blink_enable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "enabled": true, "weight": 0.8 })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_configure_eye_blink(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_configure_eye_blink_disable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "enabled": false })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_configure_eye_blink(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_configure_lip_sync_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "amplitude": 0.5, "weight": 1.0 })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_configure_lip_sync(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_configure_breath_enable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "enabled": true, "weight": 0.7 })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_configure_breath(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_configure_breath_disable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "enabled": false })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_configure_breath(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_configure_mouse_tracker_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "x": 0.5, "y": -0.3, "weight": 1.0 })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_configure_mouse_tracker(&session, args).await.unwrap()));
}

#[tokio::test]
async fn handle_configure_physics_disable() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "enabled": false })
        .as_object().unwrap().clone();
    assert!(is_success(&mocari::mcp::runtime::handle_configure_physics(&session, args).await.unwrap()));
}

// ===========================================================================
// Tick / State
// ===========================================================================

#[tokio::test]
async fn handle_tick_success() {
    let (session, model_id) = load_ren_model().await;
    let args = serde_json::json!({ "model_id": model_id, "delta_seconds": 0.016 })
        .as_object().unwrap().clone();
    let result = mocari::mcp::runtime::handle_tick(&session, args).await.unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert_eq!(parsed["success"], true);
    assert!(parsed["events"].as_array().is_some());
}

#[tokio::test]
async fn handle_get_state_success() {
    let (session, model_id) = load_ren_model().await;
    let result = mocari::mcp::runtime::handle_get_state(&session, args_with_model_id(&model_id)).await.unwrap();
    assert!(is_success(&result));
    let parsed: serde_json::Value = serde_json::from_str(extract_text(&result)).unwrap();
    assert!(!parsed["parameters"].as_array().unwrap().is_empty());
    assert!(parsed["drawable_count"].as_u64().unwrap() > 0);
}

// ===========================================================================
// Error paths: missing args → transport error
// ===========================================================================

macro_rules! test_missing_args {
    ($name:ident, $handler:path) => {
        #[tokio::test]
        async fn $name() {
            let session = Arc::new(Mutex::new(ModelSession::new()));
            assert!($handler(&session, empty_args()).await.is_err());
        }
    };
}

test_missing_args!(load_model_missing_path_is_error, mocari::mcp::runtime::handle_load_model);
test_missing_args!(unload_model_missing_id_is_error, mocari::mcp::runtime::handle_unload_model);
test_missing_args!(list_parameters_missing_id_is_error, mocari::mcp::runtime::handle_list_parameters);
test_missing_args!(set_parameter_missing_args_is_error, mocari::mcp::runtime::handle_set_parameter);
test_missing_args!(get_parameter_missing_args_is_error, mocari::mcp::runtime::handle_get_parameter);
test_missing_args!(list_drawables_missing_id_is_error, mocari::mcp::runtime::handle_list_drawables);
test_missing_args!(set_drawable_visible_missing_args_is_error, mocari::mcp::runtime::handle_set_drawable_visible);
test_missing_args!(set_drawable_color_missing_args_is_error, mocari::mcp::runtime::handle_set_drawable_color);
test_missing_args!(play_motion_missing_args_is_error, mocari::mcp::runtime::handle_play_motion);
test_missing_args!(stop_motions_missing_id_is_error, mocari::mcp::runtime::handle_stop_motions);
test_missing_args!(play_expression_missing_args_is_error, mocari::mcp::runtime::handle_play_expression);
test_missing_args!(stop_expressions_missing_id_is_error, mocari::mcp::runtime::handle_stop_expressions);
test_missing_args!(configure_eye_blink_missing_args_is_error, mocari::mcp::runtime::handle_configure_eye_blink);
test_missing_args!(configure_lip_sync_missing_args_is_error, mocari::mcp::runtime::handle_configure_lip_sync);
test_missing_args!(configure_breath_missing_args_is_error, mocari::mcp::runtime::handle_configure_breath);
test_missing_args!(configure_mouse_tracker_missing_args_is_error, mocari::mcp::runtime::handle_configure_mouse_tracker);
test_missing_args!(configure_physics_missing_args_is_error, mocari::mcp::runtime::handle_configure_physics);
test_missing_args!(tick_missing_args_is_error, mocari::mcp::runtime::handle_tick);
test_missing_args!(get_state_missing_id_is_error, mocari::mcp::runtime::handle_get_state);

// ===========================================================================
// Error paths: invalid model_id → tool error
// ===========================================================================

macro_rules! test_invalid_model_id {
    ($name:ident, $handler:path, $args:expr) => {
        #[tokio::test]
        async fn $name() {
            let session = Arc::new(Mutex::new(ModelSession::new()));
            let result = $handler(&session, $args).await.unwrap();
            assert!(is_error(&result));
        }
    };
}

test_invalid_model_id!(unload_model_invalid_id_is_tool_error, mocari::mcp::runtime::handle_unload_model, args_with_model_id("model_999"));
test_invalid_model_id!(list_parameters_invalid_id_is_tool_error, mocari::mcp::runtime::handle_list_parameters, args_with_model_id("model_999"));
test_invalid_model_id!(set_parameter_invalid_id_is_tool_error, mocari::mcp::runtime::handle_set_parameter, serde_json::json!({"model_id":"m","parameter_id":"p","value":1.0}).as_object().unwrap().clone());
test_invalid_model_id!(get_parameter_invalid_id_is_tool_error, mocari::mcp::runtime::handle_get_parameter, serde_json::json!({"model_id":"m","parameter_id":"p"}).as_object().unwrap().clone());
test_invalid_model_id!(list_drawables_invalid_id_is_tool_error, mocari::mcp::runtime::handle_list_drawables, args_with_model_id("model_999"));
test_invalid_model_id!(set_drawable_visible_invalid_id_is_tool_error, mocari::mcp::runtime::handle_set_drawable_visible, serde_json::json!({"model_id":"m","drawable_id":"d","visible":true}).as_object().unwrap().clone());
test_invalid_model_id!(set_drawable_color_invalid_id_is_tool_error, mocari::mcp::runtime::handle_set_drawable_color, serde_json::json!({"model_id":"m","drawable_id":"d","multiply":[1.0,1.0,1.0]}).as_object().unwrap().clone());
test_invalid_model_id!(play_motion_invalid_id_is_tool_error, mocari::mcp::runtime::handle_play_motion, serde_json::json!({"model_id":"m","path":"x.motion3.json"}).as_object().unwrap().clone());
test_invalid_model_id!(stop_motions_invalid_id_is_tool_error, mocari::mcp::runtime::handle_stop_motions, args_with_model_id("model_999"));
test_invalid_model_id!(play_expression_invalid_id_is_tool_error, mocari::mcp::runtime::handle_play_expression, serde_json::json!({"model_id":"m","path":"x.exp3.json"}).as_object().unwrap().clone());
test_invalid_model_id!(stop_expressions_invalid_id_is_tool_error, mocari::mcp::runtime::handle_stop_expressions, args_with_model_id("model_999"));
test_invalid_model_id!(configure_eye_blink_invalid_id_is_tool_error, mocari::mcp::runtime::handle_configure_eye_blink, serde_json::json!({"model_id":"m","enabled":true}).as_object().unwrap().clone());
test_invalid_model_id!(configure_lip_sync_invalid_id_is_tool_error, mocari::mcp::runtime::handle_configure_lip_sync, serde_json::json!({"model_id":"m","amplitude":0.5}).as_object().unwrap().clone());
test_invalid_model_id!(configure_breath_invalid_id_is_tool_error, mocari::mcp::runtime::handle_configure_breath, serde_json::json!({"model_id":"m","enabled":true}).as_object().unwrap().clone());
test_invalid_model_id!(configure_mouse_tracker_invalid_id_is_tool_error, mocari::mcp::runtime::handle_configure_mouse_tracker, serde_json::json!({"model_id":"m","x":0.0,"y":0.0}).as_object().unwrap().clone());
test_invalid_model_id!(configure_physics_invalid_id_is_tool_error, mocari::mcp::runtime::handle_configure_physics, serde_json::json!({"model_id":"m","enabled":false}).as_object().unwrap().clone());
test_invalid_model_id!(tick_invalid_id_is_tool_error, mocari::mcp::runtime::handle_tick, serde_json::json!({"model_id":"m","delta_seconds":0.016}).as_object().unwrap().clone());
test_invalid_model_id!(get_state_invalid_id_is_tool_error, mocari::mcp::runtime::handle_get_state, args_with_model_id("model_999"));
