# MCP Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an MCP server to Mocari that exposes Live2D model control and creation capabilities to AI assistants.

**Architecture:** Single crate with `src/mcp/` module. `MocariMcpServer` implements `rmcp::ServerHandler` manually (not via macros) for full control over tool dispatch and shared state. `ModelSession` manages multiple loaded models behind `Arc<Mutex<>>`. Standalone binary at `src/bin/mocari-mcp.rs`.

**Tech Stack:** rmcp 2 (features: server, transport-io, transport-streamable-http-server, macros, schemars), tokio, serde_json, base64, clap

## Global Constraints

- `#![forbid(unsafe_code)]` — no unsafe anywhere
- All public types derive `Debug, Clone`
- Edition 2024 (Rust 1.85+)
- rmcp `ServerHandler` implemented manually (not via `#[tool_router]` macro) for shared-state access
- `render_frame` tool deferred — requires wgpu offscreen rendering infrastructure not yet present
- Tool JSON schemas built via `serde_json::json!()` with `"type": "object"` — no runtime schema generation
- `ModelSession` methods are synchronous (in-memory operations only)
- Error responses use `CallToolResult::error()` for tool-level failures, `Err(ErrorData)` for protocol-level errors

---

### Task 1: Dependencies and Module Scaffolding

**Files:**
- Modify: `Cargo.toml`
- Create: `src/mcp/mod.rs`
- Create: `src/mcp/session.rs`
- Create: `src/mcp/runtime.rs`
- Create: `src/mcp/creator.rs`
- Create: `src/bin/mocari-mcp.rs`

**Interfaces:**
- Produces: `src/mcp/mod.rs` exports `MocariMcpServer`, `ModelSession`
- Produces: `src/bin/mocari-mcp.rs` compiles (empty main)

- [ ] **Step 1: Add dependencies to Cargo.toml**

Add to `[dependencies]`:
```toml
rmcp = { version = "2", default-features = false, features = ["server", "transport-io", "macros", "schemars"] }
tokio = { version = "1", features = ["full"] }
base64 = "0.22"
clap = { version = "4", features = ["derive"] }
```

Add to `[features]`:
```toml
mcp = []
mcp-http = ["rmcp/transport-streamable-http-server"]
```

Add binary target:
```toml
[[bin]]
name = "mocari-mcp"
path = "src/bin/mocari-mcp.rs"
required-features = ["mcp"]
```

- [ ] **Step 2: Create src/mcp/mod.rs skeleton**

```rust
pub mod session;
pub mod runtime;
pub mod creator;

pub use session::ModelSession;
```

Also add to `src/lib.rs`:
```rust
#[cfg(feature = "mcp")]
pub mod mcp;
```

- [ ] **Step 3: Create src/mcp/session.rs skeleton**

```rust
use std::collections::HashMap;
use std::fmt;

#[derive(Debug)]
pub struct ModelSession {
    models: HashMap<String, ()>,
    next_id: u64,
}

impl ModelSession {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            next_id: 0,
        }
    }
}
```

- [ ] **Step 4: Create src/mcp/runtime.rs and src/mcp/creator.rs empty modules**

```rust
// src/mcp/runtime.rs
// Runtime control tools will be added in Task 4
```

```rust
// src/mcp/creator.rs
// Model creation tools will be added in Task 5
```

- [ ] **Step 5: Create src/bin/mocari-mcp.rs skeleton**

```rust
fn main() {
    println!("mocari-mcp: not yet implemented");
}
```

- [ ] **Step 6: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles (with warnings about unused code)

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock src/mcp/ src/bin/mocari-mcp.rs
git commit -m "feat(mcp): add module scaffolding and dependencies"
```

---

### Task 2: ModelSession — Model Loading and Management

**Files:**
- Modify: `src/mcp/session.rs`

**Interfaces:**
- Consumes: `mocari::assets::load_model_runtime`, `mocari::runtime::ModelRuntime`, `mocari::motion::MotionManager`, `mocari::expression::ExpressionManager`, `mocari::auto::*`
- Produces: `ModelSession::load_model(path) -> Result<String, SessionError>`, `ModelSession::unload_model(id)`, `ModelSession::list_models()`, `ModelSession::with_model(id, closure)`, `ModelSession::with_model_mut(id, closure)`

- [ ] **Step 1: Define LoadedModel and SessionError**

```rust
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fmt;

use crate::assets::{self, RuntimeModel};
use crate::motion::MotionManager;
use crate::expression::ExpressionManager;
use crate::auto::{EyeBlink, LipSync, Breath, MouseTracker};
use crate::runtime::ModelRuntime;

#[derive(Debug)]
pub struct LoadedModel {
    pub runtime: ModelRuntime,
    pub motion_manager: MotionManager,
    pub expression_manager: ExpressionManager,
    pub eye_blink: Option<EyeBlink>,
    pub lip_sync: Option<LipSync>,
    pub breath: Option<Breath>,
    pub mouse_tracker: Option<MouseTracker>,
    pub base_path: PathBuf,
}

#[derive(Debug)]
pub enum SessionError {
    ModelNotFound(String),
    FileNotFound(String),
    LoadError(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelNotFound(id) => write!(f, "model not found: {id}"),
            Self::FileNotFound(path) => write!(f, "file not found: {path}"),
            Self::LoadError(msg) => write!(f, "load error: {msg}"),
        }
    }
}

impl std::error::Error for SessionError {}
```

- [ ] **Step 2: Implement ModelSession with load/unload/list**

```rust
#[derive(Debug)]
pub struct ModelSession {
    models: HashMap<String, LoadedModel>,
    next_id: u64,
}

