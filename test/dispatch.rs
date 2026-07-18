use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

use mocari::mcp::MocariMcpServer;
use mocari::mcp::ModelSession;
use mocari::mcp::tools::all_tools;
use rmcp::handler::server::ServerHandler;
use rmcp::{RoleServer, serve_server};
use rmcp::service::RunningService;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Send a JSON-RPC request and read back the response line.
async fn rpc_call(
    writer: &mut (impl AsyncWriteExt + Unpin),
    reader: &mut BufReader<impl AsyncReadExt + Unpin>,
    method: &str,
    params: serde_json::Value,
    id: u64,
) -> serde_json::Value {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id,
    });
    let mut line = serde_json::to_string(&msg).unwrap();
    line.push('\n');
    writer.write_all(line.as_bytes()).await.unwrap();

    let mut buf = Vec::new();
    reader.read_until(b'\n', &mut buf).await.unwrap();
    serde_json::from_slice(&buf).unwrap()
}

/// Send a JSON-RPC notification (no id, no response expected).
async fn rpc_notify(
    writer: &mut (impl AsyncWriteExt + Unpin),
    method: &str,
    params: serde_json::Value,
) {
    let msg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    let mut line = serde_json::to_string(&msg).unwrap();
    line.push('\n');
    writer.write_all(line.as_bytes()).await.unwrap();
}

/// Helper struct to hold the split duplex stream halves and keep the server alive.
struct McpClient {
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    reader: BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    next_id: u64,
    _server_handle: tokio::task::JoinHandle<
        Result<RunningService<RoleServer, MocariMcpServer>, rmcp::service::ServerInitializeError>,
    >,
}

impl McpClient {
    async fn new() -> Self {
        let (server_io, client_io) = tokio::io::duplex(8192);
        let server = MocariMcpServer::new(ModelSession::new());

        // Spawn server so it can process requests concurrently with client.
        let server_handle = tokio::spawn(async move {
            serve_server(server, server_io).await
        });

        let (read_half, write_half) = tokio::io::split(client_io);
        let mut client = Self {
            writer: write_half,
            reader: BufReader::new(read_half),
            next_id: 1,
            _server_handle: server_handle,
        };

        // Initialize handshake
        let resp = client
            .call(
                "initialize",
                serde_json::json!({
                    "protocolVersion": "2025-03-26",
                    "capabilities": {},
                    "clientInfo": {"name": "test", "version": "0.1"}
                }),
            )
            .await;
        assert!(resp.get("result").is_some(), "initialize failed: {resp:?}");

        client
            .notify("notifications/initialized", serde_json::json!({}))
            .await;

        client
    }

    async fn call(&mut self, method: &str, params: serde_json::Value) -> serde_json::Value {
        let id = self.next_id;
        self.next_id += 1;
        rpc_call(&mut self.writer, &mut self.reader, method, params, id).await
    }

    async fn notify(&mut self, method: &str, params: serde_json::Value) {
        rpc_notify(&mut self.writer, method, params).await;
    }
}

// ===========================================================================
// Static tests
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
// Full MCP protocol dispatch tests
// ===========================================================================

#[tokio::test]
async fn mcp_dispatch_list_tools() {
    let mut client = McpClient::new().await;
    let resp = client.call("tools/list", serde_json::json!({})).await;
    let tools = resp["result"]["tools"].as_array().expect("tools array");
    assert_eq!(tools.len(), 28, "should list 28 tools");

    let names: Vec<&str> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"load_model"));
    assert!(names.contains(&"create_model_json"));
    assert!(names.contains(&"tick"));
}

#[tokio::test]
async fn mcp_dispatch_tools_call_unknown_tool() {
    let mut client = McpClient::new().await;
    let resp = client
        .call(
            "tools/call",
            serde_json::json!({
                "name": "nonexistent_tool",
                "arguments": {}
            }),
        )
        .await;
    assert!(
        resp.get("error").is_some(),
        "unknown tool should return error: {resp:?}"
    );
}

#[tokio::test]
async fn mcp_dispatch_tools_call_list_models_empty() {
    let mut client = McpClient::new().await;
    let resp = client
        .call(
            "tools/call",
            serde_json::json!({
                "name": "list_models",
                "arguments": {}
            }),
        )
        .await;
    let result = resp.get("result").expect("should have result");
    assert_eq!(result["isError"], false);
    let text = result["content"][0]["text"].as_str().unwrap();
    assert_eq!(text, "[]");
}

#[tokio::test]
async fn mcp_dispatch_tools_call_create_model_json() {
    let mut client = McpClient::new().await;
    let resp = client
        .call(
            "tools/call",
            serde_json::json!({
                "name": "create_model_json",
                "arguments": {
                    "name": "Test",
                    "moc_path": "test.moc3",
                    "textures": ["t.png"]
                }
            }),
        )
        .await;
    let result = resp.get("result").expect("should have result");
    assert_eq!(result["isError"], false);
    let text = result["content"][0]["text"].as_str().unwrap();
    let parsed: serde_json::Value = serde_json::from_str(text).unwrap();
    assert_eq!(parsed["Version"], 3);
}

#[tokio::test]
async fn mcp_dispatch_tools_call_missing_required_arg() {
    let mut client = McpClient::new().await;
    let resp = client
        .call(
            "tools/call",
            serde_json::json!({
                "name": "load_model",
                "arguments": {}
            }),
        )
        .await;
    assert!(
        resp.get("error").is_some(),
        "missing required arg should return error: {resp:?}"
    );
}

#[tokio::test]
async fn mcp_dispatch_tools_call_set_parameter_missing_args() {
    let mut client = McpClient::new().await;
    let resp = client
        .call(
            "tools/call",
            serde_json::json!({
                "name": "set_parameter",
                "arguments": {}
            }),
        )
        .await;
    assert!(
        resp.get("error").is_some(),
        "set_parameter without args should return error: {resp:?}"
    );
}

// ===========================================================================
// Verify all 28 tool names are dispatchable via MCP protocol
// ===========================================================================

#[tokio::test]
async fn mcp_dispatch_all_tool_names_recognized() {
    let tools = all_tools();
    let mut client = McpClient::new().await;

    for tool in &tools {
        let resp = client
            .call(
                "tools/call",
                serde_json::json!({
                    "name": tool.name,
                    "arguments": {}
                }),
            )
            .await;

        // Should NOT be a "method not found" error (-32601).
        // The tool name must be recognized by the dispatcher.
        // It may return a tool-level error (missing args, etc) which is fine.
        if let Some(error) = resp.get("error") {
            let code = error["code"].as_i64().unwrap_or(0);
            assert_ne!(
                code, -32601,
                "tool '{}' was not recognized (method not found): {resp:?}",
                tool.name
            );
        }
    }
}
