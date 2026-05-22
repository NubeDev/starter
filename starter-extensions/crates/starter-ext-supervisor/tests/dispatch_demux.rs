//! End-to-end: spawn the `hello-process` example binary and exercise the
//! `SupervisorHandle::call` request/response demultiplexer.
//!
//! This is the load-bearing test for PEER-REVIEW Rec 1: it proves the
//! supervisor can correlate JSON-RPC dispatch ids with the matching
//! response frame the child produces, which is the foundation the four
//! transport adapters (`starter-ext-cli`, `starter-ext-server`,
//! `starter-ext-grpc`, `starter-ext-mcp`) rely on to remove their
//! `ProcessXxxDispatcher::NotWired` paths.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use starter_ext_host::ExtensionRecord;
use starter_ext_spi::{ExtensionId, LifecycleState};
use starter_ext_supervisor::Supervisor;
use tempfile::TempDir;

fn fixtures_root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    here.parent().unwrap().parent().unwrap().to_path_buf()
}

fn hello_process_bin() -> PathBuf {
    let workspace_root = fixtures_root().parent().unwrap().to_path_buf();
    workspace_root
        .join("target")
        .join("debug")
        .join("hello-process")
}

fn stage_bundle() -> Option<(TempDir, ExtensionRecord)> {
    let bin = hello_process_bin();
    if !bin.exists() {
        return None;
    }
    let src_root = fixtures_root().join("examples").join("hello-process");
    let bundle = tempfile::tempdir().expect("tempdir");

    std::fs::copy(
        src_root.join("block.yaml"),
        bundle.path().join("block.yaml"),
    )
    .unwrap();
    std::fs::copy(&bin, bundle.path().join("hello-process")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perm = std::fs::metadata(bundle.path().join("hello-process"))
            .unwrap()
            .permissions();
        perm.set_mode(0o755);
        std::fs::set_permissions(bundle.path().join("hello-process"), perm).unwrap();
    }

    let yaml = std::fs::read_to_string(bundle.path().join("block.yaml")).unwrap();
    let manifest: starter_ext_spi::Manifest = serde_yaml::from_str(&yaml).unwrap();
    let record = ExtensionRecord {
        id: Some(ExtensionId::new("com.acme.hello").unwrap()),
        id_hint: "com.acme.hello".to_string(),
        bundle_dir: bundle.path().to_path_buf(),
        state: LifecycleState::Validated,
        manifest: Some(manifest),
        failure: None,
    };
    Some((bundle, record))
}

async fn wait_for_running(handle: &starter_ext_supervisor::SupervisorHandle) {
    let mut state_rx = handle.state();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        if *state_rx.borrow() == LifecycleState::Running {
            return;
        }
        if tokio::time::Instant::now() >= deadline {
            panic!(
                "child never reached Running; events: {:#?}",
                handle.events()
            );
        }
        let _ = tokio::time::timeout(Duration::from_millis(200), state_rx.changed()).await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn call_demultiplexes_echo_response() {
    let Some((_bundle, record)) = stage_bundle() else {
        eprintln!(
            "skipping: target/debug/hello-process not built. \
             Run `cargo build -p hello-process` first."
        );
        return;
    };

    let handle = Supervisor::start(&record).expect("supervisor start");
    wait_for_running(&handle).await;

    // Round-trip the echo tool. `hello-process` implements
    // `com.acme.hello.echo` and the child loop routes `tools/<id>` into
    // `ExtensionDispatch::dispatch_tool`.
    let input = json!({ "msg": "ping", "n": 42 });
    let out = handle
        .call(
            "tools/com.acme.hello.echo",
            input.clone(),
            Duration::from_secs(5),
        )
        .await
        .expect("call should round-trip");
    assert_eq!(out, input, "echo handler returns the input verbatim");

    // Two concurrent calls should both demultiplex correctly — proves
    // the per-id pending-map routing isn't accidentally serialising.
    let h2 = handle.clone();
    let h3 = handle.clone();
    let a = tokio::spawn(async move {
        h2.call(
            "tools/com.acme.hello.echo",
            json!({"k": "a"}),
            Duration::from_secs(5),
        )
        .await
    });
    let b = tokio::spawn(async move {
        h3.call(
            "tools/com.acme.hello.echo",
            json!({"k": "b"}),
            Duration::from_secs(5),
        )
        .await
    });
    let ra = a.await.unwrap().expect("a");
    let rb = b.await.unwrap().expect("b");
    assert_eq!(ra, json!({"k": "a"}));
    assert_eq!(rb, json!({"k": "b"}));

    handle.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn call_unknown_method_returns_kernel_error() {
    let Some((_bundle, record)) = stage_bundle() else {
        eprintln!("skipping: target/debug/hello-process not built");
        return;
    };

    let handle = Supervisor::start(&record).expect("supervisor start");
    wait_for_running(&handle).await;

    // The child's SDK loop responds with a JSON-RPC error envelope for
    // unknown methods; the supervisor must surface that as Err(...).
    let err = handle
        .call("tools/does-not-exist", json!({}), Duration::from_secs(5))
        .await
        .expect_err("unknown method must error");
    let msg = format!("{err}");
    assert!(
        msg.contains("unknown") || msg.contains("not found") || msg.contains("does-not-exist"),
        "unexpected error message: {msg}"
    );

    handle.shutdown().await;
}

#[tokio::test(flavor = "current_thread")]
async fn call_times_out_when_child_is_slow() {
    // Validate the timeout path without depending on a slow handler in
    // the child: use a very short deadline against a non-existent
    // extension binary. The supervisor never reaches Running, so the
    // call cannot succeed; the timeout (or transport error from a
    // closed channel) must surface in bounded time.
    let bundle = tempfile::tempdir().unwrap();
    let yaml = r#"
v: 1
id: com.acme.absent
version: 0.0.1
display_name: "Absent"
runtime:
  kind: process
  bin: does-not-exist
"#;
    std::fs::write(bundle.path().join("block.yaml"), yaml).unwrap();
    let manifest: starter_ext_spi::Manifest = serde_yaml::from_str(yaml).unwrap();
    let record = ExtensionRecord {
        id: Some(ExtensionId::new("com.acme.absent").unwrap()),
        id_hint: "com.acme.absent".to_string(),
        bundle_dir: bundle.path().to_path_buf(),
        state: LifecycleState::Validated,
        manifest: Some(manifest),
        failure: None,
    };
    let handle = Supervisor::start(&record).expect("supervisor start");

    // Either the call times out OR the supervisor task drops the
    // pending sender first (child failed to spawn). Both are valid
    // transport errors; we only assert the call returns in bounded
    // time rather than hanging forever.
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        handle.call("tools/anything", json!({}), Duration::from_millis(500)),
    )
    .await;

    match result {
        Ok(Err(_)) => { /* expected */ }
        Ok(Ok(v)) => panic!("unexpectedly succeeded with {v}"),
        Err(_) => panic!("call did not return in bounded time"),
    }

    let _ = tokio::time::timeout(Duration::from_secs(2), handle.shutdown()).await;
}
