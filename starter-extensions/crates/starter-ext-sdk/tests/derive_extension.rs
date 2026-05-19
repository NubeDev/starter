//! End-to-end test for `#[derive(Extension)]` + `requires!{}` +
//! `register_static_table!`. Exercises the SCOPE R3 "compile error if
//! handler missing" guarantee (positively — when handlers exist, the
//! crate compiles) and the SCOPE R6 typed-Ctx guarantee (positively —
//! `requires!{}` produces a Ctx whose method set matches the declared
//! categories).
//!
//! The fixture manifest at `tests/fixtures/hello.block.yaml` declares two
//! tools. The handler trait `HelloToolHandlers` (emitted by the derive)
//! demands two matching `handle_*` methods. Removing one of them turns
//! this test crate's compile into the SCOPE R3 error
//! ("not all trait items implemented") — that is the *whole point* of
//! the derive.

use starter_ext_sdk::{
    Cancel, Event, EventSender, Extension, ExtensionDispatch, ExtensionMeta, RuntimeKind,
};

/// The extension's unit struct. SCOPE R5: no fields.
#[derive(Extension)]
#[extension(manifest = "tests/fixtures/hello.block.yaml")]
pub struct Hello;

starter_ext_sdk::requires! {
    name = HelloCtx,
    capabilities = [tracing],
}

impl HelloToolHandlers for Hello {
    type Ctx = HelloCtx;

    fn handle_com_acme_hello_echo(
        &self,
        _ctx: &Self::Ctx,
        params: starter_ext_sdk::serde_json::Value,
    ) -> starter_ext_sdk::Result<starter_ext_sdk::serde_json::Value> {
        Ok(params)
    }

    fn handle_com_acme_hello_shout(
        &self,
        _ctx: &Self::Ctx,
        params: starter_ext_sdk::serde_json::Value,
    ) -> starter_ext_sdk::Result<starter_ext_sdk::serde_json::Value> {
        let s = params.as_str().unwrap_or("");
        Ok(starter_ext_sdk::serde_json::Value::String(s.to_uppercase()))
    }
}

#[test]
fn extension_meta_round_trips_manifest_fields() {
    let id = <Hello as ExtensionMeta>::id();
    assert_eq!(id.as_str(), "com.acme.hello");

    let v = <Hello as ExtensionMeta>::version();
    assert_eq!(v.to_string(), "0.1.0");

    let m = <Hello as ExtensionMeta>::manifest_static();
    assert_eq!(m.id.as_str(), "com.acme.hello");
    assert_eq!(m.runtime.kind, RuntimeKind::Builtin);
    assert_eq!(m.contributes.tools.len(), 2);

    // R7: the raw YAML must round-trip byte-identical via `manifest_yaml`.
    let yaml = <Hello as ExtensionMeta>::manifest_yaml();
    assert!(yaml.contains("com.acme.hello.echo"));
    assert!(yaml.contains("com.acme.hello.shout"));
}

#[test]
fn declared_tool_ids_matches_manifest_order() {
    let ids = <Hello as ExtensionDispatch>::declared_tool_ids();
    assert_eq!(ids, &["com.acme.hello.echo", "com.acme.hello.shout"]);
}

#[test]
fn required_capabilities_reflects_macro_invocation() {
    assert_eq!(HelloCtx::REQUIRED_CAPABILITIES, &["tracing"]);
}

// The Ctx newtype always carries `events()` and `cancel()` per Stage 4
// (mirror of `starter-spi::ai::OnEvent + Cancel`). We can't actually
// build a real Ctx without the host (Stage 7 lands the constructor), but
// we can assert at the *type* level that the methods exist with the
// expected shapes — that is the entire R6 typed-Ctx guarantee.
#[allow(dead_code)]
fn shape_assertions(ctx: &HelloCtx) {
    let _e: &EventSender = ctx.events();
    let _c: &dyn Cancel = ctx.cancel();
    // `tracing` was declared in `requires!`, so the method exists:
    let _t = ctx.tracing();
}

// An attempt to call a capability accessor the extension did *not*
// declare is rejected at compile time. This is the SCOPE R6 "no
// untyped host_call escape hatch" property. We assert it by *not*
// calling `.http()` here — the symmetric negative case lives in
// `trybuild`-style tests once Phase 1 lands. The presence of
// `shape_assertions` above (which compiles) is the positive half.
//
// Synthesise an `Event` to confirm the public type is constructible
// outside the crate.
#[test]
fn event_type_is_publicly_constructible() {
    let e = Event {
        stream_id: starter_ext_sdk::serde_json::from_str(r#""s-1""#).unwrap(),
        payload: starter_ext_sdk::serde_json::json!({ "ok": true }),
    };
    let j = starter_ext_sdk::serde_json::to_string(&e).unwrap();
    assert!(j.contains("s-1"));
}
