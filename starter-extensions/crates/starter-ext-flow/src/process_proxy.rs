//! `ProcessNodeProxy` — bridges an engine [`NodeBehavior::invoke`] call
//! onto the extension's child process via `flow.node.invoke`
//! (`DOCS/extensions/scope/FLOW-NODES.md` R-flow-node-1 +
//! R-flow-node-5).
//!
//! The proxy owns one invocation: it
//!
//! 1. mints a `StreamId` from `ulid::Ulid::new()` (the proxy-owned
//!    correlation key, *not* the JSON-RPC `id` — R-flow-node-5);
//! 2. subscribes to `stream.*` notifications keyed by that
//!    `stream_id` *before* it issues the call so a fast child cannot
//!    emit a `stream.event` before the proxy is listening;
//! 3. forwards `ctx.cancel` trips as `stream.cancel { stream_id }`
//!    via [`SupervisorHandle::stream_cancel`] while still awaiting
//!    the call response (the cancel envelope is one-shot; the proxy
//!    waits for the child's own NODE_CANCELLED response or
//!    `stream.cancel` echo before returning [`NodeError::Cancelled`]);
//! 4. drives a single [`SupervisorHandle::call`] for
//!    [`FLOW_NODE_INVOKE`] (the host-side timeout is the hard bound;
//!    the manifest's `deadline_ms` is advisory and forwarded as a
//!    param);
//! 5. for streaming kinds, drains stream notifications into a
//!    collected output slot and returns the accumulated batch when
//!    the stream terminates (normal `stream.end`, child-side cancel,
//!    or `stream.error`); for non-streaming kinds the response
//!    payload alone populates the output [`SlotMap`].
//!
//! Drop guards: the stream subscription is unregistered on every
//! exit path so a completed invocation never leaks state in the
//! supervisor.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use starter_ext_spi::jsonrpc::flow_node_error_codes;
use starter_ext_spi::{Error as ExtError, StreamId, StreamNotification, FLOW_NODE_INVOKE};
use starter_ext_supervisor::SupervisorHandle;
use starter_flow_spi::node::{KindId, NodeBehavior, NodeCtx, NodeError, SlotMap, SlotValue};

/// Default host-side call timeout if the proxy's owner did not
/// override it. Long enough to outlast reasonable per-node work
/// without holding a stuck child forever; the host can shorten by
/// constructing the proxy through [`ProcessNodeProxy::with_timeout`].
const DEFAULT_INVOKE_TIMEOUT: Duration = Duration::from_secs(60);

/// One end of a child-process bridge for one node kind.
///
/// Cheap to clone; the underlying [`SupervisorHandle`] is itself
/// `Arc`-backed. One proxy serves every invocation of one kind
/// (cardinality is per-kind, not per-invocation — the kind's
/// per-invocation state lives entirely in the in-flight async task,
/// not on the proxy).
pub struct ProcessNodeProxy {
    kind: KindId,
    supervisor: SupervisorHandle,
    /// Advisory deadline forwarded to the child as `deadline_ms` on
    /// every `flow.node.invoke` (R-flow-node-5). Host-side timeout is
    /// the authoritative bound; this field is purely a hint so a
    /// well-behaved child can short-circuit its own work.
    advisory_deadline: Duration,
    /// Hard host-side timeout for the underlying
    /// [`SupervisorHandle::call`]. Always ≥ `advisory_deadline` so a
    /// child that respects the advisory hint wins the race; expiry
    /// returns [`NodeError::Backend("timeout")`] and best-effort
    /// issues a `stream.cancel` so the child can wind down.
    host_timeout: Duration,
    /// `true` for streaming kinds (advisory: per
    /// `block.yaml.contributes.nodes[].streaming`). Streaming proxies
    /// drain `stream.event` chunks into the output `events` slot
    /// until the stream terminates; non-streaming proxies use only
    /// the initial response.
    streaming: bool,
}

impl ProcessNodeProxy {
    /// Construct a proxy. Uses [`DEFAULT_INVOKE_TIMEOUT`] for the
    /// host-side hard bound.
    pub fn new(kind: KindId, supervisor: SupervisorHandle, streaming: bool) -> Self {
        Self::with_timeout(
            kind,
            supervisor,
            streaming,
            DEFAULT_INVOKE_TIMEOUT,
            DEFAULT_INVOKE_TIMEOUT,
        )
    }

    /// Construct a proxy with explicit timeouts. The host-side hard
    /// bound (`host_timeout`) should be ≥ `advisory_deadline` so a
    /// child that respects the advisory hint is never killed by the
    /// host's authoritative timer first (R-flow-node-5).
    pub fn with_timeout(
        kind: KindId,
        supervisor: SupervisorHandle,
        streaming: bool,
        advisory_deadline: Duration,
        host_timeout: Duration,
    ) -> Self {
        Self {
            kind,
            supervisor,
            advisory_deadline,
            host_timeout,
            streaming,
        }
    }

    /// Borrow the underlying supervisor handle (mainly for tests).
    pub fn supervisor(&self) -> &SupervisorHandle {
        &self.supervisor
    }
}

