pub mod helpers;
pub mod params;
pub mod server;

use std::sync::{Arc, Mutex, RwLock};

use rmcp::ServiceExt;
use rmcp::transport::stdio;

use wdttg_core::config::AppConfig;
use wdttg_core::storage::cache::MonthCache;
use wdttg_core::storage::file_manager::FileManager;

use crate::server::{McpState, WdttgMcpServer};

/// Start the MCP server on stdio transport. Blocks until the connection closes.
pub async fn run_server(
    config: AppConfig,
    file_manager: FileManager,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let state = Arc::new(McpState {
        config: RwLock::new(config),
        file_manager,
        cache: Mutex::new(MonthCache::default()),
    });

    let server = WdttgMcpServer::new(state);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;

    Ok(())
}