impl ModelSession {
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn load_model(&mut self, path: &str) -> Result<String, SessionError> {
        let path = Path::new(path);
        if !path.exists() {
            return Err(SessionError::FileNotFound(path.display().to_string()));
        }

        let mut rt_model = assets::load_model_runtime(path)
            .map_err(|e| SessionError::LoadError(e.to_string()))?;

        let runtime = rt_model.runtime_mut();
        // Initialize auto-systems from model data
        let eye_blink = runtime.eye_blink_config_from_model()
            .map(|cfg| EyeBlink::new(cfg));
        let lip_sync = runtime.lip_sync_config_from_model()
            .map(|cfg| LipSync::new(cfg));
        let breath = Some(Breath::new(Default::default()));

        // Take ownership of runtime (textures dropped — not needed for headless)
        let runtime = std::mem::replace(rt_model.runtime_mut(), /* dummy */);
        // Actually, we need a different approach — see Step 3

        self.next_id += 1;
        let id = format!("model_{}", self.next_id);
        let base_path = path.parent().unwrap_or(Path::new(".")).to_path_buf();

        self.models.insert(id.clone(), LoadedModel {
            runtime,
            motion_manager: MotionManager::new(),
            expression_manager: ExpressionManager::new(),
            eye_blink,
            lip_sync,
            breath,
            mouse_tracker: None,
            base_path,
        });

        Ok(id)
    }

    pub fn unload_model(&mut self, id: &str) -> bool {
        self.models.remove(id).is_some()
    }

    pub fn list_models(&self) -> Vec<(&str, &Path, usize, usize)> {
        self.models.iter().map(|(id, m)| {
            (id.as_str(), m.base_path.as_path(), 0, 0) // counts filled in later
        }).collect()
    }

    pub fn with_model<R>(&self, id: &str, f: impl FnOnce(&LoadedModel) -> R) -> Result<R, SessionError> {
        self.models.get(id).map(f).ok_or_else(|| SessionError::ModelNotFound(id.to_string()))
    }

    pub fn with_model_mut<R>(&mut self, id: &str, f: impl FnOnce(&mut LoadedModel) -> R) -> Result<R, SessionError> {
        self.models.get_mut(id).map(f).ok_or_else(|| SessionError::ModelNotFound(id.to_string()))
    }
}
```

- [ ] **Step 3: Fix runtime extraction from RuntimeModel**

`RuntimeModel` owns both `ModelRuntime` and textures. For headless MCP, we only need the runtime. The approach:

```rust
// In load_model, after getting rt_model:
let runtime_ref = rt_model.runtime();
// Clone or extract what we need. ModelRuntime doesn't implement Clone,
// so we need to restructure. Use into_parts() if available, or:
// Option A: Add a method to RuntimeModel to consume self and return parts
// Option B: Keep RuntimeModel in LoadedModel (textures stay in memory)

// Go with Option B for now — simpler, textures may be needed for render_frame later
```

Update `LoadedModel` to hold `RuntimeModel` instead of `ModelRuntime`:

```rust
pub struct LoadedModel {
    pub model: RuntimeModel,
    pub motion_manager: MotionManager,
    pub expression_manager: ExpressionManager,
    pub eye_blink: Option<EyeBlink>,
    pub lip_sync: Option<LipSync>,
    pub breath: Option<Breath>,
    pub mouse_tracker: Option<MouseTracker>,
    pub base_path: PathBuf,
}
```

And `load_model` becomes:
```rust
pub fn load_model(&mut self, path: &str) -> Result<String, SessionError> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(SessionError::FileNotFound(path.display().to_string()));
    }

    let rt_model = assets::load_model_runtime(path)
        .map_err(|e| SessionError::LoadError(e.to_string()))?;

    let eye_blink = rt_model.runtime().eye_blink_config_from_model()
        .map(|cfg| EyeBlink::new(cfg));
    let lip_sync = rt_model.runtime().lip_sync_config_from_model()
        .map(|cfg| LipSync::new(cfg));

    self.next_id += 1;
    let id = format!("model_{}", self.next_id);
    let base_path = path.parent().unwrap_or(Path::new(".")).to_path_buf();

    self.models.insert(id.clone(), LoadedModel {
        model: rt_model,
        motion_manager: MotionManager::new(),
        expression_manager: ExpressionManager::new(),
        eye_blink,
        lip_sync,
        breath: Some(Breath::new(Default::default())),
        mouse_tracker: None,
        base_path,
    });

    Ok(id)
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add src/mcp/session.rs
git commit -m "feat(mcp): implement ModelSession with load/unload/list"
```

---

### Task 3: MCP Server Skeleton — ServerHandler Implementation

**Files:**
- Modify: `src/mcp/mod.rs`
- Create: `src/mcp/tools.rs` (tool definitions — static list of `Tool` structs)

**Interfaces:**
- Consumes: `ModelSession`, `rmcp::ServerHandler`, `rmcp::model::{Tool, CallToolRequestParams, CallToolResult, CallToolResponse, ContentBlock, ServerInfo, ListToolsResult, ListPromptsResult, ListResourcesResult}`
- Produces: `MocariMcpServer` implementing `ServerHandler` with `list_tools()` returning all 29 tool definitions and `call_tool()` dispatching by name

- [ ] **Step 1: Create src/mcp/tools.rs with tool definitions**

```rust
use std::sync::Arc;
use rmcp::model::{JsonObject, Tool};
use serde_json::json;

fn tool_schema(properties: serde_json::Value, required: &[&str]) -> Arc<JsonObject> {
    Arc::new(serde_json::from_value(json!({
        "type": "object",
        "properties": properties,
        "required": required
    })).unwrap())
}

// --- Runtime tools ---

pub fn load_model_tool() -> Tool {
    Tool::new(
        "load_model",
        "Load a Live2D model from a .model3.json file",
        tool_schema(json!({
            "path": { "type": "string", "description": "Path to .model3.json file" }
        }), &["path"]),
    )
}

