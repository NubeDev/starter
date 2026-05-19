//! Socket-Mode transport: open the WSS URL via `apps.connections.open`,
//! pump frames, ack envelopes, and emit each typed event into the
//! [`EventSink`](starter_spi::service::EventSink) as
//! `kind = "slack.<event_type>"`.
//!
//! Lifted from `codeless-slack::socket_mode` — the protocol handling,
//! envelope shape, and ack discipline are the same; the dispatcher
//! seam is replaced with `EventSink::emit` so SCOPE R4 is honoured
//! (the crate does not pattern-match on payloads beyond deserialization).

use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use starter_spi::service::EventSink;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::Message;

use crate::error::SlackSocketModeError;
use crate::metrics::ServiceMetrics;

/// Endpoint that mints a single-use `wss_url`. Tests override via
/// [`SlackSocketModeConfig::base_url`](crate::SlackSocketModeConfig).
pub(crate) fn open_connection_url(base_url: &str) -> String {
    format!("{}/apps.connections.open", base_url.trim_end_matches('/'))
}

/// Wire shape of the `apps.connections.open` response. Slack returns
/// many other fields; only `ok`, `url`, and `error` are load-bearing.
#[derive(Debug, Deserialize)]
struct OpenConnectionResponse {
    ok: bool,
    url: Option<String>,
    error: Option<String>,
}

/// Envelope sent back to Slack to ack an inbound message. Slack
/// distinguishes acks from other client-to-server frames purely by the
/// presence of `envelope_id`; the field name has to match exactly.
#[derive(Debug, Serialize)]
struct Ack<'a> {
    envelope_id: &'a str,
}

/// Slack `events_api` envelope, projected onto only the fields this
/// service cares about (ack id + the event-type label that drives the
/// emit `kind`). The full payload is captured separately as raw JSON
/// and forwarded verbatim into the sink — R4 forbids the service from
/// pattern-matching past this point.
#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "type")]
    kind: Option<String>,
    envelope_id: Option<String>,
    #[serde(default)]
    payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct EventsApiPayload {
    event: Option<EventBody>,
}

#[derive(Debug, Deserialize)]
struct EventBody {
    #[serde(rename = "type")]
    kind: String,
}

/// Outcome of one connect+pump round. The outer loop in
/// [`crate::service`] decides whether to backoff (failure) or reset the
/// retry counter (clean disconnect).
#[derive(Debug)]
pub(crate) enum ConnectOutcome {
    /// `ctx.shutdown` flipped while we were pumping; exit cleanly.
    Shutdown,
    /// Slack closed the socket gracefully (server-initiated rotate /
    /// disconnect frame / stream end). Caller should reconnect after a
    /// backoff but **not** count it toward the circuit.
    Disconnected,
}

/// POST `apps.connections.open` and return the WSS URL.
pub(crate) async fn open_connection(
    http: &reqwest::Client,
    endpoint: &str,
    app_token: &str,
) -> Result<String, SlackSocketModeError> {
    let resp = http
        .post(endpoint)
        .bearer_auth(app_token)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        return Err(SlackSocketModeError::HttpStatus {
            status: status.as_u16(),
        });
    }
    let payload: OpenConnectionResponse = resp.json().await?;
    if !payload.ok {
        return Err(SlackSocketModeError::SlackApi(payload.error));
    }
    let url = payload
        .url
        .ok_or_else(|| SlackSocketModeError::BadWssUrl("response missing `url` field".into()))?;
    // Validate up front so a malformed response surfaces a clear error
    // rather than a generic dial failure deeper in tungstenite.
    url::Url::parse(&url).map_err(|e| SlackSocketModeError::BadWssUrl(e.to_string()))?;
    Ok(url)
}

/// Dial the WSS URL and pump frames until the socket closes or
/// `ctx.shutdown` flips.
///
/// Each text frame is decoded, acked (so Slack stops retrying every 3s
/// per the protocol contract), and — if it carries an `events_api`
/// event — emitted into `sink` as `slack.<event_type>`.
pub(crate) async fn pump_until_closed(
    wss_url: &str,
    shutdown: &mut watch::Receiver<bool>,
    sink: &Arc<dyn EventSink>,
    metrics: &ServiceMetrics,
    service_name: &'static str,
) -> Result<ConnectOutcome, SlackSocketModeError> {
    let (mut socket, _resp) = tokio_tungstenite::connect_async(wss_url).await?;
    metrics.running.set(1);
    let result = pump_loop(&mut socket, shutdown, sink, metrics, service_name).await;
    metrics.running.set(0);
    result
}

