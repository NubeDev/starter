//! Process-flavour entry-point glue placeholder (SCOPE R1: process).
//!
//! Process-flavour extensions are spawned by `starter-ext-supervisor` and
//! communicate over stdio JSON-RPC (R10). The full entry-point glue —
//! a `tokio::main` wrapper that reads the init handshake, drives the
//! JSON-RPC loop, and routes incoming methods through
//! `ExtensionDispatch::dispatch_tool` — lands with **Stage 9** of the
//! implementation plan (Kernel Phase 2 — `starter-ext-supervisor`).
//!
//! Stage 4 ships the cargo feature and the marker symbol so a consumer
//! who selects `--features process` today gets a *correct* dependency
//! shape and a linker error if they accidentally also enable a second
//! flavour — but trying to actually run a process-flavour extension
//! against this Stage 4 SDK is intentionally a no-op. The shape is here;
//! the body lands with Phase 2.
//!
//! Keeping the placeholder in the SDK (rather than letting `--features
//! process` compile to nothing) preserves the "one trait, three
//! flavours, one source" guarantee from R1: an extension author flipping
//! the cargo feature is the *only* delta between flavours, and the SDK
//! must compile under every flavour the manifest schema permits.

/// Placeholder. Replaced in Stage 9 by `tokio::main`-style entry-point
/// glue generated alongside `#[derive(Extension)]`.
///
/// The shape this fn will eventually have:
///
/// - Read the init handshake from stdin (host sends `Config`).
/// - Construct `CtxInner` from the in-handshake capability metadata.
/// - Loop: read a JSON-RPC envelope, dispatch through
///   `ExtensionDispatch::dispatch_tool`, write the response.
/// - On `shutdown` notification, call `ExtensionBehavior::on_shutdown`
///   and exit cleanly.
///
/// The function is a no-op today; calling it returns `Err(Spawn)` so
/// any consumer who accidentally wires it up sees a loud failure rather
/// than a silent stall.
pub fn run_process_main() -> starter_ext_spi::Result<()> {
    Err(starter_ext_spi::Error::spawn(
        "starter-ext-sdk: process-flavour entry point not implemented in Stage 4 \
         (lands with Kernel Phase 2 — starter-ext-supervisor)",
    ))
}
