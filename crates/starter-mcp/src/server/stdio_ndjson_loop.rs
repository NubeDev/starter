//! Newline-delimited JSON-RPC over stdio — the framing every real MCP
//! host (Claude Code, Codex CLI, Copilot, the official @modelcontextprotocol
//! TypeScript/Python SDKs) uses. One JSON object per line on stdin,
//! one per line on stdout. No `Content-Length` headers.
//!
//! Kept separate from [`run_stdio`](super::run_stdio) so the existing
//! Content-Length-framed loop (used by `starter-ext-supervisor`) stays
//! byte-for-byte unchanged.

use std::sync::Arc;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use starter_spi::i18n::LanguageTag;

use crate::registry::ToolRegistry;

use super::dispatch::dispatch;
use super::stdio_loop_locale::locale_from_initialize_frame;

/// Run the MCP ndjson stdio loop until stdin closes.
///
/// Each line on stdin is one JSON-RPC message. Each response is
/// written as a single line on stdout terminated by `\n`. Blank lines
/// and lines that fail UTF-8 decoding are skipped (a malformed line
/// only loses that one message, unlike the header-framed transport
/// where a bad frame poisons the stream).
pub async fn run_stdio_ndjson(registry: ToolRegistry) -> std::io::Result<()> {
    let registry = Arc::new(registry);
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut lines = BufReader::new(stdin).lines();

    let mut session_locale: Option<LanguageTag> = None;

    while let Some(line) = lines.next_line().await? {
        let body = line.trim();
        if body.is_empty() {
            continue;
        }

        if let Some(tag) = locale_from_initialize_frame(body) {
            session_locale = Some(tag);
        }

        let response = match session_locale.clone() {
            Some(tag) => crate::with_locale(tag, dispatch(&registry, body)).await,
            None => dispatch(&registry, body).await,
        };
        if let Some(resp) = response {
            let mut bytes = serde_json::to_vec(&resp).unwrap_or_else(|_| b"null".to_vec());
            bytes.push(b'\n');
            stdout.write_all(&bytes).await?;
            stdout.flush().await?;
        }
    }
    Ok(())
}
