//! Streaming-notification passthrough.
//!
//! Per SCOPE.md post-R13 ("JSON-RPC streaming convention lives in
//! `starter-ext-spi`"): the kernel shape is four reserved notifications
//! (`stream.event` / `stream.end` / `stream.error` / `stream.cancel`)
//! tagged with a `stream_id`. The supervisor *does not interpret these
//! payloads* — it forwards them to whichever adapter opened the stream
//! and lets that adapter translate to its transport's native frame (SSE,
//! gRPC server-streaming, MCP notifications).
//!
//! In v0.1 that means: when the JSON-RPC reader sees a notification
//! whose method begins with `stream.`, it pushes the raw envelope onto
//! the per-extension outbound channel. Adapters subscribe to that
//! channel; the supervisor itself never decides what to do with the
//! chunks. This module is the one-line classifier the reader uses to
//! split substrate frames from stream-passthrough frames.

use starter_ext_spi::jsonrpc::stream_methods;

/// `true` when `method` is one of the four reserved streaming
/// notifications. Adapters that listen on the supervisor's outbound
/// channel use the same classifier to know whether the frame is theirs.
#[inline]
pub fn is_streaming_notification(method: &str) -> bool {
    method == stream_methods::EVENT
        || method == stream_methods::END
        || method == stream_methods::ERROR
        || method == stream_methods::CANCEL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_all_four() {
        assert!(is_streaming_notification("stream.event"));
        assert!(is_streaming_notification("stream.end"));
        assert!(is_streaming_notification("stream.error"));
        assert!(is_streaming_notification("stream.cancel"));
    }

    #[test]
    fn rejects_non_stream_methods() {
        assert!(!is_streaming_notification("init"));
        assert!(!is_streaming_notification("health"));
        assert!(!is_streaming_notification("stream"));
        assert!(!is_streaming_notification("stream.unknown"));
    }
}
