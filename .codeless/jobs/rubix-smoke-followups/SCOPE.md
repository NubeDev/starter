# Scope — rubix-smoke-followups

## Goal

Land the uncommitted post-#30 smoke-test work as a clean stack of commits on `codeless/rubix-smoke-followups`, then close four small follow-ups the smoke surfaced (B9 ClickHouse database routing, B10 stale handover volume names, N4 dead-code warning, alert-path integration test). At the end of this job, the THIN-SLICE smoke is reproducible by a fresh operator from the published handover commands and passes 6 / 6 without an in-session workaround.

The work for stages 1–3 is **already implemented** on `master` as uncommitted changes in the working tree — see `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md` §"Re-run after fixes" for the full diff narrative, §"Engine-coordinator quiescence" for the starter-flow piece, and §"Files changed" for the canonical file list. The codeless job's value here is **discipline and sequencing**: review the diff per group, commit each group as its own coherent unit, push, then move to the next stage. Do not re-derive the implementation from scratch — it is tested, the rationale is documented, and starter-flow's 97 / 97 suite is green against it.

Stages 5–7 are additive follow-up work that the smoke surfaced; they get their own commits and ride the same branch.

## In scope

### Already-implemented work to review, commit, push (stages 1–3)

- **Group A — engine fix + rubix wiring.** Three logical commits in dependency order: `crates/starter-flow/src/run.rs` (`run_coordinator` grows `in_flight: HashSet<NodeId>` updated on `NodeStarted` / `NodeEmitted` / `NodeFailed`; the quiescence deadline arm of the `tokio::select!` carries `if in_flight.is_empty()` so a slow `invoke` cannot race the run to `RunCompleted`; inline test `slow_node_body_does_not_race_quiescence` sleeps 250 ms with 50 ms quiescence and asserts populated terminal slot), then `crates/starter-ai-agent/src/agent_loop.rs` (provider-aware `RunnerInput` dispatch — CLI providers get `RunnerInput::Cli(CliCfg)` with history folded into the prompt; REST providers keep `RestCfg` with tools + history) plus `crates/starter-ai-agent/LONG-TERM.md` new §"CLI runner tool dispatch (via MCP bridge)", then `rubix/crates/rubix-agent/src/boot/mcp/*` (split into `mod.rs` + `agent_node.rs` + `prefs.rs` + `register.rs`; per-node primary-tool dispatch from `allowed_tools[0]`; `RUBIX_AI_NARRATION` on by default with opt-out at `=0`; `ch_client` threaded through `build_mcp_surface` → `build_tool_registry` → `build_flow_registry`) + `rubix/crates/rubix-agent/src/main.rs` call-site updates and `ch_client` reordering + `rubix/crates/rubix-agent/src/bin/rubix_admin/mcp/serve.rs` (builds its own `ChClient` from `cfg.clickhouse_url` for stdio MCP) + `rubix/crates/rubix-agent/tests/mcp_stdio_test.rs` (asserts the real `{tool: {summary: {code, params}}, ...}` shape across en-US and es-AR plus the non-empty `reply` canary). Closes B8, B8.2, B12.
- **Group B — observability + error chain.** One commit: `crates/starter-observability/src/tracing/init.rs` (honor `RUST_LOG` over the argument when the env var is set and non-empty), `crates/starter-mcp/src/protocol/error.rs` (new `RpcError::internal_from_source(&dyn Error)` walks the `source()` chain into `message` joined with `: ` and `data.chain` as a JSON array), `crates/starter-mcp/src/server/dispatch.rs` (uses the new constructor and emits a `warn!` log line at the dispatch boundary). Closes B7a, B7b.
- **Group C — boot config refactor + demo port pre-flight.** One commit: `rubix/crates/rubix-agent/src/boot/migrations.rs` and `rubix/crates/rubix-agent/src/boot/clickhouse.rs` take `Option<&str>` parameters and the env reads are gone; `main.rs` passes `cfg.database_url.as_deref()` and `cfg.clickhouse_url.as_deref()`; `rubix/mani.yaml` `demo` task pre-flights `127.0.0.1:8088` with `ss -tlnp` and aborts friendly with a `pkill -f` hint. Closes B5, B6.

### New follow-up work to implement (stages 5–7)

- **B9 — ClickHouse database routing.** The `0002_history` migration writes to the `default` DB; the bootstrap creates a `rubix` DB that ends up empty. Pick one resolution per SCOPE Open Question 2 and apply it: either (a) update `0002_history` to `CREATE TABLE rubix.system_disk_history` and any related insert site so all warehouse tables live under the named tenant DB, or (b) drop the `rubix` DB creation from the bootstrap and document `default` as the canonical CH database. Default to (a) unless evidence in `git log` shows the bootstrap was intentionally pointing at `default`. Whichever choice: the integration test for history insertion passes after the change and `docs/design/warehouse/README.md` reflects reality present-tense.
- **B10 — stale volume names in handover.** Update `rubix/docs/sessions/2026-05-24-handover-codeless-orchestration.md` §2 so `docker volume rm` uses the live compose-default names `docker_rubix_postgres_data` / `docker_rubix_clickhouse_data` instead of the stale `rubix-dev-postgres-data` / `rubix-dev-clickhouse-data`. While in the doc, scan §§5–8 for any other stale paths the smoke surfaced and fix in the same commit.
- **N4 — `SUPER_ADMIN_TENANT` dead-code warning in `starter-auth-users`.** Either delete the constant (if `git log -G SUPER_ADMIN_TENANT` shows no intentional reservation), wire it into the bootstrap (if it was meant to seed the super-admin tenant id), or annotate with `#[allow(dead_code)]` plus a one-line comment explaining the future use. Default to deletion unless evidence suggests otherwise.
- **Alert-path integration test.** Step 5 of the smoke was PARTIAL because real disk usage was 83 % / 86 % — below the hardcoded 90 % threshold in `boot::insights`. Parameterise the threshold via `cfg.insights.disk_warn_threshold` defaulted to 90 in `agent.toml`, then add an integration test under `rubix/crates/rubix-agent/tests/` that sets the threshold to 50, fires the disk tool, and asserts the alert path fired (log capture or in-memory sink — pick the smaller move, justify in the test header). If even the parameterisation move is genuinely > 100 LoC of refactor, raise BLOCKED with a design sketch.

