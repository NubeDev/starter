//! Registry of running live streams, keyed by the full subscription spec.
//!
//! Two subscribers share one underlying live pipeline **only** when their whole
//! spec matches — the stream definition, the datasource, the tenant, and the
//! required permission. Keying on the source alone would let one tenant's
//! subscribers receive another tenant's data, so the tenant and permission are
//! part of the key, not an afterthought.
//!
//! Each running stream is reference-counted by its live subscribers. The last
//! subscriber to drop cancels the stream's token and releases its broadcast
//! channel, so an idle live source does not keep running.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

use crate::sink::broadcast_store::{self, LiveChannel};
use nexus_spi::dto::stream::StreamEvent;

/// The identity of a live stream. Equality on this whole tuple is what decides
/// whether a new subscription can attach to an already-running stream.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StreamKey {
    /// Canonical serialization of the stream spec (input + pipeline).
    pub spec: String,
    /// Datasource the stream reads from.
    pub datasource_id: String,
    /// Owning tenant — part of the key so tenants never share a broadcast.
    pub tenant_id: String,
    /// Permission required to subscribe.
    pub permission: String,
}

struct Running {
    run_id: String,
    channel: LiveChannel,
    token: CancellationToken,
    refcount: usize,
}

/// A live subscription. Holding it keeps the underlying stream alive; dropping
/// it decrements the refcount and tears the stream down when it reaches zero.
pub struct Subscription {
    key: StreamKey,
    receiver: broadcast::Receiver<StreamEvent>,
}

impl Subscription {
    /// The receiver for this subscription's events.
    pub fn receiver(&mut self) -> &mut broadcast::Receiver<StreamEvent> {
        &mut self.receiver
    }
}

impl Drop for Subscription {
    fn drop(&mut self) {
        release(&self.key);
    }
}

fn registry() -> &'static Mutex<HashMap<StreamKey, Running>> {
    static STREAMS: OnceLock<Mutex<HashMap<StreamKey, Running>>> = OnceLock::new();
    STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Outcome of asking the registry to subscribe to `key`.
pub enum Attach {
    /// An existing stream matched; subscribe to it. Carries the subscription.
    Existing(Subscription),
    /// No stream matched; the caller must start one with [`register`], passing
    /// the `run_id` it reserved.
    StartNew { run_id: String },
}

/// Try to attach to a running stream for `key`. If one exists, returns a
/// subscription with its refcount incremented; otherwise reserves a broadcast
/// channel under a fresh `run_id` and asks the caller to start the stream.
pub fn attach(key: &StreamKey, run_id: &str) -> Attach {
    let mut map = registry().lock().unwrap();
    if let Some(running) = map.get_mut(key) {
        running.refcount += 1;
        return Attach::Existing(Subscription {
            key: key.clone(),
            receiver: running.channel.subscribe(),
        });
    }
    broadcast_store::open(run_id);
    Attach::StartNew {
        run_id: run_id.to_string(),
    }
}

/// Record a freshly-started stream and return the first subscription to it. The
/// `token` cancels the stream; the channel is the one reserved under `run_id`.
pub fn register(key: StreamKey, run_id: String, token: CancellationToken) -> Subscription {
    let mut map = registry().lock().unwrap();
    let channel = broadcast_store::lookup(&run_id).expect("channel reserved by attach()");
    let receiver = channel.subscribe();
    map.insert(
        key.clone(),
        Running {
            run_id,
            channel,
            token,
            refcount: 1,
        },
    );
    Subscription { key, receiver }
}

/// Decrement the refcount for `key`; on the last subscriber, cancel the stream
/// and release its channel.
fn release(key: &StreamKey) {
    let mut map = registry().lock().unwrap();
    if let Some(running) = map.get_mut(key) {
        running.refcount -= 1;
        if running.refcount == 0 {
            running.token.cancel();
            broadcast_store::close(&running.run_id);
            map.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(tenant: &str) -> StreamKey {
        StreamKey {
            spec: "spec".into(),
            datasource_id: "ds".into(),
            tenant_id: tenant.into(),
            permission: "view".into(),
        }
    }

    #[test]
    fn different_tenants_do_not_share_a_stream() {
        let a = key("acme");
        let b = key("globex");
        assert_ne!(a, b, "tenant is part of the key");
    }

    #[tokio::test]
    async fn last_subscriber_drop_cancels_the_stream() {
        let k = key("acme");
        let run_id = "run-test-1";
        // First attach reserves the channel and asks to start.
        match attach(&k, run_id) {
            Attach::StartNew { run_id } => {
                let token = CancellationToken::new();
                let token_probe = token.clone();
                let sub = register(k.clone(), run_id, token);

                // A second subscriber shares the running stream.
                let sub2 = match attach(&k, "ignored") {
                    Attach::Existing(s) => s,
                    Attach::StartNew { .. } => panic!("should have attached to existing"),
                };

                assert!(!token_probe.is_cancelled());
                drop(sub);
                assert!(!token_probe.is_cancelled(), "still one subscriber left");
                drop(sub2);
                assert!(token_probe.is_cancelled(), "last drop cancels the stream");
            }
            Attach::Existing(_) => panic!("first attach must start new"),
        }
    }
}
