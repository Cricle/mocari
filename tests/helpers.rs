use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use mocari::mcp::session::ModelSession;
use rmcp::model::{CallToolResult, JsonObject};

pub fn empty_args() -> JsonObject {
    serde_json::json!({}).as_object().unwrap().clone()
}

pub fn args_with_model_id(model_id: &str) -> JsonObject {
    serde_json::json!({ "model_id": model_id })
        .as_object()
        .unwrap()
        .clone()
}

pub fn ren_model_path() -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/models/Ren/Ren.model3.json")
        .display()
        .to_string()
}

pub fn extract_text(result: &CallToolResult) -> &str {
    result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("")
}

pub fn is_success(result: &CallToolResult) -> bool {
    result.is_error == Some(false)
}

pub fn is_error(result: &CallToolResult) -> bool {
    result.is_error == Some(true)
}

pub async fn load_ren_model() -> (Arc<Mutex<ModelSession>>, String) {
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