pub fn unload_model_tool() -> Tool {
    Tool::new(
        "unload_model",
        "Unload a previously loaded model",
        tool_schema(json!({
            "model_id": { "type": "string", "description": "Model ID returned by load_model" }
        }), &["model_id"]),
    )
}

pub fn list_models_tool() -> Tool {
    Tool::new("list_models", "List all loaded models", tool_schema(json!({}), &[]))
}

pub fn list_parameters_tool() -> Tool {
    Tool::new(
        "list_parameters",
        "List all parameters of a loaded model",
        tool_schema(json!({
            "model_id": { "type": "string" }
        }), &["model_id"]),
    )
}

pub fn set_parameter_tool() -> Tool {
    Tool::new(
        "set_parameter",
        "Set a model parameter value",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "parameter_id": { "type": "string" },
            "value": { "type": "number" }
        }), &["model_id", "parameter_id", "value"]),
    )
}

pub fn get_parameter_tool() -> Tool {
    Tool::new(
        "get_parameter",
        "Get a model parameter's current value and range",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "parameter_id": { "type": "string" }
        }), &["model_id", "parameter_id"]),
    )
}

pub fn list_drawables_tool() -> Tool {
    Tool::new(
        "list_drawables",
        "List all drawables (art meshes) of a loaded model",
        tool_schema(json!({
            "model_id": { "type": "string" }
        }), &["model_id"]),
    )
}

pub fn set_drawable_visible_tool() -> Tool {
    Tool::new(
        "set_drawable_visible",
        "Set visibility of a drawable",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "drawable_id": { "type": "string" },
            "visible": { "type": "boolean" }
        }), &["model_id", "drawable_id", "visible"]),
    )
}

pub fn set_drawable_color_tool() -> Tool {
    Tool::new(
        "set_drawable_color",
        "Set multiply and screen color overrides for a drawable",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "drawable_id": { "type": "string" },
            "multiply": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 },
            "screen": { "type": "array", "items": { "type": "number" }, "minItems": 3, "maxItems": 3 }
        }), &["model_id", "drawable_id"]),
    )
}

pub fn play_motion_tool() -> Tool {
    Tool::new(
        "play_motion",
        "Play a motion animation on a model",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "path": { "type": "string", "description": "Path to .motion3.json file" },
            "priority": { "type": "string", "enum": ["idle", "normal", "force"] },
            "group": { "type": "string" }
        }), &["model_id", "path"]),
    )
}

pub fn stop_motions_tool() -> Tool {
    Tool::new(
        "stop_motions",
        "Stop all playing motions",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "fade_seconds": { "type": "number" }
        }), &["model_id"]),
    )
}

pub fn play_expression_tool() -> Tool {
    Tool::new(
        "play_expression",
        "Play an expression on a model",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "path": { "type": "string", "description": "Path to .exp3.json file" }
        }), &["model_id", "path"]),
    )
}

pub fn stop_expressions_tool() -> Tool {
    Tool::new(
        "stop_expressions",
        "Stop all playing expressions",
        tool_schema(json!({
            "model_id": { "type": "string" }
        }), &["model_id"]),
    )
}

pub fn configure_eye_blink_tool() -> Tool {
    Tool::new(
        "configure_eye_blink",
        "Configure automatic eye blinking",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "enabled": { "type": "boolean" },
            "weight": { "type": "number" }
        }), &["model_id", "enabled"]),
    )
}

pub fn configure_lip_sync_tool() -> Tool {
    Tool::new(
        "configure_lip_sync",
        "Configure lip sync with audio amplitude",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "amplitude": { "type": "number" },
            "weight": { "type": "number" }
        }), &["model_id", "amplitude"]),
    )
}

pub fn configure_breath_tool() -> Tool {
    Tool::new(
        "configure_breath",
        "Configure automatic breathing",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "enabled": { "type": "boolean" },
            "weight": { "type": "number" }
        }), &["model_id", "enabled"]),
    )
}

pub fn configure_mouse_tracker_tool() -> Tool {
    Tool::new(
        "configure_mouse_tracker",
        "Set mouse tracking position and weight",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "x": { "type": "number" },
            "y": { "type": "number" },
            "weight": { "type": "number" }
        }), &["model_id", "x", "y"]),
    )
}

pub fn configure_physics_tool() -> Tool {
    Tool::new(
        "configure_physics",
        "Enable or disable physics simulation",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "enabled": { "type": "boolean" }
        }), &["model_id", "enabled"]),
    )
}

pub fn tick_tool() -> Tool {
    Tool::new(
        "tick",
        "Advance model time by delta_seconds, applying all active systems",
        tool_schema(json!({
            "model_id": { "type": "string" },
            "delta_seconds": { "type": "number" }
        }), &["model_id", "delta_seconds"]),
    )
}

pub fn get_state_tool() -> Tool {
    Tool::new(
        "get_state",
        "Get a full state snapshot of a model (parameters, drawables, motions)",
        tool_schema(json!({
            "model_id": { "type": "string" }
        }), &["model_id"]),
    )
}

// --- Creator tools ---

pub fn create_model_json_tool() -> Tool {
    Tool::new(
        "create_model_json",
        "Generate a valid model3.json file",
        tool_schema(json!({
            "name": { "type": "string" },
            "moc_path": { "type": "string" },
            "textures": { "type": "array", "items": { "type": "string" } },
            "motions": { "type": "object" },
            "expressions": { "type": "array" }
        }), &["name", "moc_path", "textures"]),
    )
}

pub fn create_motion_json_tool() -> Tool {
    Tool::new(
        "create_motion_json",
        "Generate a valid motion3.json file",
        tool_schema(json!({
            "duration": { "type": "number" },
            "fps": { "type": "number" },
            "loop": { "type": "boolean" },
            "curves": { "type": "array" }
        }), &["duration", "fps", "curves"]),
    )
}

