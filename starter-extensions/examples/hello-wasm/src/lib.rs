//! WASM-flavour mirror of `examples/hello-builtin` and
//! `examples/hello-process`. SCOPE.md "One source, three flavours" smoke
//! test for Kernel Phase 4: the trait impl is byte-identical to the
//! sibling examples; only the entry-point macro flips
//! (`register_static_table!` → `register_process_main!` →
//! `register_wasm_main!`).
//!
//! Built as a WASI-p2 component (`cargo component build --target
//! wasm32-wasip2`) the produced `hello_wasm.wasm` is what
//! `starter-ext-wasm`'s `WasmHost::compile_component` consumes; on the
//! host target this crate compiles to a regular `rlib` so the workspace's
//! `cargo test` stays green even though no host-side code calls into it
//! (the WASM-specific glue inside `starter_ext_sdk::wasm::run_wasm_main`
//! is `#[cfg(target_family = "wasm")]`-gated).

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
        // Echo the input verbatim. Identical to the builtin and process
        // handlers — SCOPE R1 demands that swapping flavours does not
        // touch this body.
        Ok(params)
    }
}

// Emits the WASM-flavour entry point. Inside a `wasm32-wasip2` build
// this expands to the `starter:extension/guest` interface
// implementation that `starter-ext-wasm` calls into; on a host build
// (e.g. `cargo test` on Linux/macOS) it expands to nothing so the
// workspace's smoke tests still compile.
starter_ext_sdk::register_wasm_main! {
    extension: Hello,
    ctx: HelloCtx,
    instance: Hello,
}
