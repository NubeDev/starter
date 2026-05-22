//! `mqtt-driver` — the child binary for the
//! `examples/flow-agent/extensions/com.nube.mqtt/` bundle (slice B
//! of `DOCS/extensions/scope/FLOW-NODES.md`).
//!
//! One process per supervised extension; one persistent MQTT
//! connection per process; nodes (`com.nube.mqtt.publish`,
//! `com.nube.mqtt.subscribe`) are stateless behaviours dispatched
//! on top (R-flow-node-8 — extensions are stateful processes,
//! nodes are stateless behaviours on top).
//!
//! Wire surface (one tokio::main dispatcher):
//!   - `init`              → InitReady { manifest_hash, ready }
//!   - `health`            → `null`
//!   - `flow.node.invoke`  → publish: returns SlotMap with
//!                           `published_at` (millis). subscribe:
//!                           returns open-stream ack, then emits
//!                           `stream.event` per message until
//!                           `stream.cancel` lands.
//!   - `stream.cancel`     → cancel an in-flight subscribe by
//!                           invocation_id.
//!   - `shutdown`          → exit cleanly.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context};
use rumqttc::{AsyncClient, ClientError, Event, MqttOptions, Packet, QoS};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use starter_jsonrpc_stdio::{read_frame, write_frame};
use tokio::io::{stdin, stdout, BufReader};
use tokio::sync::{mpsc, Mutex};

const FLOW_NODE_INVOKE: &str = "flow.node.invoke";
const STREAM_EVENT: &str = "stream.event";
const STREAM_END: &str = "stream.end";
const STREAM_ERROR: &str = "stream.error";
const STREAM_CANCEL: &str = "stream.cancel";

const KIND_PUBLISH: &str = "com.nube.mqtt.publish";
const KIND_SUBSCRIBE: &str = "com.nube.mqtt.subscribe";

#[derive(Debug, Clone, Deserialize)]
struct Config {
    broker_url: String,
    #[serde(default)]
    client_id_prefix: Option<String>,
}

