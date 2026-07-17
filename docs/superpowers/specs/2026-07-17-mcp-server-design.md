# MCP Server for Mocari - Design Spec

## Overview

Add an MCP (Model Context Protocol) server to Mocari that exposes Live2D model control and creation capabilities to AI assistants. The server supports both runtime control (loading models, playing motions, adjusting parameters) and model creation (generating JSON sidecar files and basic moc3 binaries).

## Goals

- AI can load, control, and render Live2D models in real-time
- AI can create complete Live2D model files from scratch
- Standalone binary for direct use, library module for integration
- Both stdio and HTTP/SSE transports

## Architecture

Single crate approach: MCP server code lives in `src/mcp/`, standalone binary in `src/bin/mocari-mcp.rs`. Uses `rmcp` crate (Rust MCP SDK) for protocol handling.

### Module Structure

```
src/
  mcp/
    mod.rs           # MCP server setup, transport config
    session.rs       # ModelSession - manages loaded models
    runtime.rs       # Runtime control tools (21 tools)
    creator.rs       # Model creation tools (8 tools)
  bin/
    mocari-mcp.rs    # Standalone server entry point
```

### Dependencies

```toml
[dependencies]
rmcp = { version = "0.1", features = ["server", "transport-io", "transport-sse"] }
tokio = { version = "1", features = ["full"] }
```

## Core Types

### ModelSession

Manages multiple loaded model instances:

```rust
pub struct ModelSession {
    models: HashMap<String, LoadedModel>,
    next_id: u64,
}

struct LoadedModel {
    runtime: ModelRuntime,
    motion_manager: MotionManager,
    expression_manager: ExpressionManager,
    eye_blink: Option<EyeBlink>,
    lip_sync: Option<LipSync>,
    breath: Option<Breath>,
    mouse_tracker: Option<MouseTracker>,
    base_path: PathBuf,
}
```

### MCP Server

```rust
pub struct MocariMcpServer {
    session: Arc<Mutex<ModelSession>>,
}
```

Implements `rmcp::ServerHandler` with tool dispatch.

## Runtime Control Tools

### load_model
- Input: `path: String` (path to .model3.json)
- Output: `{ model_id: String, parameter_count: usize, drawable_count: usize }`
- Loads model, textures, physics, pose. Returns unique model_id.

### unload_model
- Input: `model_id: String`
- Output: `{ success: bool }`

### list_models
- Input: none
- Output: `[{ model_id, path, parameter_count, drawable_count }]`

### list_parameters
- Input: `model_id: String`
- Output: `[{ id, min, max, default, current }]`

### set_parameter
- Input: `model_id: String, parameter_id: String, value: f64`
- Output: `{ success: bool, actual_value: f64 }`

### get_parameter
- Input: `model_id: String, parameter_id: String`
- Output: `{ value: f64, min: f64, max: f64 }`

### list_drawables
- Input: `model_id: String`
- Output: `[{ id, visible, multiply_color, screen_color }]`

### set_drawable_visible
- Input: `model_id: String, drawable_id: String, visible: bool`
- Output: `{ success: bool }`

### set_drawable_color
- Input: `model_id: String, drawable_id: String, multiply: [f64;3], screen: [f64;3]`
- Output: `{ success: bool }`

### play_motion
- Input: `model_id: String, path: String, priority: "idle"|"normal"|"force", group: String`
- Output: `{ success: bool, active_count: usize }`

### stop_motions
- Input: `model_id: String, fade_seconds: f64`
- Output: `{ success: bool }`

### play_expression
- Input: `model_id: String, path: String`
- Output: `{ success: bool }`

### stop_expressions
- Input: `model_id: String`
- Output: `{ success: bool }`

### configure_eye_blink
- Input: `model_id: String, enabled: bool, weight: f64`
- Output: `{ success: bool }`

### configure_lip_sync
- Input: `model_id: String, amplitude: f64, weight: f64`
- Output: `{ success: bool }`

### configure_breath
- Input: `model_id: String, enabled: bool, weight: f64`
- Output: `{ success: bool }`

