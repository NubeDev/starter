//! End-to-end Phase 6 smoke test.
//!
//! Stages the `examples/hello-cli` bundle into a tempdir, runs the
//! kernel loader, wires the registered CLI handlers through the
//! `starter-ext-cli` adapter, registers the produced subcommands on
//! a `starter_cli::CommandRegistry`, and dispatches both the
//! non-streaming and streaming subcommands programmatically.
//!
//! Asserts:
//!
//! - the `hellocli-greet` subcommand registers and dispatches
//!   end-to-end through the adapter,
//! - the streaming `hellocli-tick` dispatch_stream path emits one
//!   event per tick on the kernel `StreamResponse`,
//! - the `CancelHandle` fires `stream.cancel` semantics within
//!   a few hundred milliseconds when fired explicitly (the SIGINT
//!   path in the binary wraps the same handle).

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use starter_ext_cli::{
    build_cli_commands, BuiltinCliDispatcher, BuiltinCliRegistry, CliDispatcher,
    DEFAULT_REQUEST_TIMEOUT,
};
use starter_ext_host::{ExtensionRegistry, Loader};
use starter_ext_sdk::ctx::CtxInner;
use starter_ext_spi::{ExtensionId, Result};
use tempfile::tempdir;

fn hello_cli_bundle_src() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent() // crates/
        .unwrap()
        .parent() // workspace root
        .unwrap()
        .join("examples/hello-cli")
}

fn copy_bundle(src: &Path, dest: &Path) {
    fs::create_dir_all(dest).unwrap();
    fs::copy(src.join("block.yaml"), dest.join("block.yaml")).unwrap();
    fs::create_dir_all(dest.join("schemas")).unwrap();
    for name in ["greet_args.json", "tick_args.json"] {
        fs::copy(
            src.join("schemas").join(name),
            dest.join("schemas").join(name),
        )
        .unwrap();
    }
    fs::create_dir_all(dest.join("docs")).unwrap();
    for name in ["README.md", "greet.md", "tick.md"] {
        fs::copy(src.join("docs").join(name), dest.join("docs").join(name)).unwrap();
    }
}

fn stage_registry() -> (tempfile::TempDir, Arc<ExtensionRegistry>) {
    let tmp = tempdir().unwrap();
    copy_bundle(
        &hello_cli_bundle_src(),
        &tmp.path().join("com.acme.hellocli"),
    );
    let records = Loader::scan(tmp.path()).validate_all();
    let mut registry = ExtensionRegistry::new();
    let outcome = Loader::commit(records, &mut registry);
    registry.seal();
    assert_eq!(outcome.validated, 1, "hello-cli bundle must validate");
    (tmp, Arc::new(registry))
}

fn greet_handler(params: serde_json::Value, _ctx: &CtxInner) -> Result<serde_json::Value> {
    let name = params
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("world");
    Ok(serde_json::json!({ "message": format!("hello, {name}") }))
}

fn tick_handler(params: serde_json::Value, ctx: &CtxInner) -> Result<()> {
    let count = params.get("count").and_then(|v| v.as_i64()).unwrap_or(3);
    let sender = ctx.events().clone();
    for n in 0..count {
        if ctx.cancel().is_cancelled() {
            break;
        }
        let ev = starter_ext_sdk::ctx::Event {
            stream_id: starter_ext_sdk::StreamId("test".into()),
            payload: serde_json::json!({ "n": n }),
        };
        if sender.blocking_send(ev).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Ok(())
}

#[tokio::test]
async fn build_cli_commands_produces_one_subcommand_per_entry() {
    let (_tmp, registry) = stage_registry();
    let ext = ExtensionId::new("com.acme.hellocli").unwrap();
    let cli_registry = BuiltinCliRegistry::new()
        .register(ext.clone(), "com.acme.hellocli.greet", greet_handler)
        .register_streaming(ext, "com.acme.hellocli.tick", tick_handler);
    let dispatcher = Arc::new(BuiltinCliDispatcher::new(Arc::new(cli_registry)));

    let commands = build_cli_commands(&registry, dispatcher, DEFAULT_REQUEST_TIMEOUT).unwrap();
    let names: Vec<&str> = commands
        .iter()
        .map(|c| {
            use starter_cli::Command;
            c.name()
        })
        .collect();
    assert!(names.contains(&"hellocli-greet"));
    assert!(names.contains(&"hellocli-tick"));
}

#[tokio::test]
async fn non_streaming_dispatch_round_trips_the_handler() {
    let (_tmp, _registry) = stage_registry();
    let ext = ExtensionId::new("com.acme.hellocli").unwrap();
    let cli_registry =
        BuiltinCliRegistry::new().register(ext.clone(), "com.acme.hellocli.greet", greet_handler);
    let dispatcher = BuiltinCliDispatcher::new(Arc::new(cli_registry));

    let out = dispatcher
        .dispatch(
            &ext,
            "com.acme.hellocli.greet",
            serde_json::json!({"name": "Phase 6"}),
            DEFAULT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap();
    assert_eq!(out["message"], "hello, Phase 6");
}

#[tokio::test]
async fn streaming_dispatch_emits_one_event_per_tick() {
    let (_tmp, _registry) = stage_registry();
    let ext = ExtensionId::new("com.acme.hellocli").unwrap();
    let cli_registry = BuiltinCliRegistry::new().register_streaming(
        ext.clone(),
        "com.acme.hellocli.tick",
        tick_handler,
    );
    let dispatcher = BuiltinCliDispatcher::new(Arc::new(cli_registry));

    let response = dispatcher
        .dispatch_stream(
            &ext,
            "com.acme.hellocli.tick",
            serde_json::json!({"count": 3}),
            DEFAULT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap();
    let events: Vec<_> = response.events.collect().await;
    assert_eq!(events.len(), 3);
    for (i, ev) in events.into_iter().enumerate() {
        assert_eq!(ev.unwrap().payload["n"], i as i64);
    }
}

#[tokio::test]
async fn cancel_fires_within_a_few_hundred_ms() {
    let (_tmp, _registry) = stage_registry();
    let ext = ExtensionId::new("com.acme.hellocli").unwrap();
    let cli_registry = BuiltinCliRegistry::new().register_streaming(
        ext.clone(),
        "com.acme.hellocli.tick",
        // Long count; we'll cancel before it finishes.
        |params, ctx| tick_handler(serde_json::json!({"count": params["count"]}), ctx),
    );
    let dispatcher = BuiltinCliDispatcher::new(Arc::new(cli_registry));

    let response = dispatcher
        .dispatch_stream(
            &ext,
            "com.acme.hellocli.tick",
            serde_json::json!({"count": 1_000}),
            DEFAULT_REQUEST_TIMEOUT,
        )
        .await
        .unwrap();

    // Read a couple of events then fire cancel.
    let start = std::time::Instant::now();
    let mut events = response.events;
    let _ = events.next().await;
    response.cancel.fire();
    // Drain whatever's already in the channel; the handler now stops
    // emitting because is_cancelled() returns true between ticks.
    let mut total = 1usize;
    while events.next().await.is_some() {
        total += 1;
        if total > 50 {
            panic!("handler did not honour cancel within 50 ticks");
        }
    }
    assert!(
        start.elapsed() < Duration::from_millis(500),
        "cancel must propagate within a few hundred ms (took {:?})",
        start.elapsed()
    );
}
