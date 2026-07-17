use std::sync::Arc;

use rmcp::model::{JsonObject, Tool};
use serde_json::json;

fn tool_schema(properties: serde_json::Value, required: &[&str]) -> Arc<JsonObject> {
    Arc::new(
        serde_json::from_value(json!({
            "type": "object",
            "properties": properties,
            "required": required
        }))
        .unwrap(),
    )
}

// --- Runtime tools ---

pub fn load_model_tool() -> Tool {
    Tool::new(
        "load_model",
        "Load a Live2D model from a .model3.json file",
        tool_schema(
            json!({
                "path": { "type": "string", "description": "Path to .model3.json file" }
            }),
            &["path"],
        ),
    )
}

pub fn unload_model_tool() -> Tool {
    Tool::new(
        "unload_model",
        "Unload a previously loaded model",
        tool_schema(
            json!({
                "model_id": { "type": "string", "description": "Model ID returned by load_model" }
            }),
            &["model_id"],
        ),
    )
}

pub fn list_models_tool() -> Tool {
    Tool::new(
        "list_models",
        "List all loaded models",
        tool_schema(json!({}), &[]),
    )
}

pub fn list_parameters_tool() -> Tool {
    Tool::new(
        "list_parameters",
        "List all parameters of a loaded model",
        tool_schema(
            json!({
                "model_id": { "type": "string" }
            }),
            &["model_id"],
        ),
    )
}

pub fn set_parameter_tool() -> Tool {
    Tool::new(
        "set_parameter",
        "Set a model parameter value",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "parameter_id": { "type": "string" },
                "value": { "type": "number" }
            }),
            &["model_id", "parameter_id", "value"],
        ),
    )
}

pub fn get_parameter_tool() -> Tool {
    Tool::new(
        "get_parameter",
        "Get a model parameter's current value and range",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "parameter_id": { "type": "string" }
            }),
            &["model_id", "parameter_id"],
        ),
    )
}

pub fn list_drawables_tool() -> Tool {
    Tool::new(
        "list_drawables",
        "List all drawables (art meshes) of a loaded model",
        tool_schema(
            json!({
                "model_id": { "type": "string" }
            }),
            &["model_id"],
        ),
    )
}

pub fn set_drawable_visible_tool() -> Tool {
    Tool::new(
        "set_drawable_visible",
        "Set visibility of a drawable",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "drawable_id": { "type": "string" },
                "visible": { "type": "boolean" }
            }),
            &["model_id", "drawable_id", "visible"],
        ),
    )
}

pub fn set_drawable_color_tool() -> Tool {
    Tool::new(
        "set_drawable_color",
        "Set multiply and screen color overrides for a drawable",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "drawable_id": { "type": "string" },
                "multiply": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
                "screen": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 }
            }),
            &["model_id", "drawable_id"],
        ),
    )
}

pub fn play_motion_tool() -> Tool {
    Tool::new(
        "play_motion",
        "Play a motion animation on a model",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "path": { "type": "string", "description": "Path to .motion3.json file" },
                "priority": { "type": "string", "enum": ["idle", "normal", "force"] },
                "group": { "type": "string" }
            }),
            &["model_id", "path"],
        ),
    )
}

pub fn stop_motions_tool() -> Tool {
    Tool::new(
        "stop_motions",
        "Stop all playing motions",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "fade_seconds": { "type": "number" }
            }),
            &["model_id"],
        ),
    )
}

pub fn play_expression_tool() -> Tool {
    Tool::new(
        "play_expression",
        "Play an expression on a model",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "path": { "type": "string", "description": "Path to .exp3.json file" }
            }),
            &["model_id", "path"],
        ),
    )
}

pub fn stop_expressions_tool() -> Tool {
    Tool::new(
        "stop_expressions",
        "Stop all playing expressions",
        tool_schema(
            json!({
                "model_id": { "type": "string" }
            }),
            &["model_id"],
        ),
    )
}

pub fn configure_eye_blink_tool() -> Tool {
    Tool::new(
        "configure_eye_blink",
        "Configure automatic eye blinking",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "enabled": { "type": "boolean" },
                "weight": { "type": "number" }
            }),
            &["model_id", "enabled"],
        ),
    )
}

pub fn configure_lip_sync_tool() -> Tool {
    Tool::new(
        "configure_lip_sync",
        "Configure lip sync with audio amplitude",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "amplitude": { "type": "number" },
                "weight": { "type": "number" }
            }),
            &["model_id", "amplitude"],
        ),
    )
}

pub fn configure_breath_tool() -> Tool {
    Tool::new(
        "configure_breath",
        "Configure automatic breathing",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "enabled": { "type": "boolean" },
                "weight": { "type": "number" }
            }),
            &["model_id", "enabled"],
        ),
    )
}

pub fn configure_mouse_tracker_tool() -> Tool {
    Tool::new(
        "configure_mouse_tracker",
        "Set mouse tracking position and weight",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "x": { "type": "number" },
                "y": { "type": "number" },
                "weight": { "type": "number" }
            }),
            &["model_id", "x", "y"],
        ),
    )
}

