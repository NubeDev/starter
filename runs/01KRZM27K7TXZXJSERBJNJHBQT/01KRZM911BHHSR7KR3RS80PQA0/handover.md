## Done

- Populated the `## Decisions` section of `.codeless/jobs/starter-auth-oauth/SCOPE.md` with the four resolved open questions: callback rate-limiting deferred to `starter-server`; identity-change audit via `tracing::info!` now with shape `{provider, user_id, action ∈ {signup, link, unlink}, identity_id}` (durable sink deferred to `starter-observability::AuditSink`); email-change-as-security-event emitted at `tracing::warn!` with action `email_changed` and hashed old/new email (initial set stays at `info`); first-time signup consent screen skipped in v0.1 with `OAUTH_REQUIRE_ONBOARDING` env-var slot reserved (parsed, default false, no behaviour change).
- Each decision carries a concrete revisit trigger and an owner-on-revisit.
- Rewrote the former `## Open questions` section as a pointer to `## Decisions` so the lineage from bias → decision is traceable.
- Committed as `0a0a263` on branch `codeless/starter-auth-oauth`: "Stage 1 — resolve open questions in starter-auth-oauth SCOPE.md".

## Next

- Stage 2 (next session) per the staged plan in `.codeless/jobs/starter-auth-oauth/WORKFLOW.md`. This stage was a pure decisions-recording pass; no code yet.

## What you need to know

- The job brief references `DOCS/auth/scope/SCOPE.md` as source of truth, but that file does **not** exist in this worktree — only `DOCS/extensions/scope/SCOPE.md` is present. Decisions were therefore recorded in `.codeless/jobs/starter-auth-oauth/SCOPE.md` (the per-job copy referenced at the top of that file as `Source of truth: DOCS/auth/scope/SCOPE.md`). If a later stage materialises `DOCS/auth/scope/SCOPE.md`, the `## Decisions` block should be copied across verbatim — the per-job file says "when this file disagrees with the source-of-truth SCOPE, that doc wins".
- The `OAUTH_REQUIRE_ONBOARDING` flag is now a hard commitment for the config-parsing stage: it must be parsed and threaded through config in v0.1 even though it has no behaviour effect, so flipping it on later is a config bump not a schema bump.
- The audit-log decision picks `tracing::info!` with the fixed field shape above; the email-change event upgrades severity to `warn` and hashes (not raw) old/new email values to avoid leaking PII into general log aggregators. Implementors in later stages must keep that hashing discipline — the structured-shape-deferred caveat does not relax the no-raw-email-in-logs rule.
- The callback-rate-limiting decision means stage code does **not** add per-route limiting inside this crate; if a later stage feels tempted to add one, that is a signal to re-open the decision rather than work around it.

## Open questions

- (none) — all four stage-1 questions resolved; revisit triggers documented inline.
