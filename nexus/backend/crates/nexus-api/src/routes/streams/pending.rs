//! In-process registry of stream specs awaiting their first subscription.
//!
//! `POST /streams` authenticates and authorizes the subscription, then mints a
//! token the browser opens an `EventSource` against. The token is small and
//! signed, so it carries identity (tenant, datasource, permission) but not the
//! panel SQL. The SQL is parked here under the stream id between create and
//! subscribe. Parking it server-side keeps the query — which the create call
//! already authorized — out of the URL, and means subscribe runs exactly the
//! SQL create vetted, not whatever a crafted token might claim.
//!
//! Live fan-out is single-node for v1 (the broadcast is in-process), so an
//! in-process map is the right scope: a subscription only ever lands on the node
//! that minted its token. Entries expire on the token's own lifetime so an
//! abandoned create call cannot leak a spec forever.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// What a subscriber needs to start the live poll: the vetted SQL and the cadence.
#[derive(Debug, Clone)]
pub struct PendingSpec {
    /// The panel query, already authorized against the datasource at create time.
    pub sql: String,
    /// How often the poll re-runs the query.
    pub interval: Duration,
    expires_at: Instant,
}

fn registry() -> &'static Mutex<HashMap<String, PendingSpec>> {
    static SPECS: OnceLock<Mutex<HashMap<String, PendingSpec>>> = OnceLock::new();
    SPECS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Park a vetted spec under `stream_id`, valid for `ttl`. A repeat id overwrites.
pub fn put(stream_id: &str, sql: String, interval: Duration, ttl: Duration, now: Instant) {
    registry().lock().unwrap().insert(
        stream_id.to_string(),
        PendingSpec {
            sql,
            interval,
            expires_at: now + ttl,
        },
    );
}

/// Take the spec for `stream_id` if it exists and has not expired. Removing it on
/// read means a token cannot be replayed to start a second stream after the
/// first subscriber attached — the spec is consumed once.
pub fn take(stream_id: &str, now: Instant) -> Option<PendingSpec> {
    let spec = registry().lock().unwrap().remove(stream_id)?;
    (spec.expires_at > now).then_some(spec)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn take_consumes_and_respects_expiry() {
        let now = Instant::now();
        put("s1", "SELECT 1".into(), Duration::from_secs(1), Duration::from_secs(60), now);

        // Expired spec is not returned.
        put("s2", "SELECT 2".into(), Duration::from_secs(1), Duration::from_secs(1), now);
        assert!(take("s2", now + Duration::from_secs(2)).is_none());

        // Live spec returns once, then is gone.
        let got = take("s1", now).expect("present");
        assert_eq!(got.sql, "SELECT 1");
        assert!(take("s1", now).is_none(), "consumed on first take");
    }
}
