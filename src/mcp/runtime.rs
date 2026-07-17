use std::sync::Arc;

use rmcp::model::JsonObject;
use tokio::sync::Mutex;

use super::session::ModelSession;
use super::{ToolResult, tool_error, success, get_string, get_number, get_bool};

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
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let session = session.lock().await;
    match session.with_model(&id, |m| {
        let runtime = m.model.runtime();
        let drawables: Vec<serde_json::Value> = runtime
            .meshes()
            .iter()
            .enumerate()
            .map(|(i, mesh)| {
                serde_json::json!({
                    "id": runtime.drawable_ids().get(i).map(|s| s.as_str()).unwrap_or("unknown"),
                    "visible": runtime.is_drawable_visible(i),
                    "opacity": mesh.opacity(),
                })
            })
            .collect();
        success(serde_json::to_string(&drawables).unwrap_or_else(|_| "[]".into()))
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_set_drawable_visible(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let drawable_id = get_string(&args, "drawable_id")?;
    let visible = get_bool(&args, "visible")?;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();
        if runtime.set_drawable_visible(&drawable_id, visible) {
            success(r#"{"success": true}"#)
        } else {
            tool_error(format!("drawable not found: {drawable_id}"))
        }
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_set_drawable_color(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let drawable_id = get_string(&args, "drawable_id")?;
    let multiply = args.get("multiply").and_then(|v| {
        let arr = v.as_array()?;
        if arr.len() == 3 {
            Some([arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32])
        } else {
            None
        }
    });
    let screen = args.get("screen").and_then(|v| {
        let arr = v.as_array()?;
        if arr.len() == 3 {
            Some([arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32])
        } else {
            None
        }
    });
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();
        let mut any_set = false;
        if let Some(color) = multiply {
            any_set |= runtime.set_drawable_multiply_color(&drawable_id, color);
        }
        if let Some(color) = screen {
            any_set |= runtime.set_drawable_screen_color(&drawable_id, color);
        }
        if any_set {
            success(r#"{"success": true}"#)
        } else if multiply.is_none() && screen.is_none() {
            tool_error("provide 'multiply' and/or 'screen' color arrays")
        } else {
            tool_error(format!("drawable not found: {drawable_id}"))
        }
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_play_motion(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let path = get_string(&args, "path")?;
    let priority = args.get("priority").and_then(|v| v.as_str()).unwrap_or("normal");
    let group = args.get("group").and_then(|v| v.as_str()).unwrap_or("");
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        let full_path = m.base_path.join(&path);
        match crate::motion::load_motion(&full_path) {
            Ok(motion) => {
                let pri = match priority {
                    "idle" => crate::motion::MotionPriority::Idle,
                    "force" => crate::motion::MotionPriority::Force,
                    _ => crate::motion::MotionPriority::Normal,
                };
                m.motion_manager.start_motion(motion, pri, group);
                let count = m.motion_manager.active_count();
                success(format!(r#"{{"success": true, "active_count": {count}}}"#))
            }
            Err(e) => tool_error(format!("failed to load motion: {e}")),
        }
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_stop_motions(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        m.motion_manager.stop_all();
        success(r#"{"success": true}"#)
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_play_expression(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let path = get_string(&args, "path")?;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        let full_path = m.base_path.join(&path);
        match crate::expression::load_expression(&full_path) {
            Ok(expr) => {
                m.expression_manager.play(expr);
                success(r#"{"success": true}"#)
            }
            Err(e) => tool_error(format!("failed to load expression: {e}")),
        }
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_stop_expressions(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        m.expression_manager.stop_all();
        success(r#"{"success": true}"#)
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_configure_eye_blink(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let enabled = get_bool(&args, "enabled")?;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        if enabled {
            if m.eye_blink.is_none() {
                let cfg = m.model.runtime().eye_blink_config_from_model();
                m.eye_blink = Some(crate::auto::EyeBlink::new(cfg));
            }
            if let Some(ref mut eb) = m.eye_blink {
                eb.set_weight(weight);
            }
        } else {
            m.eye_blink = None;
        }
        success(r#"{"success": true}"#)
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_configure_lip_sync(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let amplitude = get_number(&args, "amplitude")? as f32;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        if m.lip_sync.is_none() {
            let cfg = m.model.runtime().lip_sync_config_from_model();
            m.lip_sync = Some(crate::auto::LipSync::new(cfg));
        }
        if let Some(ref mut ls) = m.lip_sync {
            ls.set_amplitude(amplitude);
            ls.set_weight(weight);
        }
        success(r#"{"success": true}"#)
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_configure_breath(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let enabled = get_bool(&args, "enabled")?;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        if enabled {
            if m.breath.is_none() {
                m.breath = Some(crate::auto::Breath::new(Default::default()));
            }
            if let Some(ref mut b) = m.breath {
                b.set_weight(weight);
            }
        } else {
            m.breath = None;
        }
        success(r#"{"success": true}"#)
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_configure_mouse_tracker(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let x = get_number(&args, "x")? as f32;
    let y = get_number(&args, "y")? as f32;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        if m.mouse_tracker.is_none() {
            m.mouse_tracker = Some(crate::auto::MouseTracker::new(Default::default()));
        }
        if let Some(ref mut mt) = m.mouse_tracker {
            mt.set_target(x, y);
            mt.set_weight(weight);
        }
        success(r#"{"success": true}"#)
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_configure_physics(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let enabled = get_bool(&args, "enabled")?;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        if enabled {
            tool_error("physics re-enablement requires model reload; only disabling is supported")
        } else {
            m.model.runtime_mut().clear_physics();
            success(r#"{"success": true}"#)
        }
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_tick(
    session: &Arc<Mutex<ModelSession>>,
    args: JsonObject,
) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let delta = get_number(&args, "delta_seconds")? as f32;
    let mut session = session.lock().await;
    match session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();

        // Auto-systems: tick advances internal state, apply writes params
        if let Some(ref mut eb) = m.eye_blink {
            eb.tick(delta);
            eb.apply(runtime);
        }
        if let Some(ref mut ls) = m.lip_sync {
            ls.tick(delta);
            ls.apply(runtime);
        }
        if let Some(ref mut b) = m.breath {
            b.tick(delta);
            b.apply(runtime);
        }
        if let Some(ref mut mt) = m.mouse_tracker {
            mt.tick(delta);
            mt.apply(runtime);
        }

        // Motion: tick advances time, apply writes parameters
        m.motion_manager.tick(delta);
        m.motion_manager.apply(runtime);
        let motion_events = m.motion_manager.drain_events();

        // Expression: tick advances fades, apply writes parameters
        m.expression_manager.tick(delta);
        m.expression_manager.apply(runtime);

        // Meshes: rebuild from current parameter state
        runtime.update_meshes();

        let events_json = serde_json::to_string(&motion_events).unwrap_or_else(|_| "[]".into());
        success(format!(r#"{{"success": true, "events": {events_json}}}"#))
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}

pub async fn handle_get_state(
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
                    "value": p.value(),
                    "min": p.minimum(),
                    "max": p.maximum(),
                    "default": p.default(),
                })
            })
            .collect();
        let state = serde_json::json!({
            "parameters": params,
            "drawable_count": runtime.meshes().len(),
            "has_eye_blink": m.eye_blink.is_some(),
            "has_lip_sync": m.lip_sync.is_some(),
            "has_breath": m.breath.is_some(),
            "has_mouse_tracker": m.mouse_tracker.is_some(),
            "active_motions": m.motion_manager.active_count(),
        });
        success(serde_json::to_string_pretty(&state).unwrap_or_else(|_| "{}".into()))
    }) {
        Ok(result) => result,
        Err(e) => tool_error(e.to_string()),
    }
}
