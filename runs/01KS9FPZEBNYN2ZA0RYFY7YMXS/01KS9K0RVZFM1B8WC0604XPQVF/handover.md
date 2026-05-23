## Done

- DecisionSink trait + DecisionEntry (split rule_id/reason per R14) + NoopDecisionSink default, in crates/starter-authz/src/audit/mod.rs
- DbDecisionSink with bounded mpsc (default depth 4096) + dedicated writer task, non-blocking try_send, drop+tracing::warn{dropped_count} on overflow, supports sqlite + postgres via internal Backend enum; in crates/starter-authz/src/audit/db.rs
- Deterministic hash sampling (per-process random seed + per-kind override map, audit_logs defaults to 1); should_sample_allow exposed pub for tests
- 0004_authz_decisions migration for sqlite + postgres with the four R14 indices (tenant_id+at, subject+at, effect+at, rule_id+at)
- audit::db::spawn_retention hourly task (batches of 10k, doc names the "table grows without bound" failure mode), plus retention_pass_once + list_via_sink helpers
- StaticRbacEngine grew sink field + with_sink/sink accessors; check() emits a DecisionEntry on every exit path (unknown_resource, no_tenant_binding, cross_tenant, condition_invalid, rule deny, rule allow, no_matching_rule). DbPolicyEngine propagates sink across reload().
- GET /v1/authz/decisions cursor-paginated (newest first, before=<rfc3339>, limit clamped to [1,500]); tenant-admins clamped to own tenant, super-admin sees all; route 404s when state.decision_sink is None
- DecisionSinkConfig::from_env / RetentionConfig::from_env honour STARTER_AUTHZ_DECISION_ALLOW_SAMPLE + STARTER_AUTHZ_DECISION_RETAIN_DAYS (sink switch via STARTER_AUTHZ_DECISION_SINK is documented; consumer-side wiring decides whether to construct DbDecisionSink, matching SCOPE-EXT's "you pay nothing if you don't enable it")
- 7 smoke tests in tests/decision_audit.rs all pass; full crates/starter-authz test suite green under --features sqlite; --features postgres compiles
- AuthzRoutesState grew decision_sink: Option<Arc<DbDecisionSink>>; legacy tests updated with decision_sink: None,
- Committed as f7c239c

## Next

- Stage 4 (slice 7d) — REST adapter permission field on ContributeRest.auth, with_permission auto-wrap in rest_router, RestBuildError::UnknownResource, examples/authz-demo simplification
- Stage 5 (slice 7d.2) — MCP / gRPC adapter parity for permission gating
- Stage 6 — final REVIEW gate after cross-tenant tests, then merge

## What you need to know

- DecisionSinkConfig.allow_sample is consulted by DbDecisionSink::record (denies always retained; sampling decided at sink boundary so check() is unchanged). Per-tenant override (tenants.audit_allow_sample column) is wired as a hook (sample_override parameter on should_sample_allow) but not resolved automatically — Stage 5 or a follow-up can plumb a TenantSampleResolver into the sink. The auth-users column from Stage 1 is still the source of truth.
- audit_logs kind is force-included via DecisionSinkConfig::new() default; the GET /v1/authz/decisions route check is therefore not sampled away (engine.check on that kind always persists). The route handler itself doesn't currently call engine.check — gating is admin-role only; if Stage 4 introduces with_permission on the decisions route, the audit_logs kind must be the one declared so the per-kind override fires.
- The DB sink uses Backend enum gated by feature flags; when both sqlite + postgres features are on, the match arms are exhaustive over the present variants. starter_store_sqlite::Pool / starter_store_postgres::Pool are both Clone (verified) so the writer task owns its own handle.
- spawn_retention skips the first tick (lets the binary boot before deleting); if a test wants immediate retention, call retention_pass_once directly (used in retention_task_deletes_expired).

## Open questions

- Per-tenant audit_allow_sample resolution is not yet wired — DbDecisionSink::record always passes None to should_sample_allow. Add a TenantSampleResolver trait + populate from starter-auth-users tenant_store in Stage 4 or as a follow-up; the column + override hook are in place.
- DbPolicyEngine::set_sink takes &mut self; production wiring typically holds DbPolicyEngine inside an Arc, so consumers should pass the sink at construction (via the new audit module) rather than mutating after the fact. Consider adding `DbPolicyEngine::new_with_sink` in Stage 4 if a builder shape is preferred.
