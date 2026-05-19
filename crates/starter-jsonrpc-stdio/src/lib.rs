//! # starter-jsonrpc-stdio
//!
//! Content-Length-framed JSON-RPC 2.0 over async stdio.
//!
//! LSP / MCP-style framing: each message is preceded by a header block
//! terminated by an empty line. The only header this crate inspects is
//! `Content-Length` (decimal byte length of the JSON body that follows).
//! Other headers are read and discarded so a peer that emits extra metadata
//! (`Content-Type`, custom annotations) is forward-compatible.
//!
//! ## Why one crate
//!
//! The decision lives in `DOCS/extensions/scope/SCOPE.md` under "Decisions
//! made — Stdio JSON-RPC framing crate":
//!
//! > Extract `starter-jsonrpc-stdio` as a small crate in the `starter`
//! > workspace, consumed by both `starter-mcp` and
//! > `starter-ext-supervisor`. Content-Length-framed JSON-RPC 2.0 is the
//! > same wire format in both worlds; duplicating it twice would invite
//! > drift.
//!
//! The crate is deliberately tiny: a [`read_frame`] / [`write_frame`] pair
//! plus the typed errors. No JSON-RPC method dispatch lives here — that
//! belongs in the consumer (the MCP server, the extension supervisor) so
//! one framing change touches one file regardless of how many higher-level
//! protocols use it.
//!
//! ## Wire shape
//!
//! ```text
//! Content-Length: 42\r\n
//! \r\n
//! {"jsonrpc":"2.0","id":1,"method":"ping"}
//! ```
//!
//! - Headers use `\r\n` line endings (the LSP spec); the reader also
//!   accepts bare `\n` so a hand-typed test fixture works.
//! - Header names are case-insensitive (matched against `content-length`).
//! - The body is `Content-Length` *bytes*, read verbatim. No trailing
//!   newline is implied — the next frame's headers start immediately.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use thiserror::Error;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Failure modes for [`read_frame`] / [`write_frame`].
///
/// The variants are deliberately coarse — a consumer that wants to surface
/// transport-level diagnostics maps them onto its own taxonomy (`starter-mcp`
/// produces JSON-RPC `-32700 parse error`; `starter-ext-supervisor` raises
/// `starter_ext_spi::Error::Transport`).
#[derive(Debug, Error)]
pub enum FrameError {
    /// Underlying I/O failed (pipe closed mid-read, write to broken pipe, …).
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Peer hung up cleanly between frames. Distinct from [`Self::Io`] so
    /// the consumer's loop can break without logging an error.
    #[error("eof")]
    Eof,

    /// The header block did not contain a `Content-Length` line, or its
    /// value was not a valid decimal integer.
    #[error("malformed frame: {0}")]
    Malformed(String),
}

/// Read one frame from `reader`. Returns `Ok(None)` on clean EOF between
/// frames; returns `Ok(Some(bytes))` with the raw JSON body otherwise.
///
/// The body is returned as `Vec<u8>` so the caller chooses whether to
/// deserialise into a typed envelope, parse as `serde_json::Value`, or
/// forward verbatim. Keeping the framing crate untyped is what lets MCP
/// and the extension supervisor — which want different envelope types —
/// share it.
pub async fn read_frame<R>(reader: &mut R) -> Result<Option<Vec<u8>>, FrameError>
where
    R: AsyncBufRead + Unpin,
{
    let mut content_length: Option<usize> = None;
    let mut saw_any_header = false;
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            // Clean EOF.
            return if saw_any_header {
                Err(FrameError::Malformed(
                    "stream ended inside header block".into(),
                ))
            } else {
                Ok(None)
            };
        }

        // Tolerate `\n`-only line endings as well as `\r\n`.
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            // End of header block.
            break;
        }
        saw_any_header = true;

        let (name, value) = match trimmed.split_once(':') {
            Some((n, v)) => (n.trim(), v.trim()),
            None => {
                return Err(FrameError::Malformed(format!(
                    "header line missing ':' — {trimmed:?}"
                )));
            }
        };
        if name.eq_ignore_ascii_case("content-length") {
            content_length = Some(value.parse().map_err(|_| {
                FrameError::Malformed(format!("invalid Content-Length value: {value:?}"))
            })?);
        }
        // Other headers are intentionally ignored for forward compatibility.
    }

    let len = content_length
        .ok_or_else(|| FrameError::Malformed("missing Content-Length header".into()))?;

    let mut body = vec![0u8; len];
    // Read exact-length body. EOF mid-body is a protocol violation —
    // distinct from EOF-between-frames.
    AsyncReadExt::read_exact(reader, &mut body)
        .await
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::UnexpectedEof {
                FrameError::Malformed("stream ended inside body".into())
            } else {
                FrameError::Io(e)
            }
        })?;
    Ok(Some(body))
}