/// Params sent on `flow.node.invoke`.
///
/// `serde(deny_unknown_fields)` so a child that hand-rolls the
/// dispatcher cannot quietly mis-spell a field name and have the
/// host ignore it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InvokeParams {
    /// Reverse-DNS kind id the host wants invoked.
    kind: String,
    /// Proxy-owned correlation id (`inv-<ulid>`). Doubles as the
    /// `stream_id` for any `stream.*` notification the invocation
    /// produces.
    invocation_id: String,
    /// Host's advisory deadline in milliseconds. Children that
    /// support it should refuse to start work it cannot finish; the
    /// host-side timeout is the hard bound either way.
    deadline_ms: u64,
    /// Input slots the engine handed to `NodeBehavior::invoke`.
    input: SlotMap,
}

#[async_trait]
impl NodeBehavior for ProcessNodeProxy {
    fn kind_id(&self) -> &KindId {
        &self.kind
    }

    async fn invoke(&self, ctx: NodeCtx<'_>, input: SlotMap) -> Result<SlotMap, NodeError> {
        // 1. mint invocation id (proxy-owned; R-flow-node-5).
        let invocation_id = StreamId(format!("inv-{}", ulid::Ulid::new()));

        // 2. subscribe BEFORE issuing the call so a fast child cannot
        //    race past our recv.
        let mut stream_rx = self.supervisor.subscribe_stream(&invocation_id);

        // Drop guard: regardless of exit path, unregister our stream
        // subscription so the supervisor's map doesn't leak.
        let _cleanup = scopeguard_for({
            let supervisor = self.supervisor.clone();
            let invocation_id = invocation_id.clone();
            move || supervisor.unsubscribe_stream(&invocation_id)
        });

        // 3. issue the host call.
        let params = InvokeParams {
            kind: self.kind.as_str().to_owned(),
            invocation_id: invocation_id.0.clone(),
            deadline_ms: self.advisory_deadline.as_millis().min(u64::MAX as u128) as u64,
            input,
        };
        let params_value = serde_json::to_value(&params)
            .map_err(|e| NodeError::Backend(format!("serialising flow.node.invoke params: {e}")))?;

        // 4. drive call + cancel forwarder in a select! so we can
        //    forward `ctx.cancel` trips without spawning a 'static
        //    task that outlives the borrowed `ctx.cancel`.
        let call_fut = self
            .supervisor
            .call(FLOW_NODE_INVOKE, params_value, self.host_timeout);
        tokio::pin!(call_fut);

        let mut cancel_forwarded = false;
        let response = loop {
            tokio::select! {
                biased;
                _ = ctx.cancel.cancelled(), if !cancel_forwarded => {
                    // R-flow-node-5: cancel envelope is keyed by
                    // invocation_id (the proxy-owned correlation), not
                    // by the JSON-RPC `id`.
                    let _ = self
                        .supervisor
                        .stream_cancel(&invocation_id, Some("run cancelled"));
                    cancel_forwarded = true;
                    // Keep awaiting the call; the child either acks
                    // with the carved NODE_CANCELLED error or returns
                    // normally (the latter is fine — we still report
                    // Cancelled because the engine asked for it).
                }
                res = &mut call_fut => break res,
            }
        };

        if cancel_forwarded {
            // The engine asked us to cancel; honour that regardless
            // of which response the child eventually sent.
            return Err(NodeError::Cancelled);
        }

        let response = match response {
            Ok(v) => v,
            Err(e) => {
                let msg = match &e {
                    ExtError::Transport(m) => m.clone(),
                    other => other.to_string(),
                };
                if msg.contains("timed out") {
                    let _ = self
                        .supervisor
                        .stream_cancel(&invocation_id, Some("host timeout"));
                    return Err(NodeError::Backend(format!("timeout: {msg}")));
                }
                // Map flow-node carve-out codes onto typed NodeError.
                if let ExtError::ExtensionInternal(payload) = &e {
                    if let Some(node_err) = map_carved_error(payload) {
                        return Err(node_err);
                    }
                }
                if matches!(e, ExtError::Transport(_)) {
                    // Transport failure typically means the child
                    // crashed mid-call — surface as Backend so the
                    // engine emits NodeFailed.
                    return Err(NodeError::Backend(format!(
                        "flow.node.invoke transport: {e}"
                    )));
                }
                return Err(NodeError::Backend(format!("flow.node.invoke: {e}")));
            }
        };

        // 5. For non-streaming kinds the initial response payload is
        //    the entire output `SlotMap`. For streaming kinds the
        //    response is the open-stream ack; the proxy drains
        //    `stream.event` chunks until the stream terminates.
        let out = if self.streaming {
            drain_stream(&mut stream_rx, response).await?
        } else {
            parse_response_as_slotmap(response).map_err(|e| {
                NodeError::Backend(format!(
                    "flow.node.invoke returned non-SlotMap response: {e}"
                ))
            })?
        };
        Ok(out)
    }
}