## Out of scope

- **No new tool stubs.** The 25 verb stubs in `rubix-tools` other than `system.disk` and `alert.send` stay stubbed — post-thin-slice work tracked in `docs/scope/GAPS.md`.
- **No extensions work (`com.rubix.example`).** Blocked on upstream `starter-ext-flow`.
- **No OAuth, no SDUI, no flow-programmer tool, no analytics, no user-admin tool, no clipboard, no undo.** Each has its own phase in SCOPE.
- **No live LLM in CI.** Recorded fixtures under `rubix/crates/rubix-agent/tests/fixtures/` remain the test seam.
- **No re-derivation of the already-implemented work in stages 1–3.** Review, commit, push. If review surfaces a real defect, raise it as a small fix on top of the existing diff with a one-paragraph justification in the commit message. Do not rewrite from scratch.
- **No `--no-verify`, no `--force` push** (only `--force-with-lease` if a rebase is needed, with operator confirmation). No phasing markers in code.

## Constraints

- **R1 — One verb per file.** ≤ 400 lines hard, ~100 typical. The `boot/mcp/` split into `mod.rs` / `agent_node.rs` / `prefs.rs` / `register.rs` already follows this; don't undo it. Any new file added in stages 5–7 must also obey.
- **R2 — Upstream-first.** Group A's starter-flow change lands before rubix-agent's narration-on default; Group B's starter-observability + starter-mcp changes land before they're depended on. Stages 1 → 2 → 3 order respects this.
- **R3 — Doc-tier rule.** Code comments link `docs/design/<area>/README.md` only — never `SCOPE.md`, `HOW-TO-CODE.md`, `NEW-SESSION.md`, `FILE-LAYOUT.md`, `docs/scope/`, or `docs/sessions/`. `./rubix/scripts/lint-doc-refs.sh` runs in every stage's `checks`.
- **R4 — Tool outputs are `Diagnostic` + structured data**, never pre-formatted strings. The alert-path test must follow this.
- **R5 — Catalogue files are the source of truth for MessageKeys.** Any new key needs an entry in both `rubix-spi/catalogues/en.json` and `rubix-spi/catalogues/es.json` in the same commit.
- **Tests live with the code in the same commit.** Group A's `slow_node_body_does_not_race_quiescence` and `mcp_stdio_test.rs` reply canary are already authored; verify they're in the diff before committing the stage.
- **Commit messages.** `feat(<crate>):` for Group A's primary functional changes, `fix(<crate>):` for Group B and C and B9 / B10 / N4, `test(rubix-agent):` for the alert test, `chore(docs+...):` for the docs+lint cleanup, `chore(docs):` for the closing docs. No `Co-Authored-By` line required.
- **PR titles match the lead commit.** Body summarises the bugs closed and links the smoke session note.

## Open questions

1. **Group A engine fix — upstream stance?** The `in_flight` tracker in `starter-flow::run_coordinator` is load-bearing for any consumer with a slow node body, not just rubix. Stage 1 must confirm the existing diff is the right shape for upstream (it should be — pure addition, no SPI change). The commit message must lead with that framing. If review finds the tracker breaks an existing starter-flow test or is implemented in a rubix-specific way, raise BLOCKED.
2. **B9 — (a) move tables to `rubix` DB, or (b) drop the `rubix` DB?** Default to (a) per the named-tenant intent in `SCOPE.md`. Stage 5 must first run `grep -rn 'CREATE DATABASE rubix\|USE rubix' crates/starter-store-clickhouse rubix/`; if the bootstrap clearly intended (a), do (a); if the bootstrap is incidental from a copy-paste, do (b). Document the choice in the migration file and `docs/design/warehouse/README.md`.
3. **Alert-path test — can the threshold be parameterised without > 100 LoC refactor?** If yes, do it (smaller move). If no, raise BLOCKED with the LoC estimate and a one-paragraph design sketch.
4. **PR shape at stage 4.** Four separate PRs off one branch is unusual (the second PR contains the first's commits). Default to one PR with a four-or-five-commit stacked history that the maintainer reviews commit-by-commit. Confirm with operator at stage 4's REVIEW gate before opening any PR.

## References

- `rubix/docs/sessions/2026-05-24-smoke-test-pr30.md` — source of truth for B5–B12, the implemented diff, the verification evidence.
- `rubix/docs/sessions/2026-05-24-handover-codeless-orchestration.md` — operator runbook; §4 gotchas, §5 canonical job flow.
- `rubix/docs/scope/THIN-SLICE.md` — the demo path. Smoke-test row in the success-criterion table flips to verified-on-<today> once stages 1–4 land (and again at stage 8 if the alert path now exercises).
- `rubix/SCOPE.md` — R1–R13 invariants.
- `rubix/FILE-LAYOUT.md` — Rule Zero in long form.
- `rubix/HOW-TO-CODE.md` — contributor entry point.
- `rubix/NEW-SESSION.md` — non-negotiables, doc-tier rules.
