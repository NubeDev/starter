//! # starter-ext-sdk
//!
//! What an extension author imports. SCOPE.md "What each crate / package
//! owns — `starter-ext-sdk`" defines the contract:
//!
//! - `#[derive(Extension)]` reads `block.yaml` at the extension's compile
//!   time and emits the metadata + handler-trait scaffolding (R3).
//! - `requires!{}` declares the capability categories the extension needs
//!   and generates a per-extension `Ctx` newtype whose method set reflects
//!   that declaration (R6). No untyped `host_call(method, params)` escape
//!   hatch in v0.1.
//! - Mutually-exclusive cargo features (`builtin`, `wasm`, `process`)
//!   gate the entry-point glue. Misconfiguration (zero or more than one
//!   flavour) is caught at build time — `compile_error!` for zero,
//!   duplicate `#[no_mangle]` symbol for more than one. SCOPE R1.
//! - Builtin entry-point glue: `register_static_table!` exposes the
//!   extension to a host that statically linked it in.
//! - The `Ctx` always carries a `Stream<Item = Event>` shape mirroring
//!   `starter-spi::ai::OnEvent + Cancel` (Stage 4 explicit requirement).
//!
//! The SDK re-exports the few `starter-ext-spi` items proc-macro-generated
//! code needs to name (`Manifest`, `ExtensionId`, `Error`, `Result`,
//! `serde_yaml`, `serde_json`, `semver`). Doing so keeps the extension's
//! own `Cargo.toml` minimal — one dep, not five.

// `deny(unsafe_code)` rather than `forbid`, with a narrow `allow` on the
// flavour-marker statics below: `#[no_mangle]` triggers the
// `unsafe_code` lint in current rustc (it lets a crate squat on a
// linker-global symbol), and the marker symbols are the entire point of
// the multi-flavour linker check. The crate body otherwise contains no
// unsafe code; the `deny` keeps that property visible.
#![deny(unsafe_code)]
#![warn(missing_docs)]

// ---------------------------------------------------------------------------
// Mutually-exclusive flavour gate (SCOPE R1)
// ---------------------------------------------------------------------------

#[cfg(not(any(feature = "builtin", feature = "wasm", feature = "process")))]
compile_error!(
    "starter-ext-sdk: exactly one of the `builtin`, `wasm`, `process` features must be enabled. \
     SCOPE.md R1: the entry-point glue differs per flavour; enabling zero or more than one is \
     a build-time misconfiguration."
);

// The duplicate-symbol trick is what gives "more than one flavour enabled"
// a *linker* error (the wording SCOPE R1 specifically uses). Each flavour
// declares its own rustc-visible item (so workspace feature unification
// does not produce a rustc E0428 — multiple flavours can co-exist in the
// same crate graph at `cargo check` time), but all three resolve to the
// same `#[export_name]` linker symbol, so the linker refuses to produce
// any binary that pulls in two or more flavours at once.
//
// Why this matters: cargo's workspace feature unifier will happily turn
// on all three features on `starter-ext-sdk` when sibling example crates
// each enable a different flavour. The SDK itself still has to compile
// in that mode; the actual "exactly one flavour per binary" invariant is
// enforced where it should be — at link time of the final binary.
//
// We expose the symbol with a deliberately-loud name so the linker
// diagnostic points the operator at the right page in SCOPE.md.

/// Marker symbol emitted exactly once per linked binary when a single
/// flavour feature is enabled. Duplicate definition (more than one
/// flavour reached the same binary) is a linker error; absent definition
/// (zero flavours) is caught by the `compile_error!` above.
///
/// The value is the flavour code (0 = builtin, 1 = wasm, 2 = process) —
/// purely informational; the link-time check does not inspect it.
#[cfg(feature = "builtin")]
#[allow(unsafe_code)]
#[export_name = "__STARTER_EXT_FLAVOUR_MARKER"]
#[doc(hidden)]
pub static __STARTER_EXT_FLAVOUR_MARKER_BUILTIN: u8 = 0;
#[cfg(feature = "wasm")]
#[allow(unsafe_code)]
#[export_name = "__STARTER_EXT_FLAVOUR_MARKER"]
#[doc(hidden)]
pub static __STARTER_EXT_FLAVOUR_MARKER_WASM: u8 = 1;
#[cfg(feature = "process")]
#[allow(unsafe_code)]
#[export_name = "__STARTER_EXT_FLAVOUR_MARKER"]
#[doc(hidden)]
pub static __STARTER_EXT_FLAVOUR_MARKER_PROCESS: u8 = 2;

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------

