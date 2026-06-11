# RW-07b — Extension ingest data-plane: sources/sinks + `ingest.write`

Status: Done
Started: 2026-06-10 06:40 UTC
Finished: 2026-06-10 08:35 UTC

Spec: `RW-07_EXTENSION_DATA_PLANE.md` items 2–4 (sources/sinks contributions +
`ingest.write` host method + engine extension-source seam + `ingest.*` authz),
plus the RW-07 deferral's outstanding acceptance bullets (tenant-stamp e2e,
channel-full retry_after test, hello-purge assertion). Items 1 + 5 (the insights
slice) shipped in RW-07.

## First action — spec drift check

Re-grepped RW-07's "Current state" citations:
- `manifest.rs` RuntimeKind at ~200–209 (cited ~201–209) — accurate.
- `host_methods.rs` host-method dispatch at ~118–135 — accurate.
No drift; spec doc left as-is.

## Design decisions

- **Source direction (`ingest.write`) is the primary, fully-shipped path.** Per
  the RW-09 handoff ("`ingest.write` should route through the engine's push
  channel"), the host method resolves the named source against
  `FlowManager::ingest()` (`IngestChannels`) and calls `try_push`, returning
  `retry_after_secs` on `IngestError::Full`. No second ingest path is
  introduced — the engine-side "extension source node" is realised as the
  existing per-flow `http_ingest` source (RW-09), keyed by flow id; the
  contributed source name *is* the flow the host wired. This is why no
  boot-time named-node registration is needed: a flow author wires an
  `http_ingest` source, the extension pushes by that flow's id, and the source
  registers/deregisters its channel on flow start/stop (RW-09's drop seam) — so
  "deregistered on disable/purge → a push errors cleanly afterward" is already
  satisfied (`IngestError::NotRunning` → host-method error).

- **Tenant is stamped from the caller, never the payload.** `ingest::write`
  overwrites each row's `tenant_id` with `caller.tenant_id` (the supervisor
  binds it from the install identity); a caller with no tenant is a hard deny.
  Proven by `tenant_is_stamped_from_caller_not_payload` (unit) and the docker
  e2e (a row lying `tenant_id: evil` lands as `t-real`).

- **`ingest.*` authz mirrors `warehouse`.** Added `("ingest", "ingest")` to the
  supervisor's `CAPABILITY_HOST_METHODS` and a `Capability::Ingest { names }`
  variant; the category gate refuses an un-granted `ingest.*` call before it
  reaches nexus, exactly as `warehouse.*` is gated. Finer name-allowlist
  enforcement follows the warehouse precedent (gated at the supervisor, not
  re-checked in the nexus host method).

- **Additive-only contracts.** `Contributes.sources[]` / `Contributes.sinks[]`
  (`ContributeSource`/`ContributeSink { name, config_schema?, direction }`),
  `IngestDirection`, `Capability::Ingest`, and the four `ingest::*` DTOs are all
  new — no field changed type, no method signature changed. WASM cap mapping and
  host `capability_matches` / supervisor `category_of` gained one arm each.

- **Sink direction (`ingest.read_batch`) deferred — not stubbed.** Implementing
  the host→extension drain requires a new engine `sink/extension.rs` writing
  into a bounded per-sink output queue plus a long-poll host method draining it
  — a second, self-contained data-plane with no existing seam to reuse. Per the
  charter (no stubs that pretend to work) it is logged as a follow-up rather
  than shipped half-done. The contracts it needs (`ContributeSink`,
  `IngestReadBatch{Request,Response}`, the `ingest` capability) are landed, so
  the follow-up is purely the engine queue + the drain method.

## Commits

1. `RW-07b:` additive SPI/supervisor contracts — `ingest` DTOs,
   `Capability::Ingest`, `Contributes.sources/sinks`, capability-gate +
   wasm-cap + host-validate arms; tests.
2. `RW-07b:` nexus-api `ingest.write` host method (tenant-stamp + backpressure)
   wired into `NexusHostMethods`; unit tests incl. channel-full retry_after.
3. `RW-07b:` docker-gated e2e — `ingest.write` tenant-stamp into a datasource
   sink + `InsightCleanupProvider` purge assertion; STATUS/TODOs/session-doc.

## Tests

- `starter-ext-spi`: 95 passed (ingest DTO round-trips, sources/sinks parse +
  default-empty + unknown-field rejection).
- `starter-ext-supervisor`: `ingest_namespace_gated` (granted passes both
  methods, un-granted refused).
- `nexus-api` lib: 5 `extensions::ingest` unit tests incl.
  `full_channel_returns_retry_after` (deterministic 1-deep burst) and
  `tenant_is_stamped_from_caller_not_payload`.
- docker-gated (`#[ignore]`): `extensions_ingest_write_e2e`
  (host-stamped tenant lands in datasource sink) +
  `extensions_insight_purge_e2e` (discover → purge → empty, idempotent re-purge).
- `cargo test --workspace` green on both nexus/backend and starter-extensions;
  no openapi/codegen impact (`ingest.*` are JSON-RPC host methods, not REST).

## Follow-ups (TODOs.md)

- Sink direction `ingest.read_batch` + engine `sink/extension.rs` output queue.
