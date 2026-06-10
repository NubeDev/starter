# RW-07 — Extension data-plane: sources, sinks, insights as contributions

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.
> Depends on RW-04 (writer registry) + RW-06 (insights store).

## Current state

- WS-14 extension system: manifests (`block.yaml`), runtimes builtin/wasm/process
  (`starter-ext-spi` `RuntimeKind`, manifest.rs ~201-209), boot lint/materialise
  (`nexus-api/src/extensions/boot.rs`), cleanup providers, host methods
  (`extensions/host_methods.rs` ~118-135: `authz.check`, `dashboard.read`,
  `warehouse.query`).
- Existing data contributions: `contributes.warehouse_templates` (query-kinds — the
  dispatcher's third source) and `contributes.ui`. Example: `extensions/com.nexus.hello/`.
- Engine registry (RW-01/02) and datasource writer registry (RW-04) are plain maps —
  ready for runtime registration.

## Scope

1. `contributes.insights[]` — manifest entries `{ name, script_file, params_schema }`.
   Boot path mirrors query-kinds exactly: lint (compile the Rhai script against the
   RW-06 sandbox at registration — reject on compile error), materialise into the RW-06
   insights table under the extension's id namespace (`com.vendor.ext.name`), cleanup
   provider removes them on purge. This is the smallest slice and proves the pattern.
2. `contributes.sources[]` / `contributes.sinks[]` — manifest declares
   `{ name, config_schema, direction }`. For `process`/`wasm` runtimes the extension
   cannot link engine traits, so the bridge is host-mediated:
   - new host method `ingest.write` — extension pushes JSON rows tagged with a registered
     source name; the host stamps tenant from the extension's install identity (NEVER
     from the payload), converts via json_to_arrow, and feeds the named flow source's
     bounded channel. Backpressure: the host method returns `retry_after` when the
     channel is full — document this in the SPI.
   - sink direction: host method `ingest.read_batch` (long-poll) or push via the
     supervisor's JSON-RPC to the extension — pick ONE based on what
     `starter-ext-supervisor` already supports best; document the choice in the
     session log.
3. Engine-side: `source/extension.rs` + `sink/extension.rs` nodes registered under the
   contributed names at extension boot, deregistered on disable/purge. A flow referencing
   a missing extension node fails to build with a clear error (test).
4. Authz: gate `ingest.*` host-method categories the same way `warehouse` is gated
   (see the kernel category gate noted in host_methods.rs comments). Migration `19xx`
   only if registration state needs persistence beyond the existing extension tables.
5. Update `com.nexus.hello` to contribute one demo insight (e.g. `hello.zscore`) so the
   e2e example covers the new path.

## Non-goals

`starter-ext-spi`/supervisor redesigns (additive manifest fields only — if a needed
change is breaking, that is a TODOs.md blocker, not a guess), wasm component model work
beyond what `starter-ext-wasm` already provides, extension-contributed datasource kinds
(follow-up).

## Acceptance

- `com.nexus.hello` ships an insight contribution: boot lints + materialises it,
  `POST /query` can apply it, `DELETE …?purge=true` removes it (extend the existing
  hello e2e).
- A process-runtime test extension pushes rows through `ingest.write` into a flow that
  lands them in a datasource sink (docker-gated e2e); tenant stamped by host, verified.
- Channel-full path returns the documented backpressure response (test with a tiny
  channel capacity).
- Disable/purge deregisters nodes; a flow using them errors cleanly afterward.