pub mod ctx;
pub mod meta;

#[cfg(feature = "builtin")]
pub mod builtin;
#[cfg(feature = "process")]
pub mod caller_local;
#[cfg(feature = "process")]
pub mod host_backends;
#[cfg(feature = "process")]
pub mod host_rpc;
#[cfg(feature = "process")]
pub mod process;
#[cfg(feature = "wasm")]
pub mod wasm;

// ---------------------------------------------------------------------------
// Public re-exports
//
// What the proc-macro and `requires!{}` expansion needs to name with an
// absolute path. Keeping these here means generated code looks the same
// regardless of which `use` statements the extension author wrote.
// ---------------------------------------------------------------------------

pub use ctx::{Cancel, CallerIdentity, EmitEvent, Event, EventReceiver, EventSender};
pub use meta::{ExtensionDispatch, ExtensionMeta};

/// Re-export so extensions can construct `Event { stream_id, payload }`
/// without taking a direct `starter-ext-spi` dep (SCOPE "Extension
/// author has zero starter-workspace deps beyond starter-ext-sdk +
/// serde_json").
pub use starter_ext_spi::jsonrpc::StreamId;
pub use starter_ext_spi::warehouse::{Row, TemplateSpec, WarehouseReadRequest};
pub use starter_ext_spi::event_bus::{EventBusMessage, EventBusPublishRequest, EventBusSubscribeRequest};
pub use starter_ext_spi::{
    AuthGate, Authority, Backoff, Capability, CliStreaming, ContributeCli, ContributeGrpc,
    ContributeRest, ContributeTool, ContributeUi, ContributeUiExpose, ContributeWorker,
    Contributes, Error, ExtensionBehavior, ExtensionId, HealthConfig, JsonRpcEnvelope,
    LifecycleState, Manifest, OnErrorPolicy, PathSpec, RestStreaming, RestartPolicy, Result,
    RetryStrategy, Runtime, RuntimeKind, Supervision,
};

/// Re-export so generated code can name `::starter_ext_sdk::semver::Version`.
pub use semver;
/// Re-export so generated code can name `::starter_ext_sdk::serde_json::Value`
/// without the extension's `Cargo.toml` adding `serde_json` itself.
pub use serde_json;
/// Re-export so generated code can lazily parse the embedded `block.yaml`.
pub use serde_yaml;

/// `#[derive(Extension)]` — see the [crate-level docs](crate) and
/// SCOPE.md "What each crate / package owns: starter-ext-sdk".
pub use starter_ext_sdk_macros::Extension;

// ---------------------------------------------------------------------------
// `requires! { … }` — declarative macro generating a per-extension `Ctx`
// newtype whose method set is determined by the capability categories the
// extension declares (SCOPE R6).
// ---------------------------------------------------------------------------

