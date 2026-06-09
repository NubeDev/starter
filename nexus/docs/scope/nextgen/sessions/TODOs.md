# Nexus Next-Gen — Blockers & Questions for the Human

> Sessions are forbidden to ask questions or hack/stub around problems. When a session hits
> something it genuinely cannot resolve (a real design ambiguity, a hard dependency on work a
> later session owns, a missing decision, a flaky/broken external dependency), it **writes a
> dated entry here, marks its WS `⛔ blocked` in STATUS.md, and the loop moves to the next
> unblocked workstream.**
>
> Triage these in the morning. When you resolve one, the next loop wake will see the WS is
> unblocked (status back to ⬜) and re-run it.

## How to write an entry (sessions follow this format)

```
### [YYYY-MM-DD HH:MM] WS-xx — <one-line blocker title>
- **What I was doing:** <the concrete task>
- **The blocker:** <why it can't proceed without a human decision — be specific>
- **Options I see:** <2–3 concrete options, with the trade-off of each>
- **My recommendation:** <which option, and why>
- **What I did instead:** <skipped / partial-landed X / marked WS blocked>
- **To unblock me:** <the exact decision or change you need to make>
```

---

## Open blockers

<!-- newest first -->

### [2026-06-09 21:20] WS-08 — connector breadth blocked on the WS-10 datasource-kind format + a gated-deps decision
- **What I was doing:** wiring the actual non-Postgres connectors (MQTT/Modbus first, per the WS-08
  priority order) so a live panel/flow ingests from a device source.
- **The blocker (three coupled, genuinely undecidable-here problems):**
  1. **The vendored ArkFlow is connector-trimmed.** `vendor/arkflow-plugin/Cargo.toml:14-16` and
     `src/input/` show the heavy upstream inputs (MQTT/Modbus/Kafka via `modbus`/`rdkafka`/…) are
     *deliberately removed* — only `memory`/`generate` remain. So MQTT/Modbus are **not** "register the
     ArkFlow input"; they need a nexus-authored `Input` impl against a **new gated client dep**
     (`rumqttc` for MQTT, `tokio-modbus` for Modbus). Adding a default-on heavy dep / choosing the
     feature-gating is exactly the "ask, don't guess" call HOW-TO-CODE §9 reserves.
  2. **The datasource record is Postgres-shaped** (`host/port/database/db_user/secret`). MQTT needs
     topic/QoS; Modbus needs unit-id/register map. Carrying kind-specific config requires either
     reshaping `NewDatasource`/`DatasourceRecord` or adding a JSON-config column (migration `1301`).
     **That config shape is the WS-10 datasource-kind declaration format — Wave 2, NOT YET BUILT**
     (WS-10 shipped only query-kinds). WS-08's own header says "WS-10 owns the declaration format; this
     WS supplies the builders." Inventing the format here would redefine a shared contract I don't own.
  3. *Query* connectors (HTTP-REST/Prometheus) additionally need the deeply `PgPool`-typed query core
     (`nexus-store/src/query/run.rs`, `datasource_pools` → `PgPool`) reshaped to be source-polymorphic
     — that reaches into WS-03's binder/runner, out of WS-08's lane.
  - The "ingests from MQTT/Modbus end-to-end" acceptance criterion also needs a live broker/device to
    verify, which an unattended run can't stand up verifiably.
- **Options I see:**
  - (a) Run **WS-10 datasource-kinds (Wave 2)** first to fix the declaration format, decide the
    gated-deps policy, then re-run WS-08 to author the builders against it. *Clean; correct ordering
    per the roadmap; defers connectors.*
  - (b) Let WS-08 add a `kind`-specific JSON-config column + `DatasourceKind::Mqtt` now with a
    nexus-authored `rumqttc` input behind a `mqtt` feature, **pre-empting** the WS-10 format. *Faster
    to a demo, but bakes a config shape WS-10 may have to migrate — the collision the roadmap warns
    against.*
  - (c) Ship only HTTP-REST as a *query* connector. *Rejected: requires the PgPool→polymorphic query
    reshape (WS-03's lane) — bigger and more cross-lane than the device connectors.*
- **My recommendation:** (a). The roadmap already sequences WS-10 datasource-kinds in Wave 2 ahead of
  the WS-08 builders; honour it. Make the gated-deps decision (`rumqttc`/`tokio-modbus`, feature-gated
  off by default) at the same time.
- **What I did instead:** landed the **pre-save `POST /datasources/test`** acceptance criterion in full
  (DTO + store probe + thin route + UI form button + mirrored tests; all gates green) — it closes the
  documented "test only works after save" gap and already dispatches on `kind`, so a future connector
  adds one match arm. Logged this blocker; marked WS-08 row partial in STATUS.md.
- **To unblock me:** confirm the WS-10 datasource-kind declaration format (config-schema/secret-fields/
  test-query/dialect) and the gated-deps decision, then re-run WS-08 to author the builders.

### ✅ RESOLVED [2026-06-09 12:25] — dead `TenantRail`/`TenantChildren` removed in commit `0d8131f3`; `pnpm typecheck` is green workspace-wide. No longer blocks any session.

### [2026-06-09 12:40] WS-03 — pre-existing `pnpm typecheck` failure in `starter-ui-authz` (out of lane)
- **What I was doing:** running the DoD gate (`pnpm typecheck && pnpm build`) for WS-03.
- **The blocker:** `packages/starter-ui-authz/src/panels/authz-admin.tsx:305` declares
  `function TenantRail(...)` that is never used → `TS6133 'TenantRail' is declared but its value is
  never read`, which fails `pnpm typecheck` for the whole workspace. This file is **committed at
  HEAD (`90939747`)**, is **not touched by WS-03**, and is **outside WS-03's owned files** (ROADMAP
  §4 — WS-03 owns `features/query-editor/**` + query/query-history API). It fails on a clean base
  independent of my work.
- **Options I see:** (a) delete the unused `TenantRail` function (one line of dead code) — but it's
  another workstream's file; (b) leave it for the owning session / a human to clean up.
- **My recommendation:** (a) — it's a trivial dead-code removal and the gate is shared; but per the
  "stay in your lane / commit only your hunks" rule I did **not** edit it.
- **What I did instead:** WS-03's own code is fully green — `cargo test` passes (binder: 17/17),
  `pnpm build` is green, `pnpm test` is green (90 passed). Only `pnpm typecheck` trips on this
  unrelated file. Landed all WS-03 work; flagging this so it doesn't read as a WS-03 regression.
- **To unblock the shared typecheck gate:** remove the unused `TenantRail` in
  `starter-ui-authz/src/panels/authz-admin.tsx` (the session that owns that package, or a human).
