## Done

- Added `CliStreaming { None, Stdout }` enum + `streaming` field on `ContributeCli` in `starter-ext-spi` (deny_unknown_fields preserved; parse-roundtrip test added — 29/29 SPI tests pass).
- Re-exported `CliStreaming` + `StreamId` from `starter-ext-sdk` so extension authors don't need a direct `starter-ext-spi` dep.
- New crate `starter-extensions/crates/starter-ext-cli` with:
- `CliDispatcher` trait taking `timeout: Duration` on both `dispatch` and `dispatch_stream`.
- `BuiltinCliDispatcher` + `BuiltinCliRegistry` for per-(ext, cli_id) closure registration; non-streaming runs on the tokio blocking pool with a `tokio::time::timeout` wrapper; streaming wires `mpsc` → `Stream` and a `watch::Sender` for cancel.
- `ProcessCliDispatcher` / `WasmCliDispatcher` — v0.1 stubs returning `DispatchError::NotWired`, both carrying the configurable `request_timeout` knob.
- `ExtensionSubcommand` impls `starter_cli::Command`; builds clap surface from `args_schema`, supports `--input <JSON>` escape hatch and `--timeout-ms <MS>` override.
- SIGINT handling: first `Ctrl-C` fires `CancelHandle`; second exits with code 130.
- `build_cli_commands` detects command-name collisions before producing any subcommand.
- 6 unit tests + 4 end-to-end tests in `tests/hello_cli_end_to_end.rs` — all pass.
- `examples/hello-cli` builtin-flavour extension: block.yaml with one non-streaming (`hellocli-greet`) and one streaming (`hellocli-tick`) entry, schemas, docs, lib with handlers, main wiring through `starter-cli::CommandRegistry`. Verified: `cargo run -p hello-cli -- hellocli-greet --name "Phase 6"` and `... hellocli-tick --count 3 --label demo` produce the expected stdout.
- Committed as fc95f7b on branch `codeless/starter-extensions`.

## Next

- Stage 15: next adapter phase (gRPC or workers per WORKFLOW order).

## What you need to know

- The `cargo build --workspace` from the starter-extensions root still fails with an existing `__STARTER_EXT_FLAVOUR_MARKER` redefinition because cargo unifies mutually-exclusive `builtin`/`process`/`wasm` features across the hello-* examples. This was present before stage 14 (the marker is a pre-existing mechanism); targeted `cargo build -p <crate>` / `cargo test -p <crate>` are the correct way to verify.
- The proc-macro (`starter-ext-sdk-macros`) was deliberately left untouched. CLI handlers register through `BuiltinCliRegistry::register{,_streaming}` keyed by `(ExtensionId, cli_id)` — not via the `*ToolHandlers` trait. The lib doc on `starter-ext-cli` explains this and points at the example.
- The process/wasm dispatcher stubs intentionally accept `request_timeout` in their constructor and observe it in their NotWired error message; the synchronous JSON-RPC body for those flavours requires extending `SupervisorHandle` with a response-demultiplexer (`call(method, params, timeout) -> Result<Value>`), which is the natural shape for the next slice.
- The streaming renderer reacquires the stdout lock per event so the lock guard never spans an `await` (Send-bounds on the returned `Future`); each `writeln!` is followed by a `flush()`.

## Open questions

- (none)