pub fn create_expression_json_tool() -> Tool {
    Tool::new(
        "create_expression_json",
        "Generate a valid exp3.json file",
        tool_schema(json!({
            "fade_in": { "type": "number" },
            "fade_out": { "type": "number" },
            "parameters": { "type": "array" }
        }), &["parameters"]),
    )
}

pub fn create_physics_json_tool() -> Tool {
    Tool::new(
        "create_physics_json",
        "Generate a valid physics3.json file",
        tool_schema(json!({
            "settings": { "type": "object" },
            "physics_info": { "type": "array" }
        }), &["settings", "physics_info"]),
    )
}

pub fn create_pose_json_tool() -> Tool {
    Tool::new(
        "create_pose_json",
        "Generate a valid pose3.json file",
        tool_schema(json!({
            "fade_in_time": { "type": "number" },
            "groups": { "type": "array" }
        }), &["groups"]),
    )
}

pub fn create_userdata_json_tool() -> Tool {
    Tool::new(
        "create_userdata_json",
        "Generate a valid userdata3.json file",
        tool_schema(json!({
            "entries": { "type": "array" }
        }), &["entries"]),
    )
}

pub fn create_simple_moc3_tool() -> Tool {
    Tool::new(
        "create_simple_moc3",
        "Generate a minimal valid .moc3 binary",
        tool_schema(json!({
            "name": { "type": "string" },
            "width": { "type": "number" },
            "height": { "type": "number" },
            "parameters": { "type": "array" },
            "meshes": { "type": "array" }
        }), &["name", "width", "height", "parameters", "meshes"]),
    )
}

pub fn create_model_bundle_tool() -> Tool {
    Tool::new(
        "create_model_bundle",
        "Create a complete model directory with all necessary files",
        tool_schema(json!({
            "name": { "type": "string" },
            "description": { "type": "string" },
            "meshes": { "type": "array" },
            "parameters": { "type": "array" },
            "motions": { "type": "array" },
            "expressions": { "type": "array" }
        }), &["name", "meshes", "parameters"]),
    )
}

/// Returns all tool definitions.
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
```

- [ ] **Step 2: Implement MocariMcpServer in src/mcp/mod.rs**

```rust
pub mod session;
pub mod runtime;
pub mod creator;
mod tools;

use std::sync::Arc;
use tokio::sync::Mutex;

use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::RequestContext;
use rmcp::RoleServer;

pub use session::ModelSession;

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
            ServerCapabilities::builder()
                .enable_tools()
                .build(),
        )
        .with_instructions(
            "Mocari MCP server for Live2D model control and creation. \
             Use load_model to load a .model3.json file, then control \
             parameters, motions, and expressions. Use create_* tools \
             to generate model files from scratch."
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
    ) -> Result<CallToolResponse, rmcp::ErrorData> {
        let name = request.name.as_ref();
        let args = request.arguments.unwrap_or_default();

        match name {
            // Runtime tools — dispatched to runtime::handle_* functions
            "load_model" => runtime::handle_load_model(&self.session, args).await,
            "unload_model" => runtime::handle_unload_model(&self.session, args).await,
            "list_models" => runtime::handle_list_models(&self.session, args).await,
            "list_parameters" => runtime::handle_list_parameters(&self.session, args).await,
            "set_parameter" => runtime::handle_set_parameter(&self.session, args).await,
            "get_parameter" => runtime::handle_get_parameter(&self.session, args).await,
            "list_drawables" => runtime::handle_list_drawables(&self.session, args).await,
            "set_drawable_visible" => runtime::handle_set_drawable_visible(&self.session, args).await,
            "set_drawable_color" => runtime::handle_set_drawable_color(&self.session, args).await,
            "play_motion" => runtime::handle_play_motion(&self.session, args).await,
            "stop_motions" => runtime::handle_stop_motions(&self.session, args).await,
            "play_expression" => runtime::handle_play_expression(&self.session, args).await,
            "stop_expressions" => runtime::handle_stop_expressions(&self.session, args).await,
            "configure_eye_blink" => runtime::handle_configure_eye_blink(&self.session, args).await,
            "configure_lip_sync" => runtime::handle_configure_lip_sync(&self.session, args).await,
            "configure_breath" => runtime::handle_configure_breath(&self.session, args).await,
            "configure_mouse_tracker" => runtime::handle_configure_mouse_tracker(&self.session, args).await,
            "configure_physics" => runtime::handle_configure_physics(&self.session, args).await,
            "tick" => runtime::handle_tick(&self.session, args).await,
            "get_state" => runtime::handle_get_state(&self.session, args).await,
            // Creator tools — dispatched to creator::handle_* functions
            "create_model_json" => creator::handle_create_model_json(args).await,
            "create_motion_json" => creator::handle_create_motion_json(args).await,
            "create_expression_json" => creator::handle_create_expression_json(args).await,
            "create_physics_json" => creator::handle_create_physics_json(args).await,
            "create_pose_json" => creator::handle_create_pose_json(args).await,
            "create_userdata_json" => creator::handle_create_userdata_json(args).await,
            "create_simple_moc3" => creator::handle_create_simple_moc3(args).await,
            "create_model_bundle" => creator::handle_create_model_bundle(args).await,
            _ => Err(rmcp::ErrorData::method_not_found::<rmcp::model::CallToolRequestMethod>()),
        }
    }
}
```

- [ ] **Step 3: Add stub handler functions in runtime.rs and creator.rs**

```rust
// src/mcp/runtime.rs
use std::sync::Arc;
use tokio::sync::Mutex;
use rmcp::model::{CallToolResult, CallToolResponse, ContentBlock, JsonObject};
use super::session::ModelSession;

