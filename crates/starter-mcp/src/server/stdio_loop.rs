//! Read JSON-RPC frames from stdin, dispatch, write responses to
//! stdout. Framing is Content-Length-headered (LSP/MCP-style) and lives
//! in `starter-jsonrpc-stdio` — the same crate the extension supervisor
//! consumes (SCOPE.md "Decisions made — Stdio JSON-RPC framing crate":
//! one framing implementation, no drift).

use std::sync::Arc;

use tokio::io::BufReader;

use starter_spi::i18n::LanguageTag;

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

    // Session-wide locale captured at the `initialize` handshake.
    // HTTP carries `Accept-Language` per request; stdio is a single
    // long-lived session, so the locale is negotiated once via
    // `params._meta.acceptLanguage` and held for every subsequent
    // `tools/call`. See `docs/design/i18n-prefs/README.md` and
    // `docs/design/starter-changes/README.md` (Phase 2b U1).
    let mut session_locale: Option<LanguageTag> = None;

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

        if let Some(tag) = locale_from_initialize_frame(body) {
            session_locale = Some(tag);
        }

        let response = match session_locale.clone() {
            Some(tag) => crate::with_locale(tag, dispatch(&registry, body)).await,
            None => dispatch(&registry, body).await,
        };
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

/// Inspect a raw JSON-RPC frame; if it is an `initialize` request and
/// carries `params._meta.acceptLanguage` (the MCP `_meta` convention
/// for transport-level hints), pick the highest-quality BCP-47 tag.
///
/// Returns `None` for non-`initialize` frames, missing `_meta`, or
/// values that fail BCP-47 validation. A garbled frame is a hint, not
/// a contract — the caller falls back to no locale binding.
fn locale_from_initialize_frame(raw: &str) -> Option<LanguageTag> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    if value.get("method")?.as_str()? != "initialize" {
        return None;
    }
    let header = value
        .get("params")?
        .get("_meta")?
        .get("acceptLanguage")?
        .as_str()?;
    crate::locale_local::locale_from_accept_language(header)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_from_initialize_frame_reads_meta_accept_language() {
        let frame = r#"{"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"_meta":{"acceptLanguage":"es-AR"}}}"#;
        let tag = locale_from_initialize_frame(frame).unwrap();
        assert_eq!(tag.as_str(), "es-AR");
    }

    #[test]
    fn locale_from_initialize_frame_ignores_non_initialize() {
        let frame = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list",
            "params":{"_meta":{"acceptLanguage":"es-AR"}}}"#;
        assert!(locale_from_initialize_frame(frame).is_none());
    }

    #[test]
    fn locale_from_initialize_frame_none_when_meta_absent() {
        let frame = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        assert!(locale_from_initialize_frame(frame).is_none());
    }

    #[test]
    fn locale_from_initialize_frame_none_for_malformed_json() {
        assert!(locale_from_initialize_frame("not json").is_none());
    }
}