#[derive(Debug, Deserialize)]
struct InvokeParams {
    kind: String,
    invocation_id: String,
    #[allow(dead_code)]
    deadline_ms: u64,
    #[serde(default)]
    input: serde_json::Map<String, Value>,
    #[serde(default)]
    settings: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct PublishSettings {
    topic: String,
    #[serde(default)]
    qos: u8,
    #[serde(default)]
    retain: bool,
}

#[derive(Debug, Deserialize)]
struct SubscribeSettings {
    topic: String,
    #[serde(default)]
    qos: u8,
}

fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn qos_from(q: u8) -> QoS {
    match q {
        1 => QoS::AtLeastOnce,
        2 => QoS::ExactlyOnce,
        _ => QoS::AtMostOnce,
    }
}

/// Per-invocation handle for a streaming subscribe. The dispatcher
/// drops the sender when `stream.cancel` arrives or the subscribe
/// task ends; the subscribe task watches the receiver and bails
/// out cleanly.
type CancelMap = Arc<Mutex<HashMap<String, mpsc::Sender<()>>>>;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::process::ExitCode {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .try_init();

    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("mqtt-driver: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let mut stdin = BufReader::new(stdin());
    let mut stdout = stdout();

    // ---- init handshake -----------------------------------------
    let init_frame = read_frame(&mut stdin)
        .await
        .context("read init frame")?
        .ok_or_else(|| anyhow!("stdin closed before init"))?;
    let init: Value =
        serde_json::from_slice(&init_frame).context("parse init JSON-RPC envelope")?;
    let init_id = init.get("id").cloned().unwrap_or(Value::Null);
    let manifest_hash = init
        .pointer("/params/manifest_hash")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();
    let config_value = init
        .pointer("/params/config")
        .cloned()
        .unwrap_or(Value::Null);
    let config: Config =
        serde_json::from_value(config_value.clone()).context("invalid extension config")?;

    let response = json!({
        "jsonrpc": "2.0",
        "id": init_id,
        "result": {
            "manifest_hash": manifest_hash,
            "ready": true,
            "version": env!("CARGO_PKG_VERSION"),
        }
    });
    write_frame(&mut stdout, &serde_json::to_vec(&response)?).await?;

    // ---- spin up MQTT client ------------------------------------
    let (client, eventloop) = build_mqtt_client(&config)?;
    let bus = MqttBus::start(client.clone(), eventloop);
    let cancel_map: CancelMap = Arc::new(Mutex::new(HashMap::new()));

    // ---- dispatcher loop ----------------------------------------
    loop {
        let frame = match read_frame(&mut stdin).await {
            Ok(Some(f)) => f,
            Ok(None) => return Ok(()), // host closed stdin → clean exit
            Err(e) => return Err(anyhow!("frame read: {e}")),
        };
        let value: Value = serde_json::from_slice(&frame).context("inbound frame not JSON")?;
        let method = value.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = value.get("id").cloned();

        match method {
            "health" => {
                if let Some(id) = id {
                    let resp = json!({"jsonrpc":"2.0","id":id,"result":null});
                    write_frame(&mut stdout, &serde_json::to_vec(&resp)?).await?;
                }
            }
            "shutdown" => {
                return Ok(());
            }
            STREAM_CANCEL => {
                let stream_id = value
                    .pointer("/params/stream_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_owned();
                if !stream_id.is_empty() {
                    let mut g = cancel_map.lock().await;
                    if let Some(tx) = g.remove(&stream_id) {
                        let _ = tx.send(()).await;
                    }
                }
            }
            FLOW_NODE_INVOKE => {
                let id_value = id.clone().unwrap_or(Value::Null);
                let params: InvokeParams = match value
                    .get("params")
                    .cloned()
                    .map(serde_json::from_value::<InvokeParams>)
                    .transpose()
                {
                    Ok(Some(p)) => p,
                    Ok(None) | Err(_) => {
                        send_error(
                            &mut stdout,
                            id_value,
                            -32051,
                            "missing or malformed flow.node.invoke params",
                        )
                        .await?;
                        continue;
                    }
                };

                match params.kind.as_str() {
                    KIND_PUBLISH => {
                        let settings =
                            match params.settings.as_ref().and_then(|s| {
                                serde_json::from_value::<PublishSettings>(s.clone()).ok()
                            }) {
                                Some(s) => s,
                                None => {
                                    send_error(
                                        &mut stdout,
                                        id_value,
                                        -32051,
                                        "invalid publish settings",
                                    )
                                    .await?;
                                    continue;
                                }
                            };
                        // Extract payload bytes from the `payload` input
                        // slot (string or bytes).
                        let payload_bytes = match params.input.get("payload") {
                            Some(slot) => slot_value_to_bytes(slot),
                            None => Vec::new(),
                        };
                        let publish_res = client
                            .publish(
                                &settings.topic,
                                qos_from(settings.qos),
                                settings.retain,
                                payload_bytes,
                            )
                            .await;
                        match publish_res {
                            Ok(()) => {
                                let resp = json!({
                                    "jsonrpc": "2.0",
                                    "id": id_value,
                                    "result": {
                                        "slots": {
                                            "published_at": {"type":"int","value": now_millis()},
                                        }
                                    }
                                });
                                write_frame(&mut stdout, &serde_json::to_vec(&resp)?).await?;
                            }
                            Err(e) => {
                                send_error(
                                    &mut stdout,
                                    id_value,
                                    -32060,
                                    &format!("broker publish: {e}"),
                                )
                                .await?;
                            }
                        }
                    }
                    KIND_SUBSCRIBE => {
                        let settings = match params.settings.as_ref().and_then(|s| {
                            serde_json::from_value::<SubscribeSettings>(s.clone()).ok()
                        }) {
                            Some(s) => s,
                            None => {
                                send_error(
                                    &mut stdout,
                                    id_value,
                                    -32051,
                                    "invalid subscribe settings",
                                )
                                .await?;
                                continue;
                            }
                        };
                        // Open subscription + register cancel sender.
                        let (cancel_tx, mut cancel_rx) = mpsc::channel::<()>(1);
                        cancel_map
                            .lock()
                            .await
                            .insert(params.invocation_id.clone(), cancel_tx);
                        let topic = settings.topic.clone();
                        let qos = qos_from(settings.qos);
                        if let Err(e) = client.subscribe(&topic, qos).await {
                            send_error(
                                &mut stdout,
                                id_value,
                                -32060,
                                &format!("subscribe failed: {e}"),
                            )
                            .await?;
                            cancel_map.lock().await.remove(&params.invocation_id);
                            continue;
                        }
                        // Initial response: open-stream ack.
                        let ack = json!({
                            "jsonrpc": "2.0",
                            "id": id_value,
                            "result": {
                                "stream_id": params.invocation_id,
                                "slots": {}
                            }
                        });
                        write_frame(&mut stdout, &serde_json::to_vec(&ack)?).await?;

                        // Drain bus events for this topic until cancel.
                        let mut bus_rx = bus.subscribe().await;
                        let invocation_id = params.invocation_id.clone();
                        let cancel_map_clone = cancel_map.clone();
                        let topic_filter = topic.clone();
                        // Spawn a per-invocation task that pushes
                        // stream.event frames onto stdout via the
                        // shared writer mutex. We hold the writer
                        // mutex on every frame so concurrent
                        // invocations interleave cleanly.
                        let writer = stdout_writer_handle();
                        tokio::spawn(async move {
                            loop {
                                tokio::select! {
                                    _ = cancel_rx.recv() => {
                                        let env = json!({
                                            "jsonrpc": "2.0",
                                            "method": STREAM_END,
                                            "params": {"stream_id": invocation_id, "payload": {"reason":"cancelled"}}
                                        });
                                        let _ = writer.write(env).await;
                                        cancel_map_clone.lock().await.remove(&invocation_id);
                                        return;
                                    }
                                    msg = bus_rx.recv() => {
                                        let msg = match msg {
                                            Ok(m) => m,
                                            Err(_) => {
                                            let env = json!({
                                                "jsonrpc": "2.0",
                                                "method": STREAM_ERROR,
                                                "params": {
                                                    "stream_id": invocation_id,
                                                    "error": {"kind":"transport","message":"mqtt bus closed"}
                                                }
                                            });
                                            let _ = writer.write(env).await;
                                            cancel_map_clone.lock().await.remove(&invocation_id);
                                            return;
                                            }
                                        };
                                        if !topic_matches(&topic_filter, &msg.topic) {
                                            continue;
                                        }
                                        let payload_str =
                                            String::from_utf8(msg.payload.clone())
                                                .unwrap_or_else(|_| String::new());
                                        let env = json!({
                                            "jsonrpc": "2.0",
                                            "method": STREAM_EVENT,
                                            "params": {
                                                "stream_id": invocation_id,
                                                "payload": {
                                                    "topic": msg.topic,
                                                    "payload": payload_str,
                                                    "qos": msg.qos,
                                                    "retain": msg.retain,
                                                }
                                            }
                                        });
                                        let _ = writer.write(env).await;
                                    }
                                }
                            }
                        });
                    }
                    other => {
                        send_error(
                            &mut stdout,
                            id_value,
                            -32050,
                            &format!("unknown kind {other:?}"),
                        )
                        .await?;
                    }
                }
            }
            // Any other inbound request is host→child capability
            // dispatch we don't implement; echo a clean "not
            // implemented" so the host doesn't hang.
            other if !other.is_empty() => {
                if let Some(id) = id {
                    let env = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": {"kind":"capability","message":format!("unhandled method {other:?}")}
                    });
                    write_frame(&mut stdout, &serde_json::to_vec(&env)?).await?;
                }
            }
            _ => {}
        }
    }
}

async fn send_error<W>(writer: &mut W, id: Value, code: i32, message: &str) -> anyhow::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let env = json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "kind": "extension_internal",
            "message": serde_json::to_string(&json!({"code": code, "message": message}))
                .unwrap_or_else(|_| message.to_owned()),
        }
    });
    write_frame(writer, &serde_json::to_vec(&env)?).await?;
    Ok(())
}

