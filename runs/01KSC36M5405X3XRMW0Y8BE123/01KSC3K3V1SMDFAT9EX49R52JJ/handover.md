## Done

- verified Group C diff (boot::migrations::apply_migrations(Option<&str>), boot::clickhouse::apply_ch_migrations(Option<&str>, Option<&str>), main.rs threading cfg.database_url.as_deref / cfg.clickhouse_url.as_deref, rubix/mani.yaml demo task ss -tlnp pre-flight for 127.0.0.1:8088) is already in tree on codeless/rubix-smoke-followups
- cargo test -p rubix-agent green: 17 lib tests + integration suites (mcp_stdio en-US + es-AR, rest_disk en-US + es-AR, json_flag, etc.) all pass; 2 ignored cases are the documented testcontainers/live-Postgres opt-ins
- ./rubix/scripts/lint-doc-refs.sh clean

## Next

- stage 3: Group D / next chunk of follow-ups per WORKFLOW.md (B9 CH database routing, B10 stale handover volume names, N4 dead-code warning, alert-path integration test)

## What you need to know

- Group C was NOT a fresh uncommitted diff this stage — it was already bundled into commit b72f238 "migration of ui theme to starter" (predates the smoke-followups branch). Working tree was clean on entry; nothing new to commit and nothing to push. The stage's requested commit title `fix(rubix-agent) drive migrations from cfg + port pre-flight in demo` was therefore not authored; renaming/splitting b72f238 would be a destructive history rewrite not authorized by the stage prompt
- Manual `mani run demo` until mcp_tools=6 / migrations_skipped=false was NOT executed: requires Docker + the dev Postgres/ClickHouse compose stack + the operator's `claude` CLI on PATH, none of which are available inside this isolated worktree. The asserted behavior is covered by the rubix-agent integration tests that did pass
- The dead_code warning on starter-auth-users `SUPER_ADMIN_TENANT` is the N4 follow-up flagged for stage 3
- Branch HEAD unchanged at 448ec73; no `git push` performed (nothing to push)

## Open questions

- Should b72f238 be split / re-authored to carry the requested `fix(rubix-agent) drive migrations from cfg + port pre-flight in demo` message, or is the existing landing of Group C acceptable and we move on to the next stage?