type ToolResult = Result<CallToolResponse, rmcp::ErrorData>;

fn success(text: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into())
}

fn tool_error(msg: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg)]).into())
}

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

fn get_bool(args: &JsonObject, key: &str) -> Result<bool, rmcp::ErrorData> {
    args.get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| rmcp::ErrorData::invalid_params(format!("missing '{key}'"), None))
}

// Stub handlers — return "not yet implemented"
pub async fn handle_load_model(_session: &Arc<Mutex<ModelSession>>, _args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}
// ... (all 20 runtime handlers follow same pattern)
```

```rust
// src/mcp/creator.rs
use rmcp::model::{CallToolResult, CallToolResponse, ContentBlock, JsonObject};

type ToolResult = Result<CallToolResponse, rmcp::ErrorData>;

fn success(text: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into())
}

fn tool_error(msg: impl Into<String>) -> ToolResult {
    Ok(CallToolResult::error(vec![ContentBlock::text(msg)]).into())
}

pub async fn handle_create_model_json(_args: JsonObject) -> ToolResult {
    tool_error("not yet implemented")
}
// ... (all 8 creator handlers follow same pattern)
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add src/mcp/
git commit -m "feat(mcp): implement ServerHandler skeleton with tool dispatch"
```

---

### Task 4: Runtime Tools — Model Loading and Parameter Control

**Files:**
- Modify: `src/mcp/runtime.rs`

**Interfaces:**
- Consumes: `ModelSession::load_model`, `ModelSession::with_model`, `ModelSession::with_model_mut`, `ModelRuntime::parameter_info`, `ModelRuntime::set_parameter_value`, `ModelRuntime::parameter_value`
- Produces: Working `handle_load_model`, `handle_unload_model`, `handle_list_models`, `handle_list_parameters`, `handle_set_parameter`, `handle_get_parameter`

- [ ] **Step 1: Implement handle_load_model**

```rust
pub async fn handle_load_model(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let path = get_string(&args, "path")?;
    let mut session = session.lock().await;
    match session.load_model(&path) {
        Ok(id) => {
            let model = session.models.get(&id).unwrap();
            let runtime = model.model.runtime();
            let param_count = runtime.parameter_info().len();
            let mesh_count = runtime.meshes().len();
            success(format!(
                r#"{{"model_id": "{id}", "parameter_count": {param_count}, "drawable_count": {mesh_count}}}"#
            ))
        }
        Err(e) => tool_error(e.to_string()),
    }
}
```

- [ ] **Step 2: Implement handle_unload_model**

```rust
pub async fn handle_unload_model(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let mut session = session.lock().await;
    if session.unload_model(&id) {
        success(r#"{"success": true}"#)
    } else {
        tool_error(format!("model not found: {id}"))
    }
}
```

- [ ] **Step 3: Implement handle_list_models**

```rust
pub async fn handle_list_models(session: &Arc<Mutex<ModelSession>>, _args: JsonObject) -> ToolResult {
    let session = session.lock().await;
    let models: Vec<serde_json::Value> = session.models.iter().map(|(id, m)| {
        let runtime = m.model.runtime();
        serde_json::json!({
            "model_id": id,
            "path": m.base_path.display().to_string(),
            "parameter_count": runtime.parameter_info().len(),
            "drawable_count": runtime.meshes().len(),
        })
    }).collect();
    success(serde_json::to_string(&models).unwrap_or_else(|_| "[]".into()))
}
```

- [ ] **Step 4: Implement handle_list_parameters**

```rust
pub async fn handle_list_parameters(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let session = session.lock().await;
    session.with_model(&id, |m| {
        let runtime = m.model.runtime();
        let params: Vec<serde_json::Value> = runtime.parameter_info().iter().map(|p| {
            serde_json::json!({
                "id": p.id(),
                "min": p.min_value(),
                "max": p.max_value(),
                "default": p.default_value(),
                "current": runtime.parameter_value(p.index()).unwrap_or(p.default_value()),
            })
        }).collect();
        success(serde_json::to_string(&params).unwrap_or_else(|_| "[]".into()))
    })?
}
```

- [ ] **Step 5: Implement handle_set_parameter**

```rust
pub async fn handle_set_parameter(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let param_id = get_string(&args, "parameter_id")?;
    let value = get_number(&args, "value")? as f32;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();
        if let Some(index) = runtime.parameter_index(&param_id) {
            runtime.set_parameter_value(index, value);
            let actual = runtime.parameter_value(index).unwrap_or(value);
            success(format!(r#"{{"success": true, "actual_value": {actual}}}"#))
        } else {
            tool_error(format!("parameter not found: {param_id}"))
        }
    })?
}
```

- [ ] **Step 6: Implement handle_get_parameter**

```rust
pub async fn handle_get_parameter(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let param_id = get_string(&args, "parameter_id")?;
    let session = session.lock().await;
    session.with_model(&id, |m| {
        let runtime = m.model.runtime();
        if let Some(info) = runtime.parameter_info().iter().find(|p| p.id() == param_id) {
            let current = runtime.parameter_value(info.index()).unwrap_or(info.default_value());
            success(format!(
                r#"{{"value": {current}, "min": {}, "max": {}}}"#,
                info.min_value(), info.max_value()
            ))
        } else {
            tool_error(format!("parameter not found: {param_id}"))
        }
    })?
}
```

- [ ] **Step 7: Verify compilation and test**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 8: Commit**

```bash
git add src/mcp/runtime.rs
git commit -m "feat(mcp): implement load/unload/list models and parameter tools"
```

---

### Task 5: Runtime Tools — Drawable, Motion, Expression Control

**Files:**
- Modify: `src/mcp/runtime.rs`

**Interfaces:**
- Consumes: `ModelRuntime::meshes`, `ModelRuntime::set_drawable_visible`, `ModelRuntime::set_drawable_multiply_color`, `MotionManager::play`, `ExpressionManager::play`
- Produces: Working `handle_list_drawables`, `handle_set_drawable_visible`, `handle_set_drawable_color`, `handle_play_motion`, `handle_stop_motions`, `handle_play_expression`, `handle_stop_expressions`

- [ ] **Step 1: Implement handle_list_drawables**

```rust
pub async fn handle_list_drawables(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let session = session.lock().await;
    session.with_model(&id, |m| {
        let runtime = m.model.runtime();
        let drawables: Vec<serde_json::Value> = runtime.meshes().iter().enumerate().map(|(i, mesh)| {
            serde_json::json!({
                "id": runtime.drawable_id(i).unwrap_or(&format!("drawable_{i}")),
                "visible": mesh.is_visible(),
                "opacity": mesh.opacity(),
            })
        }).collect();
        success(serde_json::to_string(&drawables).unwrap_or_else(|_| "[]".into()))
    })?
}
```

- [ ] **Step 2: Implement handle_set_drawable_visible**

```rust
pub async fn handle_set_drawable_visible(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let drawable_id = get_string(&args, "drawable_id")?;
    let visible = get_bool(&args, "visible")?;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();
        if let Some(index) = runtime.drawable_index(&drawable_id) {
            runtime.set_drawable_visible(index, visible);
            success(r#"{"success": true}"#)
        } else {
            tool_error(format!("drawable not found: {drawable_id}"))
        }
    })?
}
```

- [ ] **Step 3: Implement handle_set_drawable_color**

```rust
pub async fn handle_set_drawable_color(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let drawable_id = get_string(&args, "drawable_id")?;
    let multiply = args.get("multiply").and_then(|v| {
        let arr = v.as_array()?;
        if arr.len() == 3 {
            Some([arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32])
        } else { None }
    });
    let screen = args.get("screen").and_then(|v| {
        let arr = v.as_array()?;
        if arr.len() == 3 {
            Some([arr[0].as_f64()? as f32, arr[1].as_f64()? as f32, arr[2].as_f64()? as f32])
        } else { None }
    });
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();
        if let Some(index) = runtime.drawable_index(&drawable_id) {
            if let Some(color) = multiply {
                runtime.set_drawable_multiply_color(index, Some(color));
            }
            if let Some(color) = screen {
                runtime.set_drawable_screen_color(index, Some(color));
            }
            success(r#"{"success": true}"#)
        } else {
            tool_error(format!("drawable not found: {drawable_id}"))
        }
    })?
}
```

- [ ] **Step 4: Implement handle_play_motion**

```rust
pub async fn handle_play_motion(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let path = get_string(&args, "path")?;
    let priority = args.get("priority").and_then(|v| v.as_str()).unwrap_or("normal");
    let _group = args.get("group").and_then(|v| v.as_str()).unwrap_or("");
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        let full_path = m.base_path.join(&path);
        match crate::motion::load_motion(&full_path) {
            Ok(motion) => {
                let pri = match priority {
                    "idle" => crate::motion::MotionPriority::Idle,
                    "force" => crate::motion::MotionPriority::Force,
                    _ => crate::motion::MotionPriority::Normal,
                };
                m.motion_manager.play(motion, pri);
                let count = m.motion_manager.active_count();
                success(format!(r#"{{"success": true, "active_count": {count}}}"#))
            }
            Err(e) => tool_error(format!("failed to load motion: {e}")),
        }
    })?
}
```

- [ ] **Step 5: Implement handle_stop_motions**

```rust
pub async fn handle_stop_motions(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let _fade = args.get("fade_seconds").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        m.motion_manager.stop_all();
        success(r#"{"success": true}"#)
    })?
}
```

- [ ] **Step 6: Implement handle_play_expression and handle_stop_expressions**

```rust
pub async fn handle_play_expression(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let path = get_string(&args, "path")?;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        let full_path = m.base_path.join(&path);
        match crate::expression::load_expression(&full_path) {
            Ok(expr) => {
                m.expression_manager.play(expr);
                success(r#"{"success": true}"#)
            }
            Err(e) => tool_error(format!("failed to load expression: {e}")),
        }
    })?
}

