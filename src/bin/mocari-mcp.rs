use clap::Parser;
use mocari::mcp::{MocariMcpServer, ModelSession};
use rmcp::ServiceExt;

#[derive(Parser, Debug)]
#[command(name = "mocari-mcp", about = "Mocari MCP server for Live2D model control")]
struct Args {
    /// Transport to use: "stdio" or "http"
    #[arg(long, default_value = "stdio")]
    transport: String,

    /// Port for HTTP transport
    #[arg(long, default_value_t = 3000)]
    port: u16,
}

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

#[cfg(feature = "mcp-http")]
async fn run_http(_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("HTTP transport not yet fully implemented — use stdio");
    std::process::exit(1);
}

#[cfg(not(feature = "mcp-http"))]
async fn run_http(_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("HTTP transport requires the 'mcp-http' feature");
    std::process::exit(1);
}