/// Declare the capability categories an extension requires, and generate
/// the typed `Ctx` newtype it receives at runtime.
///
/// Invoke once per extension crate:
///
/// ```ignore
/// starter_ext_sdk::requires! {
///     name = WeatherCtx,
///     capabilities = [secrets, http_out, wall_clock],
/// }
/// ```
///
/// The macro emits:
///
/// - `pub struct WeatherCtx` wrapping the SDK's internal handle. The struct
///   is opaque — the only way to obtain one is from the host's entry-point
///   glue.
/// - One method per declared capability category, returning a typed
///   sub-handle. Categories not declared are not even *named* on the
///   type — `ctx.http()` on a `Ctx` that did not declare `http_out` does
///   not type-check (R6, "no untyped `host_call` escape hatch").
/// - `events()` and `cancel()` methods that mirror
///   `starter-spi::ai::OnEvent + Cancel`. These are *always* present on
///   every Ctx because streaming + cancellation are kernel-level (post-R13
///   stream sub-protocol).
///
/// The macro also emits a `const REQUIRED_CAPABILITIES: &[&str]` slice the
/// host reads at load time to cross-check against the manifest's
/// `requires:` block. Mismatch (declared in `requires!{}` but absent from
/// `block.yaml`, or vice versa) is a load-time validation error in
/// `starter-ext-host`.
#[macro_export]
macro_rules! requires {
    (
        name = $ctx_name:ident,
        capabilities = [ $( $cap:ident ),* $(,)? ] $(,)?
    ) => {
        /// Per-extension Ctx newtype generated by `starter_ext_sdk::requires!`.
        ///
        /// Method set is the union of:
        ///
        /// - One accessor per capability category listed in the macro
        ///   invocation. Methods absent from the macro invocation are absent
        ///   from this type — there is no `host_call(name, params)` escape
        ///   hatch (SCOPE R6).
        /// - The always-present `events()` / `cancel()` pair mirroring
        ///   `starter-spi::ai::OnEvent + Cancel` (Stage 4 requirement,
        ///   post-R13 stream sub-protocol).
        // Extension authors are not required to call every accessor
        // (e.g. a tool that doesn't emit stream events leaves
        // `events()` and `cancel()` untouched, and a fixture in the
        // SDK's own tests may declare zero capabilities). Allowing
        // dead code on the emitted type keeps clippy quiet in those
        // cases without forcing every test/fixture to invoke every
        // accessor.
        #[allow(dead_code)]
        #[derive(Clone)]
        pub struct $ctx_name {
            inner: $crate::ctx::CtxInner,
        }

        #[allow(dead_code)]
        impl $ctx_name {
            /// The capability categories declared by `requires!`. The
            /// host reads this slice at load time and cross-checks
            /// against the manifest's `capabilities:` block (R6).
            pub const REQUIRED_CAPABILITIES: &'static [&'static str] =
                &[ $( stringify!($cap) ),* ];

            /// Construct from an SDK-internal handle. Called only by the
            /// per-flavour entry-point glue; extensions never call this.
            #[doc(hidden)]
            pub fn __from_inner(inner: $crate::ctx::CtxInner) -> Self {
                Self { inner }
            }

            /// Borrow the bounded channel for emitting events back to the
            /// host. Mirror of `starter-spi::ai::OnEvent` — bounded
            /// `mpsc::Sender<Event>`, sync callers `try_send` and drop on
            /// overflow, async callers `.await` each send. The host turns
            /// these into `stream.event` JSON-RPC notifications.
            pub fn events(&self) -> &$crate::EventSender {
                self.inner.events()
            }

            /// Borrow the cancellation handle. Mirror of
            /// `starter-spi::ai::Cancel`. Extension handlers poll
            /// `is_cancelled()` between long steps or
            /// `.select!` against `cancelled().await` in async contexts.
            pub fn cancel(&self) -> &dyn $crate::Cancel {
                self.inner.cancel()
            }

            /// Identity of the principal this invocation is dispatched
            /// on behalf of. `None` for host-internal frames (health,
            /// init, lifecycle); populated for every tenant-scoped
            /// frame the supervisor stamped. Always-present accessor
            /// (no capability gate) — the kernel surfaces identity to
            /// every extension so handlers can scope work to the
            /// requesting tenant without re-deriving it from `params`.
            pub fn caller(&self) -> ::core::option::Option<&$crate::CallerIdentity> {
                self.inner.caller()
            }

            $(
                $crate::__requires_capability_method!($cap);
            )*
        }

        impl ::std::fmt::Debug for $ctx_name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!($ctx_name)).finish_non_exhaustive()
            }
        }

    };
}

