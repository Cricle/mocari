use rmcp::model::JsonObject;

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

// -- JSON file generators ---------------------------------------------------

pub async fn handle_create_model_json(args: JsonObject) -> ToolResult {
    let _name = get_string(&args, "name")?;
    let moc_path = get_string(&args, "moc_path")?;
    let textures = args.get("textures")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'textures'", None))?;
    let motions = args.get("motions").cloned().unwrap_or(serde_json::json!({}));
    let expressions = args.get("expressions").cloned().unwrap_or(serde_json::json!([]));

    let model = serde_json::json!({
        "Version": 3,
        "FileReferences": {
            "Moc": moc_path,
            "Textures": textures,
            "Motions": motions,
            "Expressions": expressions,
        },
        "Groups": []
    });
    success(serde_json::to_string_pretty(&model).unwrap_or_else(|_| "{}".into()))
}

pub async fn handle_create_motion_json(args: JsonObject) -> ToolResult {
    let duration = get_number(&args, "duration")?;
    let fps = get_number(&args, "fps")?;
    let loop_flag = args.get("loop").and_then(|v| v.as_bool()).unwrap_or(false);
    let curves = args.get("curves")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'curves'", None))?;

    let motion = serde_json::json!({
        "Version": 3,
        "Meta": {
            "Duration": duration,
            "Fps": fps,
            "Loop": loop_flag,
            "CurveCount": curves.len(),
            "TotalSegmentCount": 0,
            "TotalPointCount": 0,
        },
        "Curves": curves
    });
    success(serde_json::to_string_pretty(&motion).unwrap_or_else(|_| "{}".into()))
}

pub async fn handle_create_expression_json(args: JsonObject) -> ToolResult {
    let fade_in = args.get("fade_in").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let fade_out = args.get("fade_out").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let params = args.get("parameters")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'parameters'", None))?;

    let expr = serde_json::json!({
        "Type": "Additive",
        "FadeInTime": fade_in,
        "FadeOutTime": fade_out,
        "Parameters": params
    });
    success(serde_json::to_string_pretty(&expr).unwrap_or_else(|_| "{}".into()))
}

pub async fn handle_create_physics_json(args: JsonObject) -> ToolResult {
    let settings = args.get("settings")
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'settings'", None))?;
    let physics_info = args.get("physics_info")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'physics_info'", None))?;

    let physics = serde_json::json!({
        "Version": 3,
        "Meta": {
            "PhysicsSettingCount": physics_info.len(),
            "TotalInputCount": 0,
            "TotalOutputCount": 0,
            "VertexCount": 0,
            "EffectiveForces": settings,
        },
        "PhysicsSettings": physics_info
    });
    success(serde_json::to_string_pretty(&physics).unwrap_or_else(|_| "{}".into()))
}

pub async fn handle_create_pose_json(args: JsonObject) -> ToolResult {
    let fade_in = args.get("fade_in_time").and_then(|v| v.as_f64()).unwrap_or(0.5);
    let groups = args.get("groups")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'groups'", None))?;

    let pose = serde_json::json!({
        "Type": "Live2D Pose",
        "FadeInTime": fade_in,
        "Groups": groups
    });
    success(serde_json::to_string_pretty(&pose).unwrap_or_else(|_| "{}".into()))
}

pub async fn handle_create_userdata_json(args: JsonObject) -> ToolResult {
    let entries = args.get("entries")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'entries'", None))?;

    let userdata = serde_json::json!({
        "Version": 3,
        "Meta": {
            "UserDataCount": entries.len(),
        },
        "UserData": entries
    });
    success(serde_json::to_string_pretty(&userdata).unwrap_or_else(|_| "{}".into()))
}

pub async fn handle_create_simple_moc3(args: JsonObject) -> ToolResult {
    let _name = get_string(&args, "name")?;
    let _width = get_number(&args, "width")?;
    let _height = get_number(&args, "height")?;
    let _parameters = args.get("parameters")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'parameters'", None))?;
    let _meshes = args.get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'meshes'", None))?;

    // moc3 binary generation requires full Cubism SDK format implementation
    tool_error("create_simple_moc3: moc3 binary generation requires full Cubism SDK format implementation — use create_model_json to generate the JSON sidecar instead")
}

pub async fn handle_create_model_bundle(args: JsonObject) -> ToolResult {
    let name = get_string(&args, "name")?;
    let _description = args.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let _meshes = args.get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'meshes'", None))?;
    let _parameters = args.get("parameters")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'parameters'", None))?;
    let motions = args.get("motions").cloned().unwrap_or(serde_json::json!([]));
    let expressions = args.get("expressions").cloned().unwrap_or(serde_json::json!([]));

    // Generate the model3.json content
    let model_json = serde_json::json!({
        "Version": 3,
        "FileReferences": {
            "Moc": format!("{name}.moc3"),
            "Textures": [format!("{name}.png")],
            "Motions": {
                "": motions
            },
            "Expressions": expressions,
        },
        "Groups": []
    });

    let json_str = serde_json::to_string_pretty(&model_json)
        .unwrap_or_else(|_| "{}".into());

    // For bundle creation, we'd write files to disk. Since MCP is headless,
    // return the file contents and let the client write them.
    let files = vec![
        format!("{name}.model3.json"),
        format!("{name}.png"),
        format!("{name}.moc3"),
    ];

    success(format!(
        r#"{{"model_json": {}, "files": {}}}"#,
        serde_json::to_string(&json_str).unwrap_or_else(|_| "\"\"".into()),
        serde_json::to_string(&files).unwrap_or_else(|_| "[]".into())
    ))
}
