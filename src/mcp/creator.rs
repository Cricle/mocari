use rmcp::model::{CallToolResult, ContentBlock, JsonObject};

type ToolResult = Result<CallToolResult, rmcp::ErrorData>;

fn tool_error(msg: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg)]))
}

// Stub handlers -- return "not yet implemented"

pub async fn handle_create_model_json(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_create_motion_json(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_create_expression_json(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_create_physics_json(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_create_pose_json(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_create_userdata_json(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_create_simple_moc3(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}

pub async fn handle_create_model_bundle(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}
