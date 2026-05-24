## Done

- B10: fixed stale docker volume names in rubix/docs/sessions/2026-05-24-handover-codeless-orchestration.md §2 (rubix-dev-* → docker_rubix_*_data)
- Audited §§5–8 of the handover; retired §8 Thread 1 which the pr30 smoke note has superseded. No other stale paths surfaced.
- N4: annotated crates/starter-auth-users/src/token/issue.rs::SUPER_ADMIN_TENANT with #[allow(dead_code)] + intent comment. git log -G shows it was introduced deliberately in slice 7a (commit 1624372) as the super-admin sentinel for R11; kept as named constant for downstream matchers.
- cargo build -p starter-auth-users → warning-free; lint-doc-refs clean.
- Committed as f87533c "chore(docs+auth-users) handover volume names + dead-code cleanup" and pushed to codeless/rubix-smoke-followups.

## Next

- Stage 7 (alert-path integration test) — last implementation stage before the PR-shape gate.

## What you need to know

- Branch tip: f87533c on codeless/rubix-smoke-followups.
- §5 paths to .codeless/jobs/rubix-demo-wiring, rubix-agent-runtime, flow-nodes all still resolve in the worktree, so no edits needed there.
- SUPER_ADMIN_TENANT was kept (not deleted) because it is pub, documented, and exists to give downstream crates a typed sentinel to match against — deletion would be a public-API regression.

## Open questions

- (none)
