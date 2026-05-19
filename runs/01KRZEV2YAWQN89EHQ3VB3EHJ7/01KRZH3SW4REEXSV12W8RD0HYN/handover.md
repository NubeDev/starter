## Done

- Reviewed prior Phase-1 commits (c1f8e85 spi, ed4ec42 sdk, f20a9a4 host+mcp+example) against rulebook R1/R2/R4/R5/R13
- Confirmed `cargo test` green: 15 + 2 + 7 + 4 + 27 = 55 unit/integration tests across the workspace, including `hello_builtin_is_reachable_through_mcp` and `bad_manifest_is_isolated_to_its_own_extension`
- Verified crate Cargo.toml dependency arrows match SCOPE §"Dependency arrow (Rust)" — no inversions, no transport crates pulled into kernel
- Verified R4 namespace check covers tools/cli/rest/grpc/workers in `crates/starter-ext-host/src/validate.rs`
- Verified `starter-ext-mcp` reuses `starter-mcp::ToolRegistry` rather than redefining MCP wire shape (R2 single-transport, wire untouched)

## Next

- Stage 7 begins Kernel Phase 2 (supervisor + process flavour + admin routes); a fresh session picks it up
- No code patches required from this REVIEW stage

## What you need to know

- Sentinel below is the only thing the runtime parses to decide whether to continue
- All 4 hard Layer-1 invariants pass; the `contribute-to-adapter` pattern is proven by `ext-mcp` consuming `contributes.tools` from validated records and emitting per-extension-isolated failures via `RegisterError::Collected`

## Open questions

- (none)
