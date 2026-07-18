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
    assert!(instructions.contains("Live2D"));
}

#[test]
fn instructions_mention_load_model() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    let instructions = info.instructions.unwrap_or_default();
    assert!(instructions.contains("load_model"));
}

#[test]
fn capabilities_declare_tools() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    assert!(info.capabilities.tools.is_some());
}

#[test]
fn all_tools_have_nonempty_names() {
    let tools = all_tools();
    assert_eq!(tools.len(), 28);
    for tool in &tools {
        assert!(!tool.name.is_empty());
    }
}

#[test]
fn all_tools_have_nonempty_descriptions() {
    for tool in &all_tools() {
        let desc = tool.description.as_deref().unwrap_or("");
        assert!(!desc.is_empty(), "tool '{}' missing description", tool.name);
    }
}

#[test]
fn all_tools_have_valid_json_schemas() {
    for tool in &all_tools() {
        let schema = &tool.input_schema;
        assert_eq!(
            schema.get("type").and_then(|v| v.as_str()),
            Some("object"),
            "tool '{}' schema type must be 'object'",
            tool.name
        );
        assert!(
            schema.contains_key("properties"),
            "tool '{}' schema missing 'properties'",
            tool.name
        );
    }
}

#[test]
fn tool_count_is_28() {
    assert_eq!(all_tools().len(), 28);
}
