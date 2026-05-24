## Done

- Verified working tree clean — Group A work for stages 1–3 is already committed as part of master commit `b72f238` ("migration of ui theme to starter"), which the branch's ancestry already includes. No uncommitted diff exists in this worktree to split.
- Confirmed the Group A artifacts are present on disk: `rubix/crates/rubix-agent/src/boot/mcp/{mod,agent_node,prefs,register}.rs`, the provider-aware `RunnerInput` dispatch in `crates/starter-ai-agent/src/agent_loop.rs`, the in-flight tracker + `slow_node_body_does_not_race_quiescence` test in `crates/starter-flow/src/run.rs`, the LONG-TERM.md CLI dispatch note, and the `mcp_stdio_test.rs` reply canary.
- Ran `cargo test -p starter-flow -p starter-ai-agent -p rubix-agent --lib --tests`: starter-flow 97/97 green (including `slow_node_body_does_not_race_quiescence`), starter-ai-agent and rubix-agent suites green.
- Ran `./rubix/scripts/lint-doc-refs.sh` — clean.
- Pushed `codeless/rubix-smoke-followups` to origin (new remote branch, set up tracking).

## Next

- Stage 2: Group B (observability + MCP error chain). Same situation likely applies — those files are already in `b72f238`. The next session should confirm and either accept the de-facto landing or, if a clean 3-/multi-commit history is genuinely required, propose a strategy that does **not** rewrite shared master history (e.g. revert b72f238 on the branch and re-land in pieces, or just proceed to stages 5–7 which are net-new work).

## What you need to know

- The job's premise — "uncommitted post-#30 smoke-test work in the working tree" — was already false when this worktree was created: master commit `b72f238` (parent = `0511981`, the PR #30 merge) bundled all of Group A + Group B + Group C smoke-test follow-ups together with an unrelated UI-theme migration. The branch `codeless/rubix-smoke-followups` was forked from master *after* `b72f238`, so the work is already in its ancestry.
- Splitting `b72f238` into the three chronological commits the SCOPE asks for would require force-pushing a rewritten master, which SCOPE explicitly forbids (`No --no-verify, no --force push`). Recommend treating stages 1–3 as already-landed and focusing remaining effort on stages 5–7 (B9 CH routing, B10 stale volume names, N4 dead-code, alert-path test) which are net-new.
- One unrelated pre-existing failure: `starter-flow --test workspace_dep_tree_gates :: starter_flow_spi_baseline_holds` — a baseline snapshot test failing because of `serde_json v1.0.149 → v1.0.150` drift in `Cargo.lock`. Not introduced by Stage 1 work; will need a separate baseline refresh.
- Doctests for starter-flow / starter-ai-agent / rubix-agent abort with "extern location does not exist" errors when run in this worktree (rustdoc dep resolution glitch unrelated to source). The `--lib --tests` invocation works fine.

## Open questions

- Should the next session attempt to rewrite the branch to split `b72f238` (requires re-confirming with the operator since it touches shared history), or accept stages 1–3 as already-landed via master and move directly to stages 5–7?
