use std::sync::Arc;

use rmcp::model::{CallToolResult, ContentBlock, JsonObject};
use tokio::sync::Mutex;

use super::session::ModelSession;

type ToolResult = Result<CallToolResult, rmcp::ErrorData>;

fn tool_error(msg: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg)]))
}

#[allow(dead_code)]
fn get_string(args: &JsonObject, key: &str) -> Result<String, rmcp::ErrorData> {
    args.get(key)
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("missing '{key}'"), None))
}

#[allow(dead_code)]
fn get_number(args: &JsonObject, key: &str) -> Result<f64, rmcp::ErrorData> {
    args.get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("missing '{key}'"), None))
}

#[allow(dead_code)]
fn get_bool(args: &JsonObject, key: &str) -> Result<bool, rmcp::ErrorData> {
    args.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("missing '{key}'"), None))
}

// Stub handlers -- return "not yet implemented"

pub async fn handle_load_model(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_unload_model(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_list_models(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_list_parameters(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_set_parameter(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_get_parameter(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_list_drawables(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_set_drawable_visible(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_set_drawable_color(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_play_motion(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_stop_motions(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_play_expression(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_stop_expressions(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_configure_eye_blink(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_configure_lip_sync(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_configure_breath(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_configure_mouse_tracker(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_configure_physics(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_tick(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_get_state(
    _session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    tool_error("not yet implemented")
}
