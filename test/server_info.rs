use mocari::mcp::MocariMcpServer;
use mocari::mcp::ModelSession;
use mocari::mcp::tools::all_tools;
use rmcp::handler::server::ServerHandler;

#[test]
fn server_name_is_mocari_mcp() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    assert_eq!(info.server_info.name, "mocari-mcp");
}

#[test]
fn instructions_mention_live2d() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    let instructions = info.instructions.unwrap_or_default();
    assert!(
        instructions.contains("Live2D"),
        "instructions should mention Live2D, got: {instructions}"
    );
}

#[test]
fn instructions_mention_load_model() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    let instructions = info.instructions.unwrap_or_default();
    assert!(
        instructions.contains("load_model"),
        "instructions should mention load_model, got: {instructions}"
    );
}

#[test]
fn capabilities_declare_tools() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    assert!(
        info.capabilities.tools.is_some(),
        "server must declare tools capability"
    );
}

#[test]
fn all_tools_have_nonempty_names() {
    let tools = all_tools();
    assert_eq!(tools.len(), 28, "expected 28 tools");
    for tool in &tools {
        assert!(
            !tool.name.is_empty(),
            "tool name must not be empty"
        );
    }
}

#[test]
fn all_tools_have_nonempty_descriptions() {
    let tools = all_tools();
    for tool in &tools {
        let desc = tool.description.as_deref().unwrap_or("");
        assert!(
            !desc.is_empty(),
            "tool '{}' must have a non-empty description",
            tool.name
        );
    }
}

#[test]
fn all_tools_have_valid_json_schemas() {
    let tools = all_tools();
    for tool in &tools {
        let schema = &tool.input_schema;
        // Schema must be a valid JSON object
        assert!(
            schema.contains_key("type"),
            "tool '{}' schema must have 'type' field",
            tool.name
        );
        assert_eq!(
            schema.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "tool '{}' schema type must be 'object'",
            tool.name
        );
        // Must have properties
        assert!(
            schema.contains_key("properties"),
            "tool '{}' schema must have 'properties' field",
            tool.name
        );
    }
}

#[test]
fn tool_count_is_28() {
    let tools = all_tools();
    assert_eq!(tools.len(), 28);
}
