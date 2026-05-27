//! Wire types for the `wall_clock.now_ms` host method.
//!
//! `ctx.wall_clock().now_unix_ms()` returns the host's wall clock
//! to the millisecond. For process-flavour extensions, the SDK
//! marshals each call as a JSON-RPC request so a misbehaving
//! supervisor that pauses the child (e.g. for resource limiting)
//! produces a monotonic-looking clock — the child's own
//! `SystemTime::now()` would not.

use serde::{Deserialize, Serialize};

/// Wire payload an extension sends on `wall_clock.now_ms`. Empty
/// struct so future fields (resolution hint, clock id) can land
/// additively without breaking the wire.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WallClockNowRequest {}

/// Wire response for `wall_clock.now_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WallClockNowResponse {
    /// Unix epoch time in milliseconds.
    pub now_unix_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wall_clock_round_trip() {
        let req = WallClockNowRequest::default();
        let j = serde_json::to_value(&req).unwrap();
        let back: WallClockNowRequest = serde_json::from_value(j).unwrap();
        assert_eq!(back, req);

        let res = WallClockNowResponse {
            now_unix_ms: 1_700_000_000_000,
        };
        let j = serde_json::to_value(res).unwrap();
        let back: WallClockNowResponse = serde_json::from_value(j).unwrap();
        assert_eq!(back, res);
    }
}
