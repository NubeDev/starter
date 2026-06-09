# Nexus Next-Gen — Build Status Board

> Single source of truth for the orchestration loop. The loop reads this file on every wake.
> Each session updates its own row when it starts, blocks, or finishes.
> All work lands on branch **`nexus-gaps`** (sequential — one WS at a time, no worktrees).

**Legend:** ⬜ pending · 🔵 in-progress · ✅ done (build+tests green, committed) · ⛔ blocked (see [TODOs.md](./TODOs.md))

## Execution queue (dependency order — DO NOT reorder)

This is the order the loop starts sessions. It follows ROADMAP §2/§3 waves so a later
session always finds its dependencies already committed in the working tree.

| Order | WS | Title | Status | Started | Finished | Commit | Notes |
|------:|----|-------|:------:|---------|----------|--------|-------|
| 1 | WS-03 | Query Authoring + Macro Engine (C2 binder) | ✅ | 2026-06-09 11:58 | 2026-06-09 12:21 | ac76c1de | binder+history; B/C pre-shipped; unblocks 01/02/10 |
| 2 | WS-04 | Panel Editor & Viz Config | ✅ | 2026-06-09 12:40 | 2026-06-09 20:05 | d27f293f | full editor + transforms + field config |
| 3 | WS-07 | Alerting depth | ✅ | 2026-06-09 13:05 | 2026-06-09 13:30 | f52d6b38 | multi-condition + no-data/error policy + slack/email + templating + retry |
| 4 | WS-10 | Kinds — declarative extensibility | ✅ | 2026-06-09 13:30 | 2026-06-09 13:55 | dd4687ae | registry+loader+lints+pack(4)+kind-mode dispatch+catalogue route+UI picker; reuses WS-03 binder |
| 5 | WS-08 | Connector breadth (MQTT/Modbus/…) | ⛔ | 2026-06-09 14:00 | 2026-06-09 21:20 | 00e40609 | pre-save `/datasources/test` landed+GREEN; connectors blocked on WS-10 datasource-kind format + gated-deps decision (TODOs.md) |
| 6 | WS-01 | Time Range & Auto-Refresh | ⬜ | | | | needs WS-03 macros |
| 7 | WS-02 | Variables & Templating | ⬜ | | | | needs WS-03 macros |
| 8 | WS-11 | Units & datetime prefs | ⬜ | | | | needs WS-04 + WS-10 |
| 9 | WS-12 | Audit log + undo/redo | ⬜ | | | | changelog substrate |
| 10 | WS-05 | Folders/Rows/Repeat/JSON/Versioning | ⬜ | | | | repeat needs WS-02 |
| 11 | WS-06 | Flows Visual Builder | ⬜ | | | | needs WS-08 palette |
| 12 | WS-09 | Cache/Audit/Rate-limit/HA/OTel | ⬜ | | | | cache key needs C1/C3 |

## Loop log (append one line per wake)

