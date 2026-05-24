## Done

- Verified `rubix/crates/rubix-tools/src/flow_ops/deploy.rs` writes a new row via `FlowDefStore::insert_revision` and does not (and does not need to) call any `DefinitionManager` directly.
- Confirmed hot-reload path: PG migration `rubix-store-postgres/migrations/flows_definitions/0001_flows_definitions.sql` installs trigger `flows_definitions_notify_trg` emitting `NOTIFY rubix_flows_definitions` on INSERT/UPDATE.
- Confirmed listener side: `rubix/crates/rubix-agent/src/boot/flow_notify.rs::spawn_flow_notify` LISTENs on `FLOWS_DEFINITIONS_CHANNEL`, re-reads `body_yaml` from `flows_definitions`, parses/converts via `rubix_flows`, and calls `on_reload((flow_id, revision, body))`. The closure is wired by `boot::mcp::register` to `FlowRegistry::register`, which is the local equivalent of "DefinitionManager::publish".

## Next

- Proceed to Stage 6 of the job plan.

## What you need to know

- There is no `DefinitionManager` type in this repo; the role is played by `starter_flow`'s `FlowRegistry`. The stage instructions anticipated this ("publish path likely already runs via the rubix-side store + NOTIFY listener … per goals-2-4-3 PR #32 — confirm") — confirmed.
- Skipped git: no code changes, no commit (stage instructions say "otherwise mark git as skipped — verification only").
- Call chain summary for future stages: `FlowDeployTool::invoke` → `FlowDefStore::insert_revision` (PG row) → trigger NOTIFY `rubix_flows_definitions` → `PgListener` in `spawn_flow_notify` → re-SELECT body → `rubix_flows::parse_yaml`+`convert` → `ReloadFn` → `FlowRegistry::register` (hot-reload, no rubix-agent restart).

## Open questions

- (none)
