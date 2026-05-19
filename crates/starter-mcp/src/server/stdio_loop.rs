//! Read JSON-RPC frames from stdin, dispatch, write responses to
//! stdout. Each frame is one line of UTF-8 JSON.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::registry::ToolRegistry;

use super::dispatch::dispatch;

/// Run the MCP stdio loop until stdin closes.
///
/// `registry` is consumed and shared across requests. The loop is
/// single-threaded — MCP clients send one request at a time over
/// stdio so concurrency would add complexity without throughput.
pub async fn run_stdio(registry: ToolRegistry) -> std::io::Result<()> {
    let registry = Arc::new(registry);
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader.read_line(&mut line).await?;
        if read == 0 {
            return Ok(());
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let response = dispatch(&registry, trimmed).await;
        if let Some(resp) = response {
            let bytes = serde_json::to_vec(&resp).unwrap_or_else(|_| b"{}".to_vec());
            stdout.write_all(&bytes).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }
}
