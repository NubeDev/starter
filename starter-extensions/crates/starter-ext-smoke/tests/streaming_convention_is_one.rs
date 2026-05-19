//! SCOPE smoke: "Same-source streams over four transports".
//!
//! The four transports (MCP notifications/progress, SSE on REST,
//! line-delimited stdout on CLI, gRPC server-streaming) must share one
//! wire convention so a single contributing extension can serve all of
//! them without per-transport handler forks (R13). The convention lives
//! in `starter-ext-spi::jsonrpc::stream` — four named notification
//! methods (`stream.event`, `stream.end`, `stream.error`,
//! `stream.cancel`) that adapters re-render into their transport-native
//! framing.
//!
//! This smoke pins the names. Adapter-level rendering tests live next
//! to each adapter:
//!
//!   - REST SSE/NDJSON + client-disconnect → stream.cancel:
//!     `starter-ext-server/tests/rest_routes.rs`
//!   - CLI line-delimited stdout + SIGINT → stream.cancel:
//!     `starter-ext-cli/tests/hello_cli_end_to_end.rs`
//!   - Supervisor forwards stream.* without interpretation:
//!     `starter-ext-supervisor/src/stream.rs`
//!   - MCP notifications/progress + gRPC server-streaming: Adapter
//!     Phase 8 (`starter-ext-grpc`) lands the gRPC leg on the same
//!     convention; the MCP leg lives in `starter-ext-mcp` once the
//!     streaming surface stabilises.

use starter_ext_spi::jsonrpc::stream_methods as stream;
use starter_ext_supervisor::is_streaming_notification;

#[test]
fn streaming_method_names_are_the_canonical_four() {
    assert_eq!(stream::EVENT, "stream.event");
    assert_eq!(stream::END, "stream.end");
    assert_eq!(stream::ERROR, "stream.error");
    assert_eq!(stream::CANCEL, "stream.cancel");
}

#[test]
fn supervisor_recognises_only_the_canonical_four() {
    for m in [stream::EVENT, stream::END, stream::ERROR, stream::CANCEL] {
        assert!(
            is_streaming_notification(m),
            "the supervisor must forward {m} as a stream notification \
             without re-interpreting it (R13)"
        );
    }
    // Anything else is *not* a stream notification — the supervisor
    // must keep it on the request/response path.
    for m in ["init", "health", "shutdown", "stream.unknown", "stream"] {
        assert!(
            !is_streaming_notification(m),
            "{m:?} must not be treated as a streaming notification"
        );
    }
}