pub fn configure_physics_tool() -> Tool {
    Tool::new(
        "configure_physics",
        "Enable or disable physics simulation",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "enabled": { "type": "boolean" }
            }),
            &["model_id", "enabled"],
        ),
    )
}

pub fn tick_tool() -> Tool {
    Tool::new(
        "tick",
        "Advance model time by delta_seconds, applying all active systems",
        tool_schema(
            json!({
                "model_id": { "type": "string" },
                "delta_seconds": { "type": "number" }
            }),
            &["model_id", "delta_seconds"],
        ),
    )
}

pub fn get_state_tool() -> Tool {
    Tool::new(
        "get_state",
        "Get a full state snapshot of a model (parameters, drawables, motions)",
        tool_schema(
            json!({
                "model_id": { "type": "string" }
            }),
            &["model_id"],
        ),
    )
}

// --- Creator tools ---

pub fn create_model_json_tool() -> Tool {
    Tool::new(
        "create_model_json",
        "Generate a valid model3.json file",
        tool_schema(
            json!({
                "name": { "type": "string" },
                "moc_path": { "type": "string" },
                "textures": { "type": "array", "items": { "type": "string" } },
                "motions": { "type": "object" },
                "expressions": { "type": "array" }
            }),
            &["name", "moc_path", "textures"],
        ),
    )
}

pub fn create_motion_json_tool() -> Tool {
    Tool::new(
        "create_motion_json",
        "Generate a valid motion3.json file",
        tool_schema(
            json!({
                "duration": { "type": "number" },
                "fps": { "type": "number" },
                "loop": { "type": "boolean" },
                "curves": { "type": "array" }
            }),
            &["duration", "fps", "curves"],
        ),
    )
}

pub fn create_expression_json_tool() -> Tool {
    Tool::new(
        "create_expression_json",
        "Generate a valid exp3.json file",
        tool_schema(
            json!({
                "fade_in": { "type": "number" },
                "fade_out": { "type": "number" },
                "parameters": { "type": "array" }
            }),
            &["parameters"],
        ),
    )
}

pub fn create_physics_json_tool() -> Tool {
    Tool::new(
        "create_physics_json",
        "Generate a valid physics3.json file",
        tool_schema(
            json!({
                "settings": { "type": "object" },
                "physics_info": { "type": "array" }
            }),
            &["settings", "physics_info"],
        ),
    )
}

pub fn create_pose_json_tool() -> Tool {
    Tool::new(
        "create_pose_json",
        "Generate a valid pose3.json file",
        tool_schema(
            json!({
                "fade_in_time": { "type": "number" },
                "groups": { "type": "array" }
            }),
            &["groups"],
        ),
    )
}

pub fn create_userdata_json_tool() -> Tool {
    Tool::new(
        "create_userdata_json",
        "Generate a valid userdata3.json file",
        tool_schema(
            json!({
                "entries": { "type": "array" }
            }),
            &["entries"],
        ),
    )
}

pub fn create_simple_moc3_tool() -> Tool {
    Tool::new(
        "create_simple_moc3",
        "Generate a minimal valid .moc3 binary (placeholder — not yet implemented)",
        tool_schema(
            json!({
                "name": { "type": "string" },
                "width": { "type": "number" },
                "height": { "type": "number" },
                "parameters": { "type": "array" },
                "meshes": { "type": "array" }
            }),
            &["name", "width", "height", "parameters", "meshes"],
        ),
    )
}

pub fn create_model_bundle_tool() -> Tool {
    Tool::new(
        "create_model_bundle",
        "Create a complete model directory with all necessary files",
        tool_schema(
            json!({
                "name": { "type": "string" },
                "description": { "type": "string" },
                "meshes": { "type": "array" },
                "parameters": { "type": "array" },
                "motions": { "type": "array" },
                "expressions": { "type": "array" }
            }),
            &["name", "meshes", "parameters"],
        ),
    )
}

/// Returns all 28 tool definitions.
pub fn all_tools() -> Vec<Tool> {
    vec![
        // Runtime tools
        load_model_tool(),
        unload_model_tool(),
        list_models_tool(),
        list_parameters_tool(),
        set_parameter_tool(),
        get_parameter_tool(),
        list_drawables_tool(),
        set_drawable_visible_tool(),
        set_drawable_color_tool(),
        play_motion_tool(),
        stop_motions_tool(),
        play_expression_tool(),
        stop_expressions_tool(),
        configure_eye_blink_tool(),
        configure_lip_sync_tool(),
        configure_breath_tool(),
        configure_mouse_tracker_tool(),
        configure_physics_tool(),
        tick_tool(),
        get_state_tool(),
        // Creator tools
        create_model_json_tool(),
        create_motion_json_tool(),
        create_expression_json_tool(),
        create_physics_json_tool(),
        create_pose_json_tool(),
        create_userdata_json_tool(),
        create_simple_moc3_tool(),
        create_model_bundle_tool(),
    ]
}
