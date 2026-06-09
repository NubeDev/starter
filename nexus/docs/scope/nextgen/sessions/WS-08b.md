# WS-08b — Datasource-kinds declaration format + MQTT connector

> Status: Done
> Started: 2026-06-10 (UTC)
> Finished: 2026-06-09 18:13 (UTC)
> Commit: a16185a1
> Branch: `nexus-gaps` · Migration block: `13xx` (none needed — manifest-only kinds, no schema)

The human-unblocked successor to WS-08. HUMAN DECISION (2026-06-09): build the WS-10
datasource-kind declaration format FIRST, then the MQTT connector against it. Modbus is
DROPPED. `rumqttc` is feature-gated OFF by default.

## Re-verification (ROADMAP §0)

Re-grepped WS-08's "Current state (evidence)" claims against branch tip (`84708c5a`):

- `DatasourceKind` has one variant `Postgres` — CONFIRMED (`nexus-spi/src/dto/datasource/shared.rs:11`).
- Registered engine inputs are only `http_poll` + `simulator` — CONFIRMED
  (`nexus-engine/src/registry/inputs.rs`).
- The vendored ArkFlow is connector-trimmed (heavy MQTT/Modbus/Kafka deps removed) — CONFIRMED
  (`backend/vendor/arkflow-plugin/Cargo.toml` description + only `memory`/`generate` inputs in
  `src/input/`). So MQTT needs a nexus-authored client against a new gated dep, not an ArkFlow
  input registration — matching the WS-08 blocker analysis.
- WS-10 shipped query-kinds only (registry/loader/lint at `nexus-api/src/kinds/**`); the
  datasource-kind declaration format (§4.1B) was NOT built. CONFIRMED — this WS builds it.

No drift required a WS-doc edit; the WS-08 evidence still holds. (WS-08.md keeps its
`Verified: 82a6a19a`; the connector facts are unchanged at tip.)

## Task breakdown

1. Datasource-kind declaration format — a declarative manifest mirroring the query-kind
   registry/loader: each datasource-kind declares `name`, `surface` (query|stream),
   `config_schema` (JSON Schema), `secret_fields`, `test` (probe descriptor), and an optional
   `dialect` (time-macro mapping for query connectors). `nexus-api/src/datasource_kinds/**`.
2. A built-in datasource-kinds pack: `postgres` (query) + `mqtt` (stream), loaded at boot,
   config validated against each kind's schema.
3. MQTT connection probe in `nexus-store` behind a `mqtt` feature (rumqttc, OFF by default).
   The feature-off build returns a clear "not enabled" error rather than a fake success.
4. Registry mounted on `AppState`; a read-only catalogue route
   `GET /api/v1/datasources/kinds` so the UI can render per-kind config forms (DTO-first).
5. Mirrored tests: pack load + lints, config-schema validation, secret-field declaration,
   MQTT probe (feature-off error path; feature-on closed-port failure).

## Assumptions / deviations

- The datasource-kind format is a WS-10 §4.1B deliverable; per the human decision WS-08b owns
  building it. It lives beside the query-kind registry as a sibling module rather than editing
  WS-10's `kinds/**` (which is query-kind-specific), keeping each registry single-responsibility.
- Per the relaxed acceptance, "ingests end-to-end from a live broker" is a documented MANUAL
  follow-up (no broker in an unattended run); the MQTT *connection test* is verified by
  unit/mock tests.
- Modbus dropped per the decision; no `tokio-modbus` added.

## Follow-ups

- **MANUAL: live-broker ingestion.** The relaxed unattended acceptance covers the
  declaration format + an MQTT *connection test* (unit/mock). Verifying that an MQTT
  datasource ingests events end-to-end from a live broker into a live panel/flow is a
  manual follow-up that needs a broker stood up (deferred — no broker in an unattended run).
- **MQTT through the HTTP `/datasources/test` route needs a DTO reshape.** The existing
  `TestConnectionRequest` DTO is Postgres-shaped (host/port/database/user/password). Wiring the
  MQTT probe through the shared pre-save test route would require reshaping that shared DTO
  (per-kind config) — the original WS-08 blocker. The probe is verified directly by unit tests
  (`nexus-store/src/datasource/mqtt/probe.rs`) to satisfy the relaxed acceptance without
  touching the shared DTO. Reshape is a separate WS.
- **MQTT feature flag.** `rumqttc` is OFF by default (`mqtt` feature on `nexus-store`). The
  default build returns an honest "not enabled" error; enable with
  `cargo build -p nexus-store --features mqtt` (or wire the feature up through `nexus-api`) when
  a build that actually connects is wanted.

## Done — verification

- `cargo test -p nexus-api --lib` → 106 passed (incl. 12 `datasource_kinds::*`).
- `cargo test -p nexus-store --lib datasource::mqtt` → feature-off "not enabled" path passes.
- `cargo test -p nexus-store --features mqtt --lib datasource::mqtt` → feature-on closed-port
  failure passes.
- DTO-first: `cargo run --bin nexus-openapi` (additive `GET /api/v1/datasources/kinds` +
  `DatasourceKind{List,Summary}` schemas) → `pnpm codegen`.
- UI gate: `pnpm typecheck` / `pnpm test` (215 pass, 7 skip) / `pnpm build` all green.