/// Internal helper expanding to one accessor method per capability category.
///
/// Lives at crate root so `requires!{}` can name it via `$crate::`. Each
/// arm returns a category-specific borrow — secrets, http, fs, wall clock,
/// custom — implemented as thin views over the same internal `CtxInner`.
///
/// Adding a new category here is the SDK side of the additive-extension
/// pattern SCOPE describes ("Adding a new host method is an additive trait
/// extension"). The matching `Capability` variant lives in
/// `starter-ext-spi`.
#[doc(hidden)]
#[macro_export]
macro_rules! __requires_capability_method {
    (secrets) => {
        /// Secret-store access. Granted by `capabilities.secrets:` in
        /// `block.yaml`; an empty prefix list neutralises every call.
        pub fn secrets(&self) -> &$crate::ctx::SecretsHandle {
            self.inner.secrets()
        }
    };
    (http_out) => {
        /// Outbound HTTP. Granted by `capabilities.http_out:` in
        /// `block.yaml`; an empty allowlist neutralises every call.
        pub fn http(&self) -> &$crate::ctx::HttpOutHandle {
            self.inner.http_out()
        }
    };
    (fs) => {
        /// Filesystem access. Granted by `capabilities.fs:`.
        pub fn fs(&self) -> &$crate::ctx::FsHandle {
            self.inner.fs()
        }
    };
    (wall_clock) => {
        /// Wall-clock time access. Boolean grant.
        pub fn wall_clock(&self) -> &$crate::ctx::WallClockHandle {
            self.inner.wall_clock()
        }
    };
    (tracing) => {
        /// Structured tracing surface. Always-safe; treated as a capability
        /// so the manifest documents which extensions emit telemetry.
        pub fn tracing(&self) -> &$crate::ctx::TracingHandle {
            self.inner.tracing()
        }
    };
    (warehouse_read) => {
        /// Named-template warehouse reads. Granted by
        /// `capabilities.warehouse_read.tables:` in `block.yaml`; an
        /// empty `tables` allowlist neutralises every call. The host
        /// binds `$caller_tenant_id` from `ctx.caller()` so a tool
        /// invoked without a tenant-scoped caller is refused.
        pub fn warehouse_read(&self) -> &$crate::ctx::WarehouseReadHandle {
            self.inner.warehouse_read()
        }
    };
    (event_bus) => {
        /// In-process publish/subscribe bus. Granted by
        /// `capabilities.event_bus.publish:` / `subscribe:` in
        /// `block.yaml`; empty allowlists neutralise the
        /// corresponding direction. V1 exposes `publish` only.
        pub fn event_bus(&self) -> &$crate::ctx::EventBusHandle {
            self.inner.event_bus()
        }
    };
    (warehouse_write) => {
        /// Warehouse-table writes. Granted by
        /// `capabilities.warehouse_write.tables:` in `block.yaml`;
        /// an empty `tables` allowlist neutralises every insert. The
        /// host stamps `tenant_id` from `ctx.caller()` onto every
        /// row, so a tool invoked without a tenant-scoped caller is
        /// refused. Companion to `warehouse_read`.
        pub fn warehouse_write(&self) -> &$crate::ctx::WarehouseWriteHandle {
            self.inner.warehouse_write()
        }
    };
    (dashboard) => {
        /// SDUI dashboard read/write. Granted by
        /// `capabilities.dashboard_read.pages:` /
        /// `capabilities.dashboard_write.pages:` in `block.yaml`.
        /// Each call is gated against the caller's tenancy from
        /// `ctx.caller()`; cross-tenant reads or writes are refused.
        pub fn dashboard(&self) -> &$crate::ctx::DashboardHandle {
            self.inner.dashboard()
        }
    };
    (authz) => {
        /// Authorization-engine probe. Granted by
        /// `capabilities.authz_check.kinds:` in `block.yaml`. The
        /// host binds the calling principal from `ctx.caller()`
        /// before evaluating `(action, resource)`.
        pub fn authz(&self) -> &$crate::ctx::AuthzHandle {
            self.inner.authz()
        }
    };
    ($other:ident) => {
        compile_error!(concat!(
            "starter_ext_sdk::requires!: unknown capability category `",
            stringify!($other),
            "`. Known categories: secrets, http_out, fs, wall_clock, tracing, warehouse_read, warehouse_write, event_bus, dashboard, authz. ",
            "If you need a host-specific category, declare it as `custom` in the manifest \
             and obtain it via the host's `Custom` capability variant — there is no untyped \
             `host_call` escape hatch (SCOPE R6)."
        ));
    };
}

// ---------------------------------------------------------------------------
// `register_static_table!` — builtin entry-point glue (SCOPE R1, "builtin").
// ---------------------------------------------------------------------------

