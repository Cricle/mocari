use std::sync::Arc;

use rmcp::model::JsonObject;
use tokio::sync::Mutex;

use super::session::ModelSession;
use super::{ToolResult, tool_error, success};

fn get_string(args: &JsonObject, key: &str) -> Result<String, rmcp::ErrorData> {
    args.get(key)
        .and_then(|v| v.as_str().map(String::from))
        .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("missing '{key}'"), None))
}

fn get_number(args: &JsonObject, key: &str) -> Result<f64, rmcp::ErrorData> {
    args.get(key)
        .and_then(|v| v.as_f64())
        .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("missing '{key}'"), None))
}

#[allow(dead_code)] // used by later handler tasks
fn get_bool(args: &JsonObject, key: &str) -> Result<bool, rmcp::ErrorData> {
    args.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("missing '{key}'"), None))
}

// -- Model loading / listing -------------------------------------------------

pub async fn handle_load_model(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let path = get_string(&args, "path")?;
    let mut session = session.lock().await;
    match session.load_model(&path) {
        Ok(id) => {
            let model = session.models.get(&id).unwrap();
            let runtime = model.model.runtime();
            let param_count = runtime.parameter_infos().count();
            let mesh_count = runtime.meshes().len();
            success(format!(
                r#"{{"model_id": "{id}", "parameter_count": {param_count}, "drawable_count": {mesh_count}}}"#
            ))
        }
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_unload_model(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let mut session = session.lock().await;
    if session.unload_model(&id) {
        success(r#"{"success": true}"#)
    } else {
        tool_error(format!("model not found: {id}"))
    }
}

pub async fn handle_list_models(
    session: &Arc<Mutex<ModelSession>>,
    _args: JsonObject,
) -> ToolResult {
    let session = session.lock().await;
    let models: Vec<serde_json::Value> = session
        .models
        .iter()
        .map(|(id, m)| {
            let runtime = m.model.runtime();
            serde_json::json!({
                "model_id": id,
                "path": m.base_path.display().to_string(),
                "parameter_count": runtime.parameter_infos().count(),
                "drawable_count": runtime.meshes().len(),
            })
        })
        .collect();
    success(serde_json::to_string(&models).unwrap_or_else(|_| "[]".into()))
}

// -- Parameter tools ---------------------------------------------------------

pub async fn handle_list_parameters(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let session = session.lock().await;
    match session.with_model(&id, |m| {
        let runtime = m.model.runtime();
        let params: Vec<serde_json::Value> = runtime
            .parameter_infos()
            .map(|p| {
                serde_json::json!({
                    "id": p.id(),
                    "min": p.minimum(),
                    "max": p.maximum(),
                    "default": p.default(),
                    "current": p.value(),
                })
            })
            .collect();
        success(serde_json::to_string(&params).unwrap_or_else(|_| "[]".into()))
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_set_parameter(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let param_id = get_string(&args, "parameter_id")?;
    let value = get_number(&args, "value")? as f32;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();
        if runtime.set_parameter(&param_id, value) {
            let actual = runtime.parameter_value(&param_id).unwrap_or(value);
            success(format!(
                r#"{{"success": true, "actual_value": {actual}}}"#
            ))
        } else {
            tool_error(format!("parameter not found: {param_id}"))
        }
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_get_parameter(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let param_id = get_string(&args, "parameter_id")?;
    let session = session.lock().await;
    match session.with_model(&id, |m| {
        let runtime = m.model.runtime();
        if let Some(info) = runtime.parameter_info(&param_id) {
            success(format!(
                r#"{{"value": {}, "min": {}, "max": {}, "default": {}}}"#,
                info.value(),
                info.minimum(),
                info.maximum(),
                info.default()
            ))
        } else {
            tool_error(format!("parameter not found: {param_id}"))
        }
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
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
