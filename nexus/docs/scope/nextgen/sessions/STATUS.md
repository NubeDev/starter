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
| 2 | WS-04 | Panel Editor & Viz Config | ⬜ | | | | renderers ready |
| 3 | WS-07 | Alerting depth | ⬜ | | | | mostly independent |
| 4 | WS-10 | Kinds — declarative extensibility | ⬜ | | | | reuses WS-03 binder |
| 5 | WS-08 | Connector breadth (MQTT/Modbus/…) | ⬜ | | | | feeds WS-06 palette |
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
