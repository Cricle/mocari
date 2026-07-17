pub mod creator;
pub mod runtime;
pub mod session;
mod tools;

use std::sync::Arc;

use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use tokio::sync::Mutex;

pub use session::ModelSession;

// Shared tool helpers used by both runtime and creator modules.

pub(super) type ToolResult = Result<CallToolResult, rmcp::ErrorData>;

pub(super) fn tool_error(msg: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg)]))
}

pub(super) fn success(text: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into())
}

#[derive(Debug, Clone)]
pub struct MocariMcpServer {
    session: Arc<Mutex<ModelSession>>,
}

impl MocariMcpServer {
    pub fn new(session: ModelSession) -> Self {
        Self {
            session: Arc::new(Mutex::new(session)),
        }
    }
}

impl ServerHandler for MocariMcpServer {
    fn get_info(&self) -> ServerInfo {
        InitializeResult::new(
            ServerCapabilities::builder().enable_tools().build(),
        )
        .with_server_info(Implementation::new(
            "mocari-mcp",
            env!("CARGO_PKG_VERSION"),
        ))
        .with_instructions(
            "Mocari MCP server for Live2D model control and creation. \
             Use load_model to load a .model3.json file, then control \
             parameters, motions, and expressions. Use create_* tools \
             to generate model files from scratch.",
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, rmcp::ErrorData> {
        Ok(ListToolsResult {
            tools: tools::all_tools(),
            ..Default::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let name = request.name.as_ref();
        let args = request.arguments.unwrap_or_default();

        match name {
            // Runtime tools
            "load_model" => runtime::handle_load_model(&self.session, args).await,
            "unload_model" => runtime::handle_unload_model(&self.session, args).await,
            "list_models" => runtime::handle_list_models(&self.session, args).await,
            "list_parameters" => runtime::handle_list_parameters(&self.session, args).await,
            "set_parameter" => runtime::handle_set_parameter(&self.session, args).await,
            "get_parameter" => runtime::handle_get_parameter(&self.session, args).await,
            "list_drawables" => runtime::handle_list_drawables(&self.session, args).await,
            "set_drawable_visible" => {
                runtime::handle_set_drawable_visible(&self.session, args).await
            }
            "set_drawable_color" => runtime::handle_set_drawable_color(&self.session, args).await,
            "play_motion" => runtime::handle_play_motion(&self.session, args).await,
            "stop_motions" => runtime::handle_stop_motions(&self.session, args).await,
            "play_expression" => runtime::handle_play_expression(&self.session, args).await,
            "stop_expressions" => runtime::handle_stop_expressions(&self.session, args).await,
            "configure_eye_blink" => {
                runtime::handle_configure_eye_blink(&self.session, args).await
            }
            "configure_lip_sync" => {
                runtime::handle_configure_lip_sync(&self.session, args).await
            }
            "configure_breath" => runtime::handle_configure_breath(&self.session, args).await,
            "configure_mouse_tracker" => {
                runtime::handle_configure_mouse_tracker(&self.session, args).await
            }
            "configure_physics" => runtime::handle_configure_physics(&self.session, args).await,
            "tick" => runtime::handle_tick(&self.session, args).await,
            "get_state" => runtime::handle_get_state(&self.session, args).await,
            // Creator tools
            "create_model_json" => creator::handle_create_model_json(args).await,
            "create_motion_json" => creator::handle_create_motion_json(args).await,
            "create_expression_json" => creator::handle_create_expression_json(args).await,
            "create_physics_json" => creator::handle_create_physics_json(args).await,
            "create_pose_json" => creator::handle_create_pose_json(args).await,
            "create_userdata_json" => creator::handle_create_userdata_json(args).await,
            "create_simple_moc3" => creator::handle_create_simple_moc3(args).await,
            "create_model_bundle" => creator::handle_create_model_bundle(args).await,
            _ => Err(rmcp::ErrorData::method_not_found::<
                CallToolRequestMethod,
            >()),
        }
    }
}
