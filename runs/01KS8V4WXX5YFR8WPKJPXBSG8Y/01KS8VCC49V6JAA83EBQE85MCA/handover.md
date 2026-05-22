## Done

- Added `contributes.nodes` + `ContributeNode` (with reused `AuthGate`) to `starter-ext-spi::manifest`, re-exported, and covered by two new manifest tests.
- Extended `starter-ext-host::validate::check_namespace` to walk `contributes.nodes[].kind` with two distinct rejection paths per R-flow-node-3 (host-reserved prefix vs namespace mismatch) + two new tests.
- Widened `NodeDescriptor` fields to `Cow<'static, str>` with `const fn new(&'static str, …)` shim (zero-alloc for built-ins, `new_owned` for extensions, no `Box::leak`). Switched the 13 `pub const DESCRIPTOR` items in `starter-flow-nodes` to `pub static DESCRIPTOR` so taking `&DESCRIPTOR` still yields `&'static` after Cow added a Drop impl.
- Added `DynamicNodeKindEntry` (descriptor + closure factory returning `Arc<dyn NodeBehavior>`), `DynamicNodeKindRegistry`, and `CompositeNodeKindRegistry` (static-first lookup) in `starter-flow-spi::node`.
- Wired `starter-ext-flow::contributed_node_kinds(manifest, extension_root, behavior_factory) -> Vec<ContributedNodeKind { entry, meta }>` with `ContributedNodeKindMeta::new(...)` constructor; `unbound_behavior_factory()` produces the slice-A `UnboundNodeBehavior` whose `invoke` returns `NodeError::Domain { code: "no_behaviour_bound", … }`. i18n keys default to `<kind>.{label,summary,help}`. Dependencies: added `starter-flow-spi` to the starter-extensions workspace, threaded into ext-flow.
- Added `examples/flow-agent/src/node_kinds.rs`: `NodeKindsState` over `ArcSwap<CompositeNodeKindRegistry>` + `install_dynamic` swap path, en catalog bundled via `include_str!`, three GET routes (`/api/node-kinds`, `/api/node-kinds/{kind}/settings-schema` — files for dynamic kinds, schemars JSON for built-ins; `/api/node-kinds/{kind}/description`). Mounted on the axum router in `server::build`. Three unit tests pass.
- Frontend: `state/node-kinds-store.ts` (react-query, 30s staleTime, `useInvalidateNodeKinds`); `lib/api.ts` gained `NodeKindDto` + `api.nodeKinds.{list,settingsSchema,description}`. `FlowEditor.tsx` merges built-ins with the server descriptors; `pnpm typecheck` clean.
- Test fixture `starter-extensions/crates/starter-ext-flow/tests/fixtures/com.nube.mqtt/` (block.yaml + 2 schemas + 3 markdown files) and integration test `tests/stage_a_contributes_nodes.rs` (4 tests — manifest parses, walker resolves paths, dynamic registry round-trips, placeholder invoke returns the typed `no_behaviour_bound` error).
- `cargo fmt --all` clean; `cargo test -p starter-flow-spi -p starter-flow-nodes -p flow-agent` and `cargo test --manifest-path starter-extensions/Cargo.toml -p starter-ext-spi -p starter-ext-host -p starter-ext-flow` all green. Workspace check passes excluding the pre-existing aws-* MSRV failure.
- Committed as `5208bc2` on `codeless/flow-nodes` with message starting `stage 1 (slice A) — manifest + dynamic registry + flow-agent wiring (descriptors only)`.

## Next

- Slice B (stage 2): `FLOW_NODE_INVOKE` constant + JSON-RPC error-code range on `starter-ext-spi::jsonrpc`; `ProcessNodeProxy` in `starter-ext-flow::process_proxy`; `SupervisorHandle::stream_cancel` helper; `POST /admin/extensions/reload` on flow-agent with the full R-flow-node-6 algorithm; the `examples/flow-agent/extensions/com.nube.mqtt/` bundle (`bin/mqtt-driver` over `rumqttc` + `starter-jsonrpc-stdio`); end-to-end MQTT acceptance test against a `mosquitto` container.
- At the REVIEW gate: paste the validator rejection transcript, `GET /api/node-kinds` response with resolved i18n labels, and the placeholder invoke error transcript per WORKFLOW.md.

## What you need to know

- `cargo clippy --workspace --all-features -- -D warnings` currently fails on the **baseline** branch (`examples/flow-agent/src/cache_demo.rs:176` — `manual_clamp`). Pre-existing; not caused by this stage. Stage 2 may want to clean up before the gate.
- `cargo check --workspace` also fails on the baseline because `aws-sdk-s3` and friends require rustc 1.91+ while the toolchain here is 1.90. Pre-existing; ext-workspace check (`--manifest-path starter-extensions/Cargo.toml`) and `--exclude starter-blob-s3 --exclude starter-blob-garage --exclude starter-blob-compose` both compile clean.
- `cargo fmt --all` rewrote ~150 unrelated files (whitespace from rustfmt's accumulated config drift). These are included in the stage-1 commit because the WORKFLOW gates require `cargo fmt --check` green; do not revert in stage 2.
- The placeholder `UnboundNodeBehavior` lives in `starter-ext-flow` (not in `starter-flow-spi`) so the SPI crate stays free of behaviour-flavour code. Slice B's `ProcessNodeProxy` should plug into the same `behavior_factory` parameter that `contributed_node_kinds` already accepts — swap `unbound_behavior_factory()` for a closure that constructs a proxy per kind.
- The flow-agent's `node_kinds::NodeKindsState::install_dynamic(dynamic, meta)` is the slice-B entry point for the reload algorithm; the `ArcSwap` is already in place so the swap can be wait-free.
- Built-in node-kind settings schemas are materialised in `node_kinds::builtin_settings_schemas()` — only `log` and `trigger-explicit` are wired today because those are the only `starter-flow-nodes` features `flow-agent` enables. Adding a feature flag means adding a line to that function.
- Frontend `FlowEditor` only merges extension descriptors (skips entries where `extension_id` is null) so the curated `BUILTIN_NODE_KINDS` shape wins on built-ins. The dedicated editor refactor for native reverse-DNS support is explicitly out of scope per the SCOPE's "Out of scope" carve-out.

## Open questions

- (none)
