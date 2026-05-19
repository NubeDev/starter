# starter-ext-smoke

Cross-crate smoke tests for `starter-extensions`. One crate, one
`cargo test -p starter-ext-smoke`, every named scenario in
`DOCS/extensions/scope/SCOPE.md` § "Smoke tests (before merging
anything)" — covered either by an integration test in this crate or by
an explicit pointer to where the check lives in its natural home.

## Scenarios

| SCOPE name                                                | Where it lives                                                                                                |
| --------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| One source, three flavours                                | `tests/one_source_three_flavours.rs` (manifest + source audit) + the **CI matrix** (real compile per flavour) |
| Extension survives host restart                           | `starter-ext-supervisor/tests/hello_process.rs::handshake_and_shutdown_end_to_end` + supervisor unit tests    |
| Crash loop is bounded                                     | `tests/crash_loop_is_bounded.rs`                                                                              |
| Capability violation is rejected, logged, counted         | `tests/capability_violation.rs`                                                                               |
| Two extensions, no React duplication                      | `packages/starter-ext-ui` vitest (`pnpm -r run test`)                                                         |
| Bad manifest is isolated to its own extension             | `starter-ext-mcp/tests/hello_builtin_end_to_end.rs::bad_manifest_is_isolated_to_its_own_extension`            |
| LLM-facing description is byte-identical at load + call   | `tests/r7_description_byte_identical.rs`                                                                      |
| Streaming-response cancels promptly (SSE + CLI + MCP)     | `starter-ext-server/tests/rest_routes.rs::streaming_response_cancels_promptly` + `starter-ext-cli/tests/hello_cli_end_to_end.rs::cancel_fires_within_a_few_hundred_ms` |
| Same-source streams over four transports                  | `tests/streaming_convention_is_one.rs` (the convention) + each adapter's streaming test                       |
| Extension author has zero starter-workspace deps          | `tests/zero_extra_deps.rs`                                                                                    |

## CI matrix

`.github/workflows/starter-extensions.yml` runs three jobs:

1. `cargo test --workspace` against the sibling workspace.
2. A `cargo check -p hello-*` matrix, one job per `example × flavour`,
   so the mutually-exclusive `builtin` / `wasm` / `process` feature
   guard is exercised on every PR. Two flavours simultaneously trips
   the duplicate-`#[no_mangle]` linker trap; zero flavours trips the
   `compile_error!`.
3. A dedicated `extension-author dep audit` job that runs
   `cargo test -p starter-ext-smoke --test zero_extra_deps` on its own,
   so a regression there shows up as its own red dot.