/// Builtin flavour entry-point glue.
///
/// SCOPE R1: builtin extensions are statically linked into the host. They
/// register themselves into a host-side dispatch table at startup. This
/// macro is the only call an extension author makes to expose itself to a
/// host that built it in.
///
/// Invoke once per extension crate, *after* the `#[derive(Extension)]`
/// type and its `Handlers` impl:
///
/// ```ignore
/// starter_ext_sdk::register_static_table! {
///     extension: Weather,
///     instance: Weather, // unit struct → const-constructible
/// }
/// ```
///
/// The macro emits a `#[linkme::distributed_slice]`-style entry under a
/// fixed symbol the host (`starter-ext-host`) scans at startup. For v0.1
/// we stay on stable Rust without `linkme`: the host calls a discovery
/// function the SDK exposes per crate (`__starter_ext_register`) via a
/// `#[ctor]`-style constructor — there isn't one on stable, so the
/// host's `Loader::commit()` calls a user-provided `register_extensions!`
/// macro that lists every linked crate. The list is short (one line per
/// extension); the alternative (link-time discovery) is a stability
/// landmine on stable Rust.
///
/// In Phase 1 (this stage) the macro emits a `pub fn register(table: &mut
/// BuiltinTable)` symbol the host calls explicitly. Phase 2 of the SDK
/// will introduce link-time discovery if and when stable Rust gains it.
#[cfg(feature = "builtin")]
#[macro_export]
macro_rules! register_static_table {
    ( extension: $ty:ty, ctx: $ctx_ty:ty, instance: $instance:expr $(,)? ) => {
        /// Register this extension's dispatch entries into a host-side
        /// table. Called by the host's `BuiltinRegistry::collect()` once
        /// per linked extension at startup.
        ///
        /// This is the single symbol that distinguishes a builtin
        /// extension from a non-extension Rust library — it is the
        /// builtin-flavour analogue of the WASM component's
        /// `wit_bindgen::export!` and the process binary's
        /// `tokio::main`.
        ///
        /// The closure inside re-wraps the host's `CtxInner` into the
        /// per-extension `Ctx` newtype generated by `requires!{}` before
        /// invoking `ExtensionDispatch::dispatch_tool`. That keeps the
        /// host's dispatch table flat (one closure type per extension,
        /// erased into `Arc<BuiltinDispatchFn>`) without losing the
        /// typed `Ctx` discipline of R6.
        pub fn register(table: &mut $crate::builtin::BuiltinTable) {
            // SAFETY note: `$instance` is moved into the closure once and
            // shared across calls. SCOPE R5 demands `&self` methods, so a
            // single instance shared across calls is correct by design.
            let instance: $ty = $instance;
            table.insert(
                <$ty as $crate::ExtensionMeta>::id().clone(),
                $crate::builtin::BuiltinEntry::new(
                    <$ty as $crate::ExtensionDispatch>::declared_tool_ids(),
                    move |tool_id, ctx_inner, params| {
                        let ctx = <$ctx_ty>::__from_inner(ctx_inner.clone());
                        <$ty as $crate::ExtensionDispatch>::dispatch_tool(
                            &instance, tool_id, &ctx, params,
                        )
                    },
                ),
            );
        }
    };
}

// ---------------------------------------------------------------------------
// `register_process_main!` — process flavour entry-point glue (SCOPE R1).
//
// Mirrors `register_static_table!` but emits a `fn main()` rather than a
// host-side registration function. The body calls `process::run_process_main`
// passing a closure that re-wraps `CtxInner` into the per-extension `Ctx`
// newtype generated by `requires!{}`.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// `register_wasm_main!` — wasm-flavour entry-point glue (SCOPE R1, "wasm").
// Stage 11 — Kernel Phase 4.
//
// Mirrors `register_static_table!` (builtin) and `register_process_main!`
// (process) for the WASI-p2 component flavour. Emits a `pub fn dispatch`
// the wasm host calls into; on `wasm32-wasip2` builds a future
// `wit_bindgen::export!` invocation will route the WIT-side
// `starter:extension/guest.dispatch-tool` export onto this symbol, so the
// shape today is what every future bindgen generation will hang on.
// ---------------------------------------------------------------------------

/// Wasm-flavour entry-point glue.
///
/// SCOPE R1: wasm extensions are WASI-p2 components instantiated by
/// `starter-ext-wasm::WasmHost`. The host calls into the component's
/// `starter:extension/guest.dispatch-tool` export; this macro is the
/// single call an extension author makes to expose itself to that host.
///
/// Invoke once per extension crate, *after* the `#[derive(Extension)]`
/// type and its `Handlers` impl:
///
/// ```ignore
/// starter_ext_sdk::register_wasm_main! {
///     extension: Hello,
///     ctx: HelloCtx,
///     instance: Hello,
/// }
/// ```
///
/// The macro emits `pub fn dispatch(tool_id, params) -> Result<Value>`
/// that routes through [`crate::wasm::run_wasm_main`]. The function is
/// the symbol a future `wit_bindgen::export!` invocation hangs on; on
/// a host-target build (e.g. the workspace's `cargo test`) it stays an
/// ordinary public function so the example compiles cleanly even
/// without a wasm runtime.
#[cfg(feature = "wasm")]
#[macro_export]
macro_rules! register_wasm_main {
    ( extension: $ty:ty, ctx: $ctx_ty:ty, instance: $instance:expr $(,)? ) => {
        /// Route one `dispatch-tool` call into the extension's typed
        /// handler. Called by the wasm host
        /// (`starter-ext-wasm::WasmHost::dispatch_tool`) via the
        /// WIT-side `starter:extension/guest.dispatch-tool` export
        /// once the v0.2 `wit_bindgen` integration lands; today this
        /// is the canonical entry-point shape, callable directly.
        pub fn dispatch(
            tool_id: &str,
            params: $crate::serde_json::Value,
        ) -> $crate::Result<$crate::serde_json::Value> {
            let instance: $ty = $instance;
            $crate::wasm::run_wasm_main(&instance, tool_id, params, |inner| {
                <$ctx_ty>::__from_inner(inner)
            })
        }
    };
}

