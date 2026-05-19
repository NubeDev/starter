//! WASM-flavour entry-point glue placeholder (SCOPE R1: wasm).
//!
//! WASM-flavour extensions are WASI-p2 components instantiated by
//! `starter-ext-wasm`. The full entry-point glue — a `wit_bindgen::export!`
//! invocation generated alongside `#[derive(Extension)]` against the
//! `starter:extension@0.1.0` WIT package — lands with **Stage 16** of the
//! implementation plan (Kernel Phase 4 — `starter-ext-wasm`).
//!
//! Stage 4 ships the cargo feature and the marker symbol so a consumer
//! who selects `--features wasm` today gets a *correct* dependency shape
//! and a linker error if they accidentally also enable a second flavour.
//! Trying to actually compile a real WASM extension against this Stage 4
//! SDK fails at the WIT-bindings step; the shape is here, the body lands
//! with Phase 4.

/// Placeholder. Replaced in Stage 16 by a `wit_bindgen::export!`
/// invocation that wires the WIT-side `dispatch_tool` import to the
/// proc-macro-generated `ExtensionDispatch::dispatch_tool`.
///
/// The function is a no-op today; the WASM host never calls it. It
/// exists so the `#[cfg(feature = "wasm")]` module is not empty —
/// keeping an empty module would still satisfy the linker check (the
/// `__STARTER_EXT_FLAVOUR_MARKER = 1` definition is in `lib.rs`), but
/// it would confuse a reader who looked for the flavour's entry-point
/// surface and found nothing.
pub fn run_wasm_main() -> starter_ext_spi::Result<()> {
    Err(starter_ext_spi::Error::spawn(
        "starter-ext-sdk: wasm-flavour entry point not implemented in Stage 4 \
         (lands with Kernel Phase 4 — starter-ext-wasm)",
    ))
}
