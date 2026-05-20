//! End-to-end Phase 8 smoke test.
//!
//! Stages a synthetic bundle with `contributes.grpc` entries, runs
//! the kernel loader, wires the registered handlers through the
//! `starter-ext-grpc` adapter, spins the tonic backplane service on
//! a loopback port, and exercises `ListMethods`, `Invoke` (unary)
//! and `InvokeStream` (server-streaming) end-to-end.

use std::fs;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use starter_ext_grpc::proto::extension_grpc_client::ExtensionGrpcClient;
use starter_ext_grpc::proto::{InvokeRequest, ListMethodsRequest};
use starter_ext_grpc::{
    build_grpc_methods, extension_grpc_server, BuiltinGrpcDispatcher, BuiltinGrpcRegistry,
    DEFAULT_REQUEST_TIMEOUT,
};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_sdk::ctx::CtxInner;
use starter_ext_spi::{ExtensionId, Result as ExtResult};
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;

const BLOCK_YAML: &str = r#"v: 1
id: com.acme.weather
version: 0.1.0
display_name: "Weather"
description_file: docs/README.md
authors: ["ap@nube-io.com"]

runtime:
  kind: builtin
  crate_name: weather

contributes:
  grpc:
    - id: com.acme.weather.current
      service: weather.v1.Weather
      method: Current
      proto: proto/weather.proto
      description_file: docs/current.md
    - id: com.acme.weather.live
      service: weather.v1.Weather
      method: Live
      proto: proto/weather.proto
      description_file: docs/live.md
      streaming: true
"#;

fn stage_bundle() -> TempDir {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("com.acme.weather");
    fs::create_dir_all(root.join("proto")).unwrap();
    fs::create_dir_all(root.join("docs")).unwrap();
    fs::write(root.join("block.yaml"), BLOCK_YAML).unwrap();
    fs::write(
        root.join("proto/weather.proto"),
        "syntax = \"proto3\"; package weather.v1;\n",
    )
    .unwrap();
    fs::write(root.join("docs/README.md"), "Weather extension\n").unwrap();
    fs::write(
        root.join("docs/current.md"),
        "Get the current temperature.\n",
    )
    .unwrap();
    fs::write(
        root.join("docs/live.md"),
        "Stream live temperature updates.\n",
    )
    .unwrap();
    tmp
}

fn loaded_registry(tmp: &Path) -> Arc<ExtensionRegistry> {
    let candidates = Loader::scan(tmp).validate_all();
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(candidates, &mut registry);
    registry.seal();
    assert_eq!(outcome.validated, 1, "bundle must validate");
    Arc::new(registry)
}

fn current_handler(args: serde_json::Value, _ctx: &CtxInner) -> ExtResult<serde_json::Value> {
    let city = args
        .get("city")
        .and_then(|v| v.as_str())
        .unwrap_or("nowhere");
    Ok(serde_json::json!({ "city": city, "temp_c": 21.4 }))
}

fn live_handler(args: serde_json::Value, ctx: &CtxInner) -> ExtResult<()> {
    let n = args.get("n").and_then(|v| v.as_i64()).unwrap_or(3);
    let sender = ctx.events().clone();
    for i in 0..n {
        if ctx.cancel().is_cancelled() {
            break;
        }
        let ev = starter_ext_sdk::ctx::Event {
            stream_id: starter_ext_sdk::StreamId("test".into()),
            payload: serde_json::json!({ "i": i, "temp_c": 21.0 + (i as f64) * 0.1 }),
        };
        if sender.blocking_send(ev).is_err() {
            break;
        }
    }
    Ok(())
}

async fn spawn_server(registry: &ExtensionRegistry) -> SocketAddr {
    let methods = build_grpc_methods(registry).unwrap();
    assert_eq!(methods.len(), 2);

    let ext = ExtensionId::new("com.acme.weather").unwrap();
    let handlers = BuiltinGrpcRegistry::new()
        .register(ext.clone(), "com.acme.weather.current", current_handler)
        .register_streaming(ext, "com.acme.weather.live", live_handler);
    let dispatcher = Arc::new(BuiltinGrpcDispatcher::new(Arc::new(handlers)));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let stream = TcpListenerStream::new(listener);
    let server = extension_grpc_server(methods, dispatcher, DEFAULT_REQUEST_TIMEOUT);

    tokio::spawn(async move {
        let _ = tonic::transport::Server::builder()
            .add_service(server)
            .serve_with_incoming(stream)
            .await;
    });

    addr
}