### configure_mouse_tracker
- Input: `model_id: String, x: f64, y: f64, weight: f64`
- Output: `{ success: bool }`

### configure_physics
- Input: `model_id: String, enabled: bool`
- Output: `{ success: bool }`

### tick
- Input: `model_id: String, delta_seconds: f64`
- Output: `{ success: bool, events: [String] }`
- Advances time, applies all active systems, returns motion events.

### get_state
- Input: `model_id: String`
- Output: Full state snapshot (parameters, drawables, motion status, etc.)

### render_frame
- Input: `model_id: String, width: u32, height: u32`
- Output: `{ image: String (base64 PNG) }`
- Renders current frame using wgpu offscreen, returns base64-encoded PNG.

## Model Creation Tools

### create_model_json
- Input: `{ name, moc_path, textures, motions, expressions, physics_path, pose_path, hit_areas, groups }`
- Output: `{ json: String }`
- Generates a valid model3.json file.

### create_motion_json
- Input: `{ duration, fps, loop, curves: [{ target, id, segments }] }`
- Output: `{ json: String }`
- Generates a valid motion3.json file.

### create_expression_json
- Input: `{ kind, fade_in, fade_out, parameters: [{ id, value, blend, target }] }`
- Output: `{ json: String }`
- Generates a valid exp3.json file.

### create_physics_json
- Input: `{ settings, physics_info: [{ id, input, output, vertices, normalization }] }`
- Output: `{ json: String }`
- Generates a valid physics3.json file.

### create_pose_json
- Input: `{ fade_in_time, groups: [{ id, link }] }`
- Output: `{ json: String }`
- Generates a valid pose3.json file.

### create_userdata_json
- Input: `{ entries: [{ target, id, value }] }`
- Output: `{ json: String }`
- Generates a valid userdata3.json file.

### create_simple_moc3
- Input: `{ name, width, f64, parameters: [{ id, min, max, default }], meshes: [{ vertices, uvs, indices, texture_index }] }`
- Output: `{ binary: String (base64) }`
- Generates a minimal valid .moc3 binary with specified art meshes and parameter bindings.

### create_model_bundle
- Input: `{ name, description, meshes, parameters, motions, expressions }`
- Output: `{ path: String, files: [String] }`
- Creates a complete model directory with all necessary files.

## Standalone Binary

```rust
// src/bin/mocari-mcp.rs
#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.transport {
        Transport::Stdio => run_stdio().await,
        Transport::Http { port } => run_http(port).await,
    }
}
```

CLI:
```
mocari-mcp --transport stdio
mocari-mcp --transport http --port 3000
```

## Library API

```rust
// src/mcp/mod.rs
pub mod session;
pub mod runtime;
pub mod creator;

pub use session::ModelSession;
pub use MocariMcpServer;
```

Users can create `MocariMcpServer` programmatically and integrate into their own MCP setups.

## Error Handling

All tools return structured errors via MCP's error protocol:
- `ModelNotFound` — invalid model_id
- `FileNotFound` — path doesn't exist
- `ParseError` — invalid JSON or moc3 data
- `RenderError` — wgpu rendering failed
- `InvalidInput` — parameter out of range, missing required field

## Testing

- **Unit tests:** ModelSession CRUD, JSON generation round-trips, moc3 binary validity
- **Integration tests:** Load model → set parameter → render → verify output
- **MCP protocol tests:** Tool invocation via mock MCP client

## Typical Usage

```
AI: load_model("assets/models/Hiyori/Hiyori.model3.json")
→ { model_id: "model_1", parameter_count: 50, drawable_count: 42 }

AI: set_parameter("model_1", "ParamAngleX", 30.0)
→ { success: true, actual_value: 30.0 }

AI: play_motion("model_1", "motions/haru_g_idle.motion3.json", "normal", "Idle")
→ { success: true, active_count: 1 }

AI: tick("model_1", 0.016)
→ { success: true, events: [] }

AI: render_frame("model_1", 1024, 1024)
→ { image: "iVBORw0KGgo..." }
```
