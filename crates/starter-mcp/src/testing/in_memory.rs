//! A bidirectional in-memory channel pair that lets tests exercise
//! the dispatch loop without spawning a subprocess.

/// Paired transports — one side calls `send_request` and `recv_response`,
/// the other is driven by [`crate::server::dispatch`].
///
/// Stubbed for v0.1; the full implementation lands with the
/// dispatch body.
pub struct InMemoryTransport {
    _private: (),
}