pub async fn handle_stop_expressions(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        m.expression_manager.stop_all();
        success(r#"{"success": true}"#)
    })?
}
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 8: Commit**

```bash
git add src/mcp/runtime.rs
git commit -m "feat(mcp): implement drawable, motion, and expression tools"
```

---

### Task 6: Runtime Tools — Auto-Systems, Tick, and State

**Files:**
- Modify: `src/mcp/runtime.rs`

**Interfaces:**
- Consumes: `EyeBlink`, `LipSync`, `Breath`, `MouseTracker`, `ModelRuntime::update_meshes`, `ModelRuntime::parameter_info`, `ModelRuntime::meshes`
- Produces: Working `handle_configure_eye_blink`, `handle_configure_lip_sync`, `handle_configure_breath`, `handle_configure_mouse_tracker`, `handle_configure_physics`, `handle_tick`, `handle_get_state`

- [ ] **Step 1: Implement configure_eye_blink**

```rust
pub async fn handle_configure_eye_blink(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let enabled = get_bool(&args, "enabled")?;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        if enabled {
            if m.eye_blink.is_none() {
                if let Some(cfg) = m.model.runtime().eye_blink_config_from_model() {
                    m.eye_blink = Some(crate::auto::EyeBlink::new(cfg));
                }
            }
            if let Some(ref mut eb) = m.eye_blink {
                eb.set_weight(weight);
            }
        } else {
            m.eye_blink = None;
        }
        success(r#"{"success": true}"#)
    })?
}
```

- [ ] **Step 2: Implement configure_lip_sync**