fn slot_value_to_bytes(slot: &Value) -> Vec<u8> {
    // Accept the engine's tagged slot shapes plus bare strings.
    if let Some(s) = slot.as_str() {
        return s.as_bytes().to_vec();
    }
    if let Some(obj) = slot.as_object() {
        match obj.get("type").and_then(|v| v.as_str()) {
            Some("string") => {
                if let Some(s) = obj.get("value").and_then(|v| v.as_str()) {
                    return s.as_bytes().to_vec();
                }
            }
            Some("bytes") => {
                if let Some(arr) = obj.get("value").and_then(|v| v.as_array()) {
                    return arr
                        .iter()
                        .filter_map(|v| v.as_u64().map(|n| n as u8))
                        .collect();
                }
            }
            _ => {}
        }
    }
    // Fall back: serialise verbatim so a JSON payload reaches the
    // broker as its JSON text.
    serde_json::to_vec(slot).unwrap_or_default()
}

fn build_mqtt_client(cfg: &Config) -> anyhow::Result<(AsyncClient, rumqttc::EventLoop)> {
    let (host, port) = parse_broker_url(&cfg.broker_url)
        .ok_or_else(|| anyhow!("broker_url must be tcp://host:port or mqtts://host:port"))?;
    let mut id = cfg
        .client_id_prefix
        .clone()
        .unwrap_or_else(|| "flow-agent".to_owned());
    id.push_str(&format!("-{}", now_millis()));
    let mut opts = MqttOptions::new(id, host, port);
    opts.set_keep_alive(Duration::from_secs(15));
    Ok(AsyncClient::new(opts, 32))
}