/// Process-flavour entry-point glue.
///
/// SCOPE R1: process extensions are child binaries spawned by
/// `starter-ext-supervisor`. The supervisor speaks framed JSON-RPC over
/// stdio; this macro emits a `#[tokio::main]` that drives the loop:
/// init handshake → dispatch (`health`, `shutdown`, `tools/<id>`) → exit.
///
/// Invoke once per extension crate, *after* the `#[derive(Extension)]`
/// type and its `Handlers` impl:
///
/// ```ignore
/// starter_ext_sdk::register_process_main! {
///     extension: Hello,
///     ctx: HelloCtx,
///     instance: Hello,
/// }
/// ```
///
/// The macro emits a single `pub async fn run() -> Result<()>`. A
/// process-flavour extension's binary `main` calls it inside its own
/// `#[tokio::main]` — keeping the macro free of `fn main` means the
/// same macro invocation works from a lib (so an integration test can
/// call `run()` directly) and from a binary crate.
#[cfg(feature = "process")]
#[macro_export]
macro_rules! register_process_main {
    ( extension: $ty:ty, ctx: $ctx_ty:ty, instance: $instance:expr $(,)? ) => {
        /// Drive the process-flavour wire loop until the supervisor
        /// disconnects or asks us to shut down.
        pub async fn run() -> $crate::Result<()> {
            let instance: $ty = $instance;
            $crate::process::run_process_main(instance, |inner| <$ctx_ty>::__from_inner(inner))
                .await
        }
    };
}

#[cfg(test)]
mod tests {
    //! Compile-time sanity tests for the `requires!` macro. The flavour
    //! feature gate is exercised implicitly: this crate's test target is
    //! built with `--features builtin` (the workspace default), which
    //! satisfies the `compile_error!`. Multi-flavour misconfiguration is
    //! a *linker* check and is exercised in the workspace's integration
    //! tests, not here.

    // The `requires!` macro expands into a struct with always-present
    // `events()` and `cancel()` methods plus per-capability accessors.
    // We expand it under a known shape and assert the surface.

    crate::requires! {
        name = NoCapsCtx,
        capabilities = [],
    }

    crate::requires! {
        name = HttpCtx,
        capabilities = [http_out],
    }

    crate::requires! {
        name = MultiCtx,
        capabilities = [secrets, http_out, wall_clock, tracing, fs],
    }

    #[test]
    fn required_capabilities_slice_matches_macro_arguments() {
        // Use assert_eq! against an empty slice rather than is_empty()
        // — clippy::overly_complex_bool_expr / assertions_on_constants
        // flags the latter as always-true because the const slice's
        // length is known at compile time. We *want* to pin that fact;
        // the equality form expresses it without the lint trip.
        assert_eq!(NoCapsCtx::REQUIRED_CAPABILITIES, &[] as &[&str]);
        assert_eq!(HttpCtx::REQUIRED_CAPABILITIES, &["http_out"]);
        assert_eq!(
            MultiCtx::REQUIRED_CAPABILITIES,
            &["secrets", "http_out", "wall_clock", "tracing", "fs"]
        );
    }

    #[test]
    fn flavour_marker_symbol_matches_enabled_feature() {
        // The marker value encodes which flavour feature is active so
        // the linker symbol clash (R1 — "more than one flavour ⇒
        // linker error") has a value to point at when diagnosing.
        // 0 = builtin, 1 = wasm, 2 = process. The test asserts each
        // feature gate emits the matching value; runs under whichever
        // single feature the test invocation enabled.
        #[cfg(feature = "builtin")]
        assert_eq!(super::__STARTER_EXT_FLAVOUR_MARKER_BUILTIN, 0);
        #[cfg(feature = "wasm")]
        assert_eq!(super::__STARTER_EXT_FLAVOUR_MARKER_WASM, 1);
        #[cfg(feature = "process")]
        assert_eq!(super::__STARTER_EXT_FLAVOUR_MARKER_PROCESS, 2);
    }
}