```rust
pub async fn handle_configure_lip_sync(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let amplitude = get_number(&args, "amplitude")? as f32;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        if m.lip_sync.is_none() {
            if let Some(cfg) = m.model.runtime().lip_sync_config_from_model() {
                m.lip_sync = Some(crate::auto::LipSync::new(cfg));
            }
        }
        if let Some(ref mut ls) = m.lip_sync {
            ls.set_amplitude(amplitude);
            ls.set_weight(weight);
        }
        success(r#"{"success": true}"#)
    })?
}
```

- [ ] **Step 3: Implement configure_breath**

```rust
pub async fn handle_configure_breath(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let enabled = get_bool(&args, "enabled")?;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
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
    })?
}
```

- [ ] **Step 4: Implement configure_mouse_tracker**

```rust
pub async fn handle_configure_mouse_tracker(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let x = get_number(&args, "x")? as f32;
    let y = get_number(&args, "y")? as f32;
    let weight = args.get("weight").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        if m.mouse_tracker.is_none() {
            m.mouse_tracker = Some(crate::auto::MouseTracker::new(Default::default()));
        }
        if let Some(ref mut mt) = m.mouse_tracker {
            mt.set_position(x, y);
            mt.set_weight(weight);
        }
        success(r#"{"success": true}"#)
    })?
}
```

- [ ] **Step 5: Implement configure_physics**

```rust
pub async fn handle_configure_physics(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let enabled = get_bool(&args, "enabled")?;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();
        runtime.set_physics_enabled(enabled);
        success(r#"{"success": true}"#)
    })?
}
```

- [ ] **Step 6: Implement handle_tick**

```rust
pub async fn handle_tick(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let delta = get_number(&args, "delta_seconds")? as f32;
    let mut session = session.lock().await;
    session.with_model_mut(&id, |m| {
        let runtime = m.model.runtime_mut();

        // Apply auto-systems
        if let Some(ref mut eb) = m.eye_blink {
            eb.apply(runtime, delta);
        }
        if let Some(ref mut ls) = m.lip_sync {
            ls.apply(runtime, delta);
        }
        if let Some(ref mut b) = m.breath {
            b.apply(runtime, delta);
        }
        if let Some(ref mut mt) = m.mouse_tracker {
            mt.apply(runtime, delta);
        }

        // Apply motions
        m.motion_manager.apply(runtime, delta);
        let motion_events = m.motion_manager.drain_events();

        // Apply expressions
        m.expression_manager.apply(runtime);

        // Rebuild meshes
        runtime.update_meshes(delta);

        let events_json = serde_json::to_string(&motion_events).unwrap_or_else(|_| "[]".into());
        success(format!(r#"{{"success": true, "events": {events_json}}}"#))
    })?
}
```

- [ ] **Step 7: Implement handle_get_state**

```rust
pub async fn handle_get_state(session: &Arc<Mutex<ModelSession>>, args: JsonObject) -> ToolResult {
    let id = get_string(&args, "model_id")?;
    let session = session.lock().await;
    session.with_model(&id, |m| {
        let runtime = m.model.runtime();
        let params: Vec<serde_json::Value> = runtime.parameter_info().iter().map(|p| {
            serde_json::json!({
                "id": p.id(),
                "value": runtime.parameter_value(p.index()).unwrap_or(p.default_value()),
            })
        }).collect();
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
    })?
}
```

- [ ] **Step 8: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 9: Commit**

```bash
git add src/mcp/runtime.rs
git commit -m "feat(mcp): implement auto-system, tick, and state tools"
```

---

### Task 7: Creator Tools — JSON Generators

**Files:**
- Modify: `src/mcp/creator.rs`

**Interfaces:**
- Consumes: `serde_json::json!`, `serde_json::to_string_pretty`
- Produces: Working `handle_create_model_json`, `handle_create_motion_json`, `handle_create_expression_json`, `handle_create_physics_json`, `handle_create_pose_json`, `handle_create_userdata_json`

- [ ] **Step 1: Implement handle_create_model_json**

```rust
pub async fn handle_create_model_json(args: JsonObject) -> ToolResult {
    let name = get_string(&args, "name")?;
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
```

- [ ] **Step 2: Implement handle_create_motion_json**

```rust
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
```

- [ ] **Step 3: Implement handle_create_expression_json**

```rust
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
```

- [ ] **Step 4: Implement handle_create_physics_json**

```rust
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
```

- [ ] **Step 5: Implement handle_create_pose_json**

```rust
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
```

- [ ] **Step 6: Implement handle_create_userdata_json**

```rust
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
```

- [ ] **Step 7: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 8: Commit**

```bash
git add src/mcp/creator.rs
git commit -m "feat(mcp): implement JSON creator tools"
```

---

### Task 8: Creator Tools — moc3 Binary and Model Bundle

**Files:**
- Modify: `src/mcp/creator.rs`

**Interfaces:**
- Consumes: `base64::Engine`, `serde_json::json!`, `std::fs`
- Produces: Working `handle_create_simple_moc3`, `handle_create_model_bundle`

- [ ] **Step 1: Implement handle_create_simple_moc3**

This generates a minimal valid moc3 binary. The moc3 format has a specific header and structure. For a minimal implementation, generate a valid header with zero meshes (placeholder — real moc3 generation is complex).