fn parse_broker_url(url: &str) -> Option<(String, u16)> {
    let (scheme, rest) = url.split_once("://")?;
    let (host, port_str) = rest.rsplit_once(':')?;
    let port: u16 = port_str.parse().ok()?;
    let _ = scheme;
    Some((host.to_owned(), port))
}

// ----- MQTT bus: persistent connection shared across invocations.

#[derive(Clone, Debug)]
struct InboundMessage {
    topic: String,
    payload: Vec<u8>,
    qos: u8,
    retain: bool,
}

struct MqttBus {
    tx: tokio::sync::broadcast::Sender<InboundMessage>,
}

impl MqttBus {
    fn start(_client: AsyncClient, mut eventloop: rumqttc::EventLoop) -> Arc<Self> {
        let (tx, _) = tokio::sync::broadcast::channel::<InboundMessage>(256);
        let tx_clone = tx.clone();
        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(p))) => {
                        let _ = tx_clone.send(InboundMessage {
                            topic: p.topic,
                            payload: p.payload.to_vec(),
                            qos: p.qos as u8,
                            retain: p.retain,
                        });
                    }
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!(error = %e, "mqtt eventloop error; reconnecting in 1s");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        });
        Arc::new(Self { tx })
    }
    async fn subscribe(&self) -> tokio::sync::broadcast::Receiver<InboundMessage> {
        self.tx.subscribe()
    }
}

// ----- Shared stdout writer ---------------------------------------
//
// `tokio::io::stdout()` is `!Sync` so concurrent spawned tasks need
// a mutex-guarded handle. We expose one writer-channel where each
// writer pushes a complete JSON value; a dedicated drain task
// serialises and frames them.

#[derive(Clone)]
struct StdoutHandle {
    tx: mpsc::Sender<Value>,
}

impl StdoutHandle {
    async fn write(&self, env: Value) -> Result<(), mpsc::error::SendError<Value>> {
        self.tx.send(env).await
    }
}

fn stdout_writer_handle() -> StdoutHandle {
    static INIT: std::sync::OnceLock<StdoutHandle> = std::sync::OnceLock::new();
    INIT.get_or_init(|| {
        let (tx, mut rx) = mpsc::channel::<Value>(64);
        tokio::spawn(async move {
            let mut out = stdout();
            while let Some(env) = rx.recv().await {
                if let Ok(bytes) = serde_json::to_vec(&env) {
                    let _ = write_frame(&mut out, &bytes).await;
                }
            }
        });
        StdoutHandle { tx }
    })
    .clone()
}

/// Minimal MQTT topic-filter matcher (supports `+` single-level
/// and `#` multi-level wildcards). Good enough for the demo; a
/// production driver would use rumqttc's own matcher.
fn topic_matches(filter: &str, topic: &str) -> bool {
    let f: Vec<&str> = filter.split('/').collect();
    let t: Vec<&str> = topic.split('/').collect();
    let mut i = 0;
    while i < f.len() {
        match f[i] {
            "#" => return true,
            "+" => {
                if i >= t.len() {
                    return false;
                }
            }
            seg => {
                if i >= t.len() || t[i] != seg {
                    return false;
                }
            }
        }
        i += 1;
    }
    i == t.len()
}

// Silence unused-clippy on Serialize import; kept for parity with
// the host's manifest schema (we may want to derive structs that
// serialise on the wire later).
#[allow(dead_code)]
fn _serialize_marker<T: Serialize>(_t: &T) {}

// Silence rumqttc's ClientError unused-import lint when MQTT calls
// short-circuit early.
#[allow(dead_code)]
fn _client_error_marker(_e: ClientError) {}
