use mocari::mcp::MocariMcpServer;
use mocari::mcp::ModelSession;
use mocari::mcp::tools::all_tools;
use rmcp::handler::server::ServerHandler;

// ===========================================================================
// list_tools
// ===========================================================================

#[test]
fn list_tools_returns_28_tools() {
    let tools = all_tools();
    assert_eq!(tools.len(), 28);
}

#[test]
fn all_tool_names_are_unique() {
    let tools = all_tools();
    let mut names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let original_len = names.len();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), original_len, "tool names must be unique");
}

// ===========================================================================
// ServerHandler::get_info — verify tools capability is present
// ===========================================================================

#[test]
fn server_info_declares_tools_capability() {
    let server = MocariMcpServer::new(ModelSession::new());
    let info = server.get_info();
    assert!(
        info.capabilities.tools.is_some(),
        "server must declare tools capability"
    );
}

// ===========================================================================
// Verify the 28 expected tool names exist
// ===========================================================================

#[test]
fn expected_runtime_tools_present() {
    let tools = all_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let expected = [
        "load_model",
        "unload_model",
        "list_models",
        "list_parameters",
        "set_parameter",
        "get_parameter",
        "list_drawables",
        "set_drawable_visible",
        "set_drawable_color",
        "play_motion",
        "stop_motions",
        "play_expression",
        "stop_expressions",
        "configure_eye_blink",
        "configure_lip_sync",
        "configure_breath",
        "configure_mouse_tracker",
        "configure_physics",
        "tick",
        "get_state",
    ];
    for name in expected {
        assert!(
            names.contains(&name),
            "expected runtime tool '{name}' not found"
        );
    }
}

#[test]
fn expected_creator_tools_present() {
    let tools = all_tools();
    let names: Vec<&str> = tools.iter().map(|t| t.name.as_ref()).collect();
    let expected = [
        "create_model_json",
        "create_motion_json",
        "create_expression_json",
        "create_physics_json",
        "create_pose_json",
        "create_userdata_json",
        "create_simple_moc3",
        "create_model_bundle",
    ];
    for name in expected {
        assert!(
            names.contains(&name),
            "expected creator tool '{name}' not found"
        );
    }
}