<!-- Format: `YYYY-MM-DD HH:MM — <action taken>` -->
2026-06-09 11:58 — Loop start. Queue empty → spawned WS-03 (first in dependency order) on nexus-gaps.
2026-06-09 12:21 — WS-03 done (commits 855db5ba, ac76c1de). C2 binder + query history landed; Scope B/C were already shipped at base (drift recorded). cargo test + pnpm build/test green; pnpm typecheck blocked only by a pre-existing out-of-lane starter-ui-authz error (TODOs.md). Next: WS-04.
2026-06-09 12:40 — WS-03 ✅, no WS in-progress. Picked first pending in queue order → spawning WS-04 (Panel Editor & Viz Config) on nexus-gaps.
2026-06-09 12:41 — Wake skipped: WS-04 already 🔵 (claimed by a concurrent in-flight wake; heartbeat wake-start ~1min old). No double-spawn. Yielding.
2026-06-09 13:05 — WS-03 ✅ & WS-04 ✅, no WS in-progress, no STOP. Picked first pending in queue order → spawning WS-07 (Alerting depth) on nexus-gaps.
2026-06-09 13:31 — Wake complete: WS-07 subagent returned Done within this wake; gate green and row marked ✅ (f52d6b38). One WS spawned this firing. Next pending → WS-10 (Kinds) on the following wake.
2026-06-09 13:30 — WS-07 ✅. Alerting depth: multi-condition rules (AND/OR, per-condition reducer), per-rule no-data/exec-error policy resolved before the untouched state machine, Slack + Email (SMTP/lettre+rustls) channels alongside webhook, safe single-pass message templating, retry-with-backoff delivery recording per-channel outcome. Fixed a real UI↔backend operator-name mismatch (dialog sent `>`/`>=`, comparator expects gt/gte). DTO-first: openapi.json + TS client regenerated (merged with the concurrent agents schema already at HEAD). cargo build + 36 alerting unit tests green; pnpm typecheck/test(134)/build green. Committed only WS-07 hunks; left the agents session's uncommitted `lib.rs` `pub mod agents;` hunk untouched. C6 (audit/undo) deferred — WS-12 substrate not yet run.
2026-06-09 13:30 — WS-07 ✅, WS-03/04 ✅, no WS in-progress, no STOP. Picked first pending in queue order → spawning WS-10 (Kinds — declarative extensibility) on nexus-gaps.
2026-06-09 13:55 — Wake complete: WS-10 subagent returned Done within this wake; row already ✅ (dd4687ae + 7a442053). Declarative query-kinds: registry+loader+two boot lints+core pack(4 kinds)+kind-mode dispatch+`GET /query/kinds` catalogue+UI KindPicker; reuses WS-03 C2 binder, tenant isolation via host-bound `$caller_tenant_id`. cargo build + `cargo test --workspace --lib` (19 new) green; pnpm typecheck/test(134)/build green. Committed only WS-10 hunks. Flagged pre-existing cross-lane breakage in WS-10.md: docker-gated `tests/routes/authz/grant_gate_test.rs` fails to compile (NewDashboard gained accent/icon from dashboard WS) — left for that owner. One WS spawned this firing. Next pending → WS-08 (Connector breadth) on the following wake.
2026-06-09 14:00 — WS-03/04/07/10 ✅, no WS in-progress, no STOP. Picked first pending in queue order → spawning WS-08 (Connector breadth — MQTT/Modbus/…) on nexus-gaps.
2026-06-09 20:05 — WS-04 ✅. Full-screen Panel Editor (Query/Visualization/Field/Overrides/Legend & Axes/Transforms) with live preview, opened from the side panel. UI-only: panel config rides the opaque layout JSON (no migration, no backend DTO). Field config (unit/decimals/min-max/multi-step thresholds/value mappings) + per-series overrides (colour/unit/displayName/hidden) honoured by gauge/stat/table/line/bar; client transform pipeline (rename/calculated/filter/group-by/reduce/organize) runs after fetch so config edits re-render from cache. pnpm typecheck + test (134 pass) + build green. Committed only WS-04 UI hunks (backend agent/* changes belong to a concurrent session — left untouched).
2026-06-09 21:20 — WS-08 ⛔ partial. Landed the pre-save `POST /datasources/test` acceptance criterion in full: `TestConnectionRequest` DTO → openapi+codegen, a `nexus-store` raw-config `probe()` (single short-lived conn, `SELECT 1`, 10s-bounded, secret dropped, nothing persisted/audited), a thin `test_connection` route (principal-gated, dispatches on kind, shared sanitizer extracted to `probe_outcome.rs`), and a "Test connection" button + `ProbeResult` banner in the create form. Mirrored tests: non-docker closed-port probe, docker-gated success + route e2e, and the UI banner. cargo build/lib-tests/clippy green; pnpm typecheck/test(139)/build green. Connectors themselves BLOCKED — vendored ArkFlow is connector-trimmed (MQTT/Modbus need new gated deps `rumqttc`/`tokio-modbus` + nexus-authored Input impls), the kind-specific config shape is the WS-10 datasource-kind format (Wave 2, not yet built), and query connectors need the PgPool query core reshaped (WS-03 lane). Recommend running WS-10 datasource-kinds first; see TODOs.md. C6 audit/undo deferred (WS-12 substrate not run). Committed only WS-08 hunks.
