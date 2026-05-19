//! Minimal builtin-flavour extension. SCOPE.md "Phase 1" smoke test.
//!
//! Demonstrates the *whole* extension-author surface in <30 lines:
//!
//! 1. Derive [`starter_ext_sdk::Extension`] — reads `block.yaml` at the
//!    extension's compile time (R3).
//! 2. Declare the Ctx shape via `requires!{}`. We need no capabilities,
//!    so the macro emits a Ctx with only the always-present `events()` /
//!    `cancel()` methods (R6).
//! 3. Implement the generated `HelloToolHandlers` trait. One handler per
//!    `contributes.tools[]` entry; a missing or extra handler is a
//!    compile error in this crate (R3).
//! 4. Call `register_static_table!` to expose this extension to a host
//!    that statically linked it in.
//!
//! The handler body is intentionally trivial — Phase 1's point is to
//! prove the kernel + adapter wiring works; subsequent phases (process
//! flavour, WASM flavour, full capability surface) layer on without
//! changing this file.

use starter_ext_sdk::Extension;

/// The extension's unit struct. SCOPE R5: no fields. State lives in
/// the host-provided Ctx; the struct itself is `()`-sized.
#[derive(Extension)]
#[extension(manifest = "block.yaml")]
pub struct Hello;

starter_ext_sdk::requires! {
    name = HelloCtx,
    capabilities = [],
}

impl HelloToolHandlers for Hello {
    type Ctx = HelloCtx;

    fn handle_com_acme_hello_echo(
        &self,
        _ctx: &Self::Ctx,
        params: starter_ext_sdk::serde_json::Value,
    ) -> starter_ext_sdk::Result<starter_ext_sdk::serde_json::Value> {
        // Echo the input verbatim. The MCP adapter has already validated
        // `params` against `schemas/echo_in.json` (the input_schema named
        // by `block.yaml`) before invoking this handler.
        Ok(params)
    }
}

starter_ext_sdk::register_static_table! {
    extension: Hello,
    ctx: HelloCtx,
    instance: Hello,
}
