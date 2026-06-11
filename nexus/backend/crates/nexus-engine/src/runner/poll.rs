//! `PollRunner` — drive a live stream by re-running a query on an interval.
//!
//! The engine has no streaming `sql` input (the SQL query path is sqlx-direct),
//! so a "live SQL panel" is a poll loop, not a push source: each tick re-runs the
//! bounded query and publishes the result to the
//! run id's broadcast channel, the same channel the SSE subscribers read. The
//! producer is supplied by the caller (nexus-api wires it to the guarded sqlx
//! query) so this crate stays free of any store/DB dependency — it owns the
//! cadence and the publish, not the data source.
//!
//! Cancelling the token (the stream registry does this when the last subscriber
//! leaves) stops the loop promptly: the tick wait races the token, so a pending
//! interval never holds the task open past teardown.

use std::future::Future;
use std::time::Duration;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::sink::broadcast_store;

/// Spawn a poll loop publishing to `run_id`'s channel until `token` cancels.
///
/// The channel must already be reserved (the stream registry's `attach` does
/// this). `producer` is invoked once immediately and then every `interval`; each
/// call's rows become one live event. A producer error is logged and skipped —
/// one failed poll does not tear the stream down, since a transient datasource
/// blip should not end every subscriber's panel.
pub fn spawn<F, Fut>(run_id: &str, interval: Duration, token: CancellationToken, producer: F)
where
    F: Fn() -> Fut + Send + 'static,
    Fut: Future<Output = Result<Vec<Value>, String>> + Send,
{
    let run_id = run_id.to_string();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            // Stop as soon as the stream is torn down, even mid-wait.
            tokio::select! {
                _ = token.cancelled() => break,
                _ = ticker.tick() => {}
            }
            // The channel vanishing means teardown raced us; end quietly.
            let Some(channel) = broadcast_store::lookup(&run_id) else {
                break;
            };
            match producer().await {
                Ok(rows) => channel.publish(rows),
                Err(e) => tracing::warn!(run_id = %run_id, error = %e, "live poll failed"),
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn polls_and_publishes_until_cancelled() {
        let run_id = "poll-test-1";
        let channel = broadcast_store::open(run_id);
        let mut rx = channel.subscribe();

        let calls = Arc::new(AtomicU64::new(0));
        let calls_in = calls.clone();
        let token = CancellationToken::new();
        spawn(
            run_id,
            Duration::from_millis(5),
            token.clone(),
            move || {
                let n = calls_in.fetch_add(1, Ordering::Relaxed);
                async move { Ok(vec![json!({ "n": n })]) }
            },
        );

        let first = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("event in time")
            .expect("event");
        let second = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("second event")
            .expect("event");
        assert!(second.seq > first.seq, "monotonic seq across polls");

        token.cancel();
        broadcast_store::close(run_id);
    }
}