fn parse_response_as_slotmap(value: serde_json::Value) -> Result<SlotMap, serde_json::Error> {
    if let Some(obj) = value.as_object() {
        if let Some(slots) = obj.get("slots") {
            return serde_json::from_value::<SlotMap>(slots.clone());
        }
    }
    serde_json::from_value::<SlotMap>(value)
}

/// Drain `stream.event` chunks into an `events` slot until the
/// stream terminates.
async fn drain_stream(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<StreamNotification>,
    initial: serde_json::Value,
) -> Result<SlotMap, NodeError> {
    let mut out: SlotMap = parse_response_as_slotmap(initial).unwrap_or_default();
    let mut events: Vec<serde_json::Value> = Vec::new();
    loop {
        match rx.recv().await {
            Some(StreamNotification::Event(e)) => events.push(e.payload),
            Some(StreamNotification::End(e)) => {
                if let Some(p) = e.payload {
                    if let Some(obj) = p.as_object() {
                        for (k, v) in obj.iter() {
                            if let Ok(sv) = serde_json::from_value::<SlotValue>(v.clone()) {
                                out.insert(k.clone(), sv);
                            }
                        }
                    }
                }
                out.insert(
                    "events".to_string(),
                    SlotValue::Json(serde_json::Value::Array(events)),
                );
                return Ok(out);
            }
            Some(StreamNotification::Error(e)) => {
                return Err(NodeError::Backend(format!(
                    "stream.error from extension: {:?}",
                    e.error
                )))
            }
            Some(StreamNotification::Cancel(_)) => return Err(NodeError::Cancelled),
            None => {
                return Err(NodeError::Backend(
                    "stream subscription channel closed before stream.end".into(),
                ));
            }
        }
    }
}

/// Map the host-visible string body of a flow-node carve-out error
/// onto the engine's typed [`NodeError`] enum.
fn map_carved_error(payload: &str) -> Option<NodeError> {
    #[derive(Deserialize)]
    struct WireErr {
        code: i32,
        #[serde(default)]
        message: String,
    }
    let wire: WireErr = serde_json::from_str(payload).ok()?;
    if !flow_node_error_codes::is_in_range(wire.code) {
        return None;
    }
    match wire.code {
        flow_node_error_codes::NODE_CANCELLED => Some(NodeError::Cancelled),
        flow_node_error_codes::NODE_BACKEND => Some(NodeError::Backend(wire.message)),
        flow_node_error_codes::INVALID_INVOCATION_PARAMS => {
            Some(NodeError::InvalidInput(wire.message))
        }
        flow_node_error_codes::NODE_KIND_NOT_BOUND => Some(NodeError::Domain {
            code: "node_kind_not_bound",
            message: wire.message,
        }),
        _ => None,
    }
}

/// Tiny scope-guard helper — we deliberately do not pull the
/// `scopeguard` crate for one call site. Drops invoke the closure
/// exactly once.
fn scopeguard_for<F: FnOnce()>(f: F) -> impl Drop {
    struct Guard<F: FnOnce()>(Option<F>);
    impl<F: FnOnce()> Drop for Guard<F> {
        fn drop(&mut self) {
            if let Some(f) = self.0.take() {
                f();
            }
        }
    }
    Guard(Some(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invoke_params_round_trip() {
        let p = InvokeParams {
            kind: "com.nube.mqtt.publish".into(),
            invocation_id: "inv-XYZ".into(),
            deadline_ms: 30_000,
            input: SlotMap::new(),
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: InvokeParams = serde_json::from_str(&s).unwrap();
        assert_eq!(back.kind, p.kind);
        assert_eq!(back.invocation_id, p.invocation_id);
        assert_eq!(back.deadline_ms, p.deadline_ms);
    }

    #[test]
    fn carved_error_decodes_cancelled() {
        let body = serde_json::to_string(&serde_json::json!({
            "code": flow_node_error_codes::NODE_CANCELLED,
            "message": "child observed stream.cancel",
        }))
        .unwrap();
        let err = map_carved_error(&body).expect("decode");
        assert!(matches!(err, NodeError::Cancelled));
    }

    #[test]
    fn carved_error_decodes_backend() {
        let body = serde_json::to_string(&serde_json::json!({
            "code": flow_node_error_codes::NODE_BACKEND,
            "message": "broker refused",
        }))
        .unwrap();
        let err = map_carved_error(&body).expect("decode");
        match err {
            NodeError::Backend(m) => assert_eq!(m, "broker refused"),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn carved_error_ignores_outside_range() {
        let body = serde_json::to_string(&serde_json::json!({
            "code": -32602,
            "message": "invalid params",
        }))
        .unwrap();
        assert!(map_carved_error(&body).is_none());
    }

    #[test]
    fn parse_response_accepts_wrapped_and_bare() {
        let bare = serde_json::json!({});
        assert!(parse_response_as_slotmap(bare).is_ok());

        let wrapped = serde_json::json!({"slots": {"published_at": {"type":"int","value": 123}}});
        let m = parse_response_as_slotmap(wrapped).unwrap();
        assert!(m.contains_key("published_at"));
    }
}