/// Write one frame to `writer`. `body` is the JSON document; this function
/// prepends the `Content-Length` header, the blank line, and the body, then
/// flushes.
///
/// Flushing on every frame is the right default for a request/response
/// protocol: a JSON-RPC peer that buffers indefinitely deadlocks. Callers
/// that want their own batching can wrap a `BufWriter` and override the
/// flush behaviour at that layer.
pub async fn write_frame<W>(writer: &mut W, body: &[u8]) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
{
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(body).await?;
    writer.flush().await?;
    Ok(())
}

/// Convenience: read one frame and deserialise the body into `serde_json::Value`.
///
/// Equivalent to `read_frame` + `serde_json::from_slice`, but the framing
/// errors and the JSON errors land in the same `FrameError` enum so the
/// consumer's loop does not branch on two error types.
pub async fn read_json<R>(reader: &mut R) -> Result<Option<serde_json::Value>, FrameError>
where
    R: AsyncBufRead + Unpin,
{
    match read_frame(reader).await? {
        Some(bytes) => {
            let v: serde_json::Value = serde_json::from_slice(&bytes)
                .map_err(|e| FrameError::Malformed(format!("invalid JSON body: {e}")))?;
            Ok(Some(v))
        }
        None => Ok(None),
    }
}

/// Convenience: serialise a value and write it as one frame.
pub async fn write_json<W>(writer: &mut W, value: &serde_json::Value) -> Result<(), FrameError>
where
    W: AsyncWrite + Unpin,
{
    let body = serde_json::to_vec(value)
        .map_err(|e| FrameError::Malformed(format!("serialising body: {e}")))?;
    write_frame(writer, &body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::BufReader;

    fn frame(body: &str) -> Vec<u8> {
        let mut out = format!("Content-Length: {}\r\n\r\n", body.len()).into_bytes();
        out.extend_from_slice(body.as_bytes());
        out
    }

    #[tokio::test]
    async fn round_trip_one_frame() {
        let mut out: Vec<u8> = Vec::new();
        write_frame(&mut out, b"{\"ok\":true}").await.unwrap();
        let mut reader = BufReader::new(&out[..]);
        let body = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(body, b"{\"ok\":true}");
    }

    #[tokio::test]
    async fn reads_two_frames_in_a_row() {
        let mut buf = frame(r#"{"a":1}"#);
        buf.extend_from_slice(&frame(r#"{"b":2}"#));
        let mut reader = BufReader::new(&buf[..]);
        let a = read_frame(&mut reader).await.unwrap().unwrap();
        let b = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(a, b"{\"a\":1}");
        assert_eq!(b, b"{\"b\":2}");
    }

    #[tokio::test]
    async fn eof_between_frames_is_none() {
        let buf: &[u8] = b"";
        let mut reader = BufReader::new(buf);
        assert!(read_frame(&mut reader).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn missing_content_length_is_malformed() {
        let buf: &[u8] = b"X-Foo: bar\r\n\r\n";
        let mut reader = BufReader::new(buf);
        let err = read_frame(&mut reader).await.unwrap_err();
        assert!(matches!(err, FrameError::Malformed(_)));
    }

    #[tokio::test]
    async fn case_insensitive_header_name() {
        let buf: &[u8] = b"content-length: 7\r\n\r\n{\"a\":1}";
        let mut reader = BufReader::new(buf);
        let body = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(body, b"{\"a\":1}");
    }

    #[tokio::test]
    async fn additional_headers_ignored() {
        let buf: &[u8] = b"Content-Type: application/vscode-jsonrpc\r\nContent-Length: 7\r\n\r\n{\"a\":1}";
        let mut reader = BufReader::new(buf);
        let body = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(body, b"{\"a\":1}");
    }

    #[tokio::test]
    async fn truncated_body_is_malformed() {
        // Header advertises 10 bytes but only 3 follow.
        let buf: &[u8] = b"Content-Length: 10\r\n\r\nabc";
        let mut reader = BufReader::new(buf);
        let err = read_frame(&mut reader).await.unwrap_err();
        assert!(matches!(err, FrameError::Malformed(_)));
    }

    #[tokio::test]
    async fn write_then_read_json_round_trip() {
        let mut out: Vec<u8> = Vec::new();
        let v = serde_json::json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
        write_json(&mut out, &v).await.unwrap();
        let mut reader = BufReader::new(&out[..]);
        let back = read_json(&mut reader).await.unwrap().unwrap();
        assert_eq!(back, v);
    }
}