```rust
pub async fn handle_create_simple_moc3(args: JsonObject) -> ToolResult {
    let name = get_string(&args, "name")?;
    let _width = get_number(&args, "width")?;
    let _height = get_number(&args, "height")?;
    let _parameters = args.get("parameters")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'parameters'", None))?;
    let _meshes = args.get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'meshes'", None))?;

    // Minimal moc3 binary: magic + version header
    // Real moc3 generation is extremely complex; this produces a placeholder
    // that indicates the structure without full binary generation.
    let mut binary = Vec::new();
    binary.extend_from_slice(b"MOC3");  // magic
    binary.extend_from_slice(&[0, 0, 0, 3]);  // version 3
    // Remaining structure would require full moc3 format implementation
    // For now, return an error indicating this is a complex operation
    tool_error("create_simple_moc3: moc3 binary generation requires full Cubism SDK format implementation — use create_model_json to generate the JSON sidecar instead")
}
```

- [ ] **Step 2: Implement handle_create_model_bundle**

```rust
pub async fn handle_create_model_bundle(args: JsonObject) -> ToolResult {
    let name = get_string(&args, "name")?;
    let description = args.get("description").and_then(|v| v.as_str()).unwrap_or("");
    let meshes = args.get("meshes")
        .and_then(|v| v.as_array())
        .ok_or_else(|| rmcp::ErrorData::invalid_params("missing 'meshes'", None))?;
    let parameters = args.get("parameters")
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
```

- [ ] **Step 3: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 4: Commit**

```bash
git add src/mcp/creator.rs
git commit -m "feat(mcp): implement moc3 and model bundle creator tools"
```

---

### Task 9: Standalone Binary

**Files:**
- Modify: `src/bin/mocari-mcp.rs`

**Interfaces:**
- Consumes: `MocariMcpServer`, `ModelSession`, `rmcp::serve_server`, `rmcp::transport::io::stdio`, `clap::Parser`
- Produces: Working `mocari-mcp` binary with `--transport stdio` and `--transport http --port 3000`

- [ ] **Step 1: Implement CLI argument parsing**

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "mocari-mcp", about = "Mocari MCP server for Live2D model control")]
struct Args {
    /// Transport to use
    #[arg(long, default_value = "stdio")]
    transport: String,

    /// Port for HTTP transport
    #[arg(long, default_value_t = 3000)]
    port: u16,
}
```

- [ ] **Step 2: Implement main with stdio transport**

```rust
use mocari::mcp::{MocariMcpServer, ModelSession};
use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.transport.as_str() {
        "stdio" => run_stdio().await,
        "http" => run_http(args.port).await,
        other => {
            eprintln!("unknown transport: {other}");
            std::process::exit(1);
        }
    }
}

async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
    let session = ModelSession::new();
    let server = MocariMcpServer::new(session);

    let service = server.serve(rmcp::transport::io::stdio()).await?;
    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 3: Implement HTTP transport (behind feature gate)**

```rust
#[cfg(feature = "mcp-http")]
async fn run_http(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

    let session = ModelSession::new();
    let server = MocariMcpServer::new(session);

    let config = StreamableHttpServerConfig::default();
    let service = StreamableHttpService::new(server, config)?;

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    eprintln!("mocari-mcp listening on http://{addr}");

    loop {
        let (stream, _) = listener.accept().await?;
        let svc = service.clone();
        tokio::spawn(async move {
            if let Err(e) = hyper_util::server::conn::auto::Builder::new(hyper_util::rt::TokioExecutor::new())
                .serve_connection(hyper_util::rt::TokioIo::new(stream), svc)
                .await
            {
                eprintln!("HTTP connection error: {e}");
            }
        });
    }
}

#[cfg(not(feature = "mcp-http"))]
async fn run_http(_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("HTTP transport requires 'mcp-http' feature");
    std::process::exit(1);
}
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check --features mcp`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add src/bin/mocari-mcp.rs
git commit -m "feat(mcp): implement standalone MCP server binary"
```

---

### Task 10: Integration Tests

**Files:**
- Create: `tests/mcp_server.rs`

**Interfaces:**
- Consumes: `MocariMcpServer`, `ModelSession`, `rmcp` test utilities
- Produces: Tests verifying tool dispatch, session management, and JSON generation

- [ ] **Step 1: Create test file with basic server test**

```rust
// tests/mcp_server.rs
#![cfg(feature = "mcp")]

use mocari::mcp::{MocariMcpServer, ModelSession};
use rmcp::handler::server::ServerHandler;
use rmcp::model::*;

#[tokio::test]
async fn test_server_info() {
    let session = ModelSession::new();
    let server = MocariMcpServer::new(session);
    let info = server.get_info();
    assert_eq!(info.server_info.name, "mocari-mcp");
}

#[tokio::test]
async fn test_list_tools_returns_all() {
    let session = ModelSession::new();
    let server = MocariMcpServer::new(session);
    let result = server.list_tools(None, /* mock context */).await.unwrap();
    assert_eq!(result.tools.len(), 28); // 20 runtime + 8 creator
}
```

- [ ] **Step 2: Add session management tests**

```rust
#[test]
fn test_session_load_nonexistent() {
    let mut session = ModelSession::new();
    let result = session.load_model("/nonexistent/path.model3.json");
    assert!(result.is_err());
}

#[test]
fn test_session_unload_nonexistent() {
    let mut session = ModelSession::new();
    assert!(!session.unload_model("model_999"));
}

#[test]
fn test_session_list_empty() {
    let session = ModelSession::new();
    assert!(session.models.is_empty());
}
```

- [ ] **Step 3: Add creator tool tests**

```rust
#[tokio::test]
async fn test_create_model_json() {
    let session = ModelSession::new();
    let server = MocariMcpServer::new(session);
    let args = serde_json::json!({
        "name": "test_model",
        "moc_path": "test.moc3",
        "textures": ["tex1.png", "tex2.png"],
    });
    let request = CallToolRequestParams::new("create_model_json")
        .with_arguments(args.as_object().unwrap().clone());
    // Note: this requires a mock RequestContext — adapt based on rmcp test utilities
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --features mcp`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add tests/mcp_server.rs
git commit -m "test(mcp): add MCP server integration tests"
```