#[tokio::test]
async fn list_methods_returns_each_contribute_grpc_entry() {
    let tmp = stage_bundle();
    let registry = loaded_registry(tmp.path());
    let addr = spawn_server(&registry).await;

    let mut client = ExtensionGrpcClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let resp = client.list_methods(ListMethodsRequest {}).await.unwrap();
    let mut methods = resp.into_inner().methods;
    methods.sort_by(|a, b| a.method.cmp(&b.method));
    assert_eq!(methods.len(), 2);
    assert_eq!(methods[0].method, "Current");
    assert_eq!(methods[0].service, "weather.v1.Weather");
    assert!(!methods[0].streaming);
    assert_eq!(
        methods[0].description.trim(),
        "Get the current temperature."
    );
    assert_eq!(methods[0].proto_path, "proto/weather.proto");
    assert_eq!(methods[1].method, "Live");
    assert!(methods[1].streaming);
}

#[tokio::test]
async fn invoke_unary_round_trips_handler() {
    let tmp = stage_bundle();
    let registry = loaded_registry(tmp.path());
    let addr = spawn_server(&registry).await;

    let mut client = ExtensionGrpcClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let resp = client
        .invoke(InvokeRequest {
            service: "weather.v1.Weather".into(),
            method: "Current".into(),
            args_proto_json: r#"{"city":"sydney"}"#.into(),
        })
        .await
        .unwrap();
    let parsed: serde_json::Value =
        serde_json::from_str(&resp.into_inner().result_proto_json).unwrap();
    assert_eq!(parsed["city"], "sydney");
    assert_eq!(parsed["temp_c"], 21.4);
}

#[tokio::test]
async fn invoke_unknown_pair_returns_not_found() {
    let tmp = stage_bundle();
    let registry = loaded_registry(tmp.path());
    let addr = spawn_server(&registry).await;

    let mut client = ExtensionGrpcClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let err = client
        .invoke(InvokeRequest {
            service: "nope.v1.Service".into(),
            method: "Nope".into(),
            args_proto_json: "{}".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::NotFound);
}

#[tokio::test]
async fn invoke_against_streaming_method_returns_failed_precondition() {
    let tmp = stage_bundle();
    let registry = loaded_registry(tmp.path());
    let addr = spawn_server(&registry).await;

    let mut client = ExtensionGrpcClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let err = client
        .invoke(InvokeRequest {
            service: "weather.v1.Weather".into(),
            method: "Live".into(),
            args_proto_json: "{}".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);
}

#[tokio::test]
async fn invoke_stream_emits_one_event_per_handler_tick() {
    let tmp = stage_bundle();
    let registry = loaded_registry(tmp.path());
    let addr = spawn_server(&registry).await;

    let mut client = ExtensionGrpcClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let resp = client
        .invoke_stream(InvokeRequest {
            service: "weather.v1.Weather".into(),
            method: "Live".into(),
            args_proto_json: r#"{"n":4}"#.into(),
        })
        .await
        .unwrap();
    let mut stream = resp.into_inner();
    let mut frames = Vec::new();
    while let Some(item) = tokio::time::timeout(Duration::from_secs(2), stream.next())
        .await
        .unwrap()
    {
        frames.push(item.unwrap());
    }
    assert_eq!(frames.len(), 4);
    let first: serde_json::Value = serde_json::from_str(&frames[0].payload_proto_json).unwrap();
    assert_eq!(first["payload"]["i"], 0);
    let last: serde_json::Value = serde_json::from_str(&frames[3].payload_proto_json).unwrap();
    assert_eq!(last["payload"]["i"], 3);
}

#[tokio::test]
async fn invoke_stream_against_unary_method_returns_failed_precondition() {
    let tmp = stage_bundle();
    let registry = loaded_registry(tmp.path());
    let addr = spawn_server(&registry).await;

    let mut client = ExtensionGrpcClient::connect(format!("http://{addr}"))
        .await
        .unwrap();
    let err = client
        .invoke_stream(InvokeRequest {
            service: "weather.v1.Weather".into(),
            method: "Current".into(),
            args_proto_json: "{}".into(),
        })
        .await
        .unwrap_err();
    assert_eq!(err.code(), tonic::Code::FailedPrecondition);
}