async fn pump_loop<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
    shutdown: &mut watch::Receiver<bool>,
    sink: &Arc<dyn EventSink>,
    metrics: &ServiceMetrics,
    service_name: &'static str,
) -> Result<ConnectOutcome, SlackSocketModeError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        tokio::select! {
            biased;
            res = shutdown.changed() => {
                // `changed()` returns Err when the sender drops; either
                // way the right move is to close gracefully and exit.
                if res.is_err() || *shutdown.borrow() {
                    let _ = socket.close(None).await;
                    return Ok(ConnectOutcome::Shutdown);
                }
            }
            frame = socket.next() => match frame {
                Some(Ok(Message::Text(text))) => {
                    handle_text_frame(socket, &text, sink, metrics, service_name).await?;
                }
                Some(Ok(Message::Ping(payload))) => {
                    if let Err(e) = socket.send(Message::Pong(payload)).await {
                        return Err(SlackSocketModeError::WebSocket(e));
                    }
                }
                Some(Ok(Message::Close(frame))) => {
                    tracing::info!(
                        service.name = service_name,
                        ?frame,
                        "slack: server closed socket",
                    );
                    return Ok(ConnectOutcome::Disconnected);
                }
                Some(Ok(_)) => {
                    // Binary / Pong / continuation frames are not used
                    // by Socket Mode; ignore.
                }
                Some(Err(e)) => return Err(SlackSocketModeError::WebSocket(e)),
                None => return Ok(ConnectOutcome::Disconnected),
            }
        }
    }
}

async fn handle_text_frame<S>(
    socket: &mut tokio_tungstenite::WebSocketStream<S>,
    text: &str,
    sink: &Arc<dyn EventSink>,
    metrics: &ServiceMetrics,
    service_name: &'static str,
) -> Result<(), SlackSocketModeError>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let envelope: Envelope = match serde_json::from_str(text) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                service.name = service_name,
                error = %e,
                raw = %text,
                "slack: failed to decode envelope",
            );
            return Ok(());
        }
    };

    // Ack first, emit second — Slack stops retrying immediately and we
    // don't risk a slow downstream causing a 3-second retry storm.
    if let Some(id) = envelope.envelope_id.as_deref() {
        let ack = serde_json::to_string(&Ack { envelope_id: id }).expect("Ack serialises");
        if let Err(e) = socket.send(Message::Text(ack)).await {
            tracing::warn!(
                service.name = service_name,
                error = %e,
                "slack: failed to ack envelope",
            );
            return Err(SlackSocketModeError::WebSocket(e));
        }
    }

    // Only `events_api` envelopes carry a typed event for the sink.
    // `hello`, `disconnect`, etc. are protocol-level and not forwarded.
    if envelope.kind.as_deref() != Some("events_api") {
        return Ok(());
    }
    let payload = envelope.payload;
    // Re-parse just the inner `event.type` so we know what `kind`
    // string to label the emit with. R4: this is the *only* pattern
    // match the service does — domain interpretation lives in the
    // consumer.
    let event_kind = match serde_json::from_value::<EventsApiPayload>(payload.clone()) {
        Ok(p) => p.event.map(|e| e.kind),
        Err(_) => None,
    };
    let kind = match event_kind {
        Some(k) => format!("slack.{k}"),
        None => "slack.unknown".to_string(),
    };

    match sink.emit(&kind, payload).await {
        Ok(()) => {
            metrics.events.with_label_values(&[&kind]).inc();
        }
        Err(e) => {
            // SCOPE D4: log-and-continue on individual sink errors;
            // back-pressure is the sink-fan-out helper's job at the
            // consumer side. We don't kill the pump on a sink failure.
            tracing::warn!(
                service.name = service_name,
                error = %e,
                kind = %kind,
                "slack: event sink emit failed",
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_connection_url_handles_trailing_slash() {
        assert_eq!(
            open_connection_url("https://slack.com/api"),
            "https://slack.com/api/apps.connections.open",
        );
        assert_eq!(
            open_connection_url("https://slack.com/api/"),
            "https://slack.com/api/apps.connections.open",
        );
    }

    #[test]
    fn envelope_decodes_events_api_kind() {
        let raw = r#"{
            "type":"events_api",
            "envelope_id":"abc",
            "payload":{"event":{"type":"app_mention","channel":"C1"}}
        }"#;
        let env: Envelope = serde_json::from_str(raw).unwrap();
        assert_eq!(env.kind.as_deref(), Some("events_api"));
        assert_eq!(env.envelope_id.as_deref(), Some("abc"));
        let p: EventsApiPayload = serde_json::from_value(env.payload).unwrap();
        assert_eq!(p.event.unwrap().kind, "app_mention");
    }
}
