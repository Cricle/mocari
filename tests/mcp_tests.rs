#![cfg(feature = "mcp")]
#![forbid(unsafe_code)]

mod helpers;
#[path = "mcp/server_info.rs"]
mod server_info;
#[path = "mcp/session.rs"]
mod session;
#[path = "mcp/runtime_handlers.rs"]
mod runtime_handlers;
#[path = "mcp/creator_handlers.rs"]
mod creator_handlers;
#[path = "mcp/dispatch.rs"]
mod dispatch;
