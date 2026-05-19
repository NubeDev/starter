## Done

- Reviewed the Stage 1 diff (prose-only edit to `.codeless/jobs/starter-auth-oauth/SCOPE.md` populating the `Decisions` section for the four resolved open questions) against the rulebook's Layer-1 invariants.
- Verdict: **PASS** on all four invariants — R1 (no Cargo.toml or code touched; the future `starter-observability::AuditSink` reference is future-tense and does not flip the dependency arrow), R2 (no new authenticator, session model, or cookie — tracing events are observability not transport), R4/R5 (audit events fire post-identity, the reserved `OAUTH_REQUIRE_ONBOARDING` 303-after-callback preserves "no DB write before identity is known", and the email-change event hashes old/new values), and wire-formats untouched (`OAUTH_REQUIRE_ONBOARDING` is parsed-but-inert in v0.1).
- Committed as empty marker `1cc4095` on `codeless/starter-auth-oauth`: `stage 2: REVIEW prerequisite decisions before any code lands — PASS` with the per-invariant rationale in the commit body.

## Next

- PASS: Layer-1 invariants (R1 dependency arrow, R2 single transport, R4/R5 trust boundary, wire-formats) all hold against Stage 1's prose-only Decisions edit; Stage 3 (Phase 1a — SemVer-breaking change to `starter-auth-users`: `LinkedProvidersLookup` trait + `NoLinkedProviders` default, `UserRecord.password_hash: Option<String>`, `UserStore::create` signature, `password_not_set` login error, plus migration `0002_users_password_optional` shipped in `starter-auth-oauth/migrations/`) may proceed in a fresh session.

## What you need to know

- The handover ends with a sentinel line the runtime parses (`PASS: …`) — see the dedicated line above this section.
- The empty commit is the cleanest record because Stage 1 was prose-only; a non-empty review-note file in `.codeless/jobs/` was considered but rejected as drive-by scope per WORKFLOW.md ("touch only what the stage names").
- The job brief references `DOCS/auth/scope/SCOPE.md` as source of truth, but that path does not exist in this worktree — the per-job copy at `.codeless/jobs/starter-auth-oauth/SCOPE.md` is the live document and is what Stage 1 edited. Stage 3 onward should keep editing that file; if a later stage materialises `DOCS/auth/scope/SCOPE.md`, the `Decisions` block must be copied across verbatim (the per-job file's preamble says "when this file disagrees with the source-of-truth SCOPE, that doc wins").
- Implementor reminders carried forward from Stage 1's handover that the gate did not relax: `OAUTH_REQUIRE_ONBOARDING` is a hard commitment for the config-parsing stage (parse + thread through config in v0.1, behaviour-inert), the email-change event must hash (never log raw) old/new emails, and the callback path must not grow per-route rate limiting inside this crate — if a later stage feels tempted, that is a signal to re-open the decision, not a workaround.

## Open questions

- (none) — Stage 2 is a clean PASS; no follow-up review items to carry into Stage 3.
