## Done

- Added `starter-ext-sdk` and `starter-ext-sdk-macros` crates to the `starter-extensions/` workspace; updated workspace `Cargo.toml` to register them and add `tokio`/`syn`/`quote`/`proc-macro2` deps.
- `#[derive(Extension)]` proc-macro reads `block.yaml` at the extension's compile time (path resolved against `CARGO_MANIFEST_DIR`, attribute `#[extension(manifest = "...")]`, default `block.yaml`). Parses against the shared `starter_ext_spi::Manifest`; emits `ExtensionMeta` (id/version/manifest_yaml/manifest_static via `OnceLock`), a per-extension `<Struct>ToolHandlers` trait with one `handle_*` method per `contributes.tools` entry, and `ExtensionDispatch` with a manifest-driven `match`. R4 namespace ownership is enforced at the extension's build. R3 missing/extra handler = compile error in the extension's crate.
- `requires! { name = …, capabilities = [secrets|http_out|fs|wall_clock|tracing] }` declarative macro generates a per-extension `Ctx` newtype with only the requested accessor methods, plus always-present `events()` and `cancel()` mirroring `starter-spi::ai::OnEvent + Cancel`. Unknown category names trigger a `compile_error!`; no untyped `host_call` escape hatch (R6). Each generated Ctx exposes an inherent `REQUIRED_CAPABILITIES: &[&str]` for the host to cross-check at load time.
- Mutually-exclusive `builtin`/`wasm`/`process` cargo features: `compile_error!` fires when zero are enabled; conflicting `#[no_mangle] static __STARTER_EXT_FLAVOUR_MARKER: u8` (one definition per feature) produces a real linker error when more than one is enabled (R1).
- Builtin entry-point glue: `register_static_table! { extension: Foo, ctx: FooCtx, instance: Foo }` emits a `register(&mut BuiltinTable)` function. `BuiltinTable`/`BuiltinEntry` use a closure-erased `Fn(&str, &CtxInner, Value) -> Result<Value>` so the host's dispatch table can hold heterogenous per-extension Ctxs uniformly.
- Process and WASM flavour modules ship as documented Stage-9/Stage-16 placeholders that compile under their feature.
- Re-exports `Manifest`, `Error`, `Result`, `ExtensionId`, `serde_json`, `serde_yaml`, `semver` etc. from the SDK root so generated code can use `::starter_ext_sdk::…` absolute paths and extension `Cargo.toml`s stay minimal.
- Integration test (`tests/derive_extension.rs` + `tests/fixtures/hello.block.yaml`) exercises the full chain end-to-end. Full workspace `cargo test`: 38 tests pass, 0 failures.

## Next

- Stage 5 (next session): `starter-ext-host` (manifest loader, two-phase validate/commit, namespace + capability checks, `ExtensionRegistry`) plus an `examples/hello-builtin` fixture that consumes the SDK end-to-end. Phase 1 of SCOPE.

## What you need to know

- Manifest path tracking uses `include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/", manifest_rel))` so a `block.yaml` edit invalidates the extension crate's cache regardless of which `.rs` file invoked the derive (proc-macros can't use the unstable `tracked_path` on stable Rust).
- The crate uses `#![deny(unsafe_code)]` with narrow `#[allow(unsafe_code)]` on the three `__STARTER_EXT_FLAVOUR_MARKER` statics — `#[no_mangle]` trips the lint. Body of the crate is otherwise unsafe-free.
- `builtin::BuiltinEntry::new` takes a closure rather than a generic ExtensionDispatch type so the host's table can store heterogenous Ctx-typed extensions; the closure body (emitted by `register_static_table!`) re-wraps `CtxInner` into the per-extension Ctx newtype via `XxCtx::__from_inner(ctx_inner.clone())` before invoking `ExtensionDispatch::dispatch_tool`.
- `CtxInner::new` requires the host to supply backend impls for `SecretsBackend` / `HttpOutBackend` / `FsBackend` / `WallClockBackend` / `TracingBackend`. Those traits exist as the seam; no concrete impls yet — they land with `starter-ext-host` in Stage 5+.

## Open questions

- (none)
