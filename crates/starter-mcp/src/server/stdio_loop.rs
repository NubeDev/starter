//! Read JSON-RPC frames from stdin, dispatch, write responses to
//! stdout. Framing is Content-Length-headered (LSP/MCP-style) and lives
//! in `starter-jsonrpc-stdio` — the same crate the extension supervisor
//! consumes (SCOPE.md "Decisions made — Stdio JSON-RPC framing crate":
//! one framing implementation, no drift).

use std::sync::Arc;

use tokio::io::BufReader;

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

    loop {
        let frame = match starter_jsonrpc_stdio::read_frame(&mut reader).await {
            Ok(Some(bytes)) => bytes,
            Ok(None) => return Ok(()),
            Err(starter_jsonrpc_stdio::FrameError::Io(e)) => return Err(e),
            Err(starter_jsonrpc_stdio::FrameError::Eof) => return Ok(()),
            Err(starter_jsonrpc_stdio::FrameError::Malformed(msg)) => {
                // A malformed frame poisons the stream — we cannot
                // reliably find the next frame boundary. Surface the
                // failure to the consumer's runner as I/O `InvalidData`
                // so it exits with a parseable code rather than hanging.
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, msg));
            }
        };

        // The MCP dispatcher consumes a `&str`; the framing layer hands us
        // raw bytes so the lossy conversion happens at one point and is
        // visible in the diff if it ever needs to be tightened.
        let body = match std::str::from_utf8(&frame) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let response = dispatch(&registry, body).await;
        if let Some(resp) = response {
            let value = serde_json::to_value(&resp).unwrap_or(serde_json::Value::Null);
            // The framing crate's errors are stricter than `io::Error`;
            // map back so the public signature stays unchanged.
            if let Err(e) = starter_jsonrpc_stdio::write_json(&mut stdout, &value).await {
                return Err(match e {
                    starter_jsonrpc_stdio::FrameError::Io(io) => io,
                    other => std::io::Error::other(other.to_string()),
                });
            }
        }
    }
}
