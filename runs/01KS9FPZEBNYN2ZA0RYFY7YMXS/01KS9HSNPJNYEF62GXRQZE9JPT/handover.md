## Done

- Reviewed stage 1 (slice 7a) diff against master and the prior handover. Verified the cross-tenant-deny test (`tenant_predicate.rs::cross_tenant_request_is_denied_before_any_rule_evaluates`) exists with the role:`*` / resource:`*` / actions:[`*`] allow rule, and that the engine evaluates the tenant predicate before consulting rules. SPI additions (`Principal.tenant_id`, `ResourceRef.tenant`, `ResourceSpec.tenant_scoped`) are strictly additive and respect R1 crate direction. No transport added (R2 holds). Wire formats unchanged on REST/MCP/gRPC adapters in this slice.

## Next

- (none) — REVIEW gate fired FAIL; runtime halts the job.

## What you need to know

- Layer-1 invariants R1/R2/R4/R5 and wire-format stability are intact in the code that did land — the failure is scope completeness of slice 7a, not invariant violation.
- Concrete gaps confirmed by grep on the worktree: (1) no `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT` reference anywhere → backfill migration with env-gate not shipped; (2) no `select-tenant` route or hand-rolled HTML interstitial → OAuth callback still mints tenantless sessions per prior handover; (3) no `owner_id` immutability triggers in any `crates/*/migrations` directory → R12 not enforced on reports/flows/pages/marts/sandboxes; (4) Postgres-side immutability triggers for `starter-auth-users` tenant tables not shipped (crate is sqlite-only today); (5) smoke tests `multi-tenant-session-binding`, `oauth-callback-resolves-tenant-via-query-param`, `oauth-callback-renders-interstitial-on-multi-membership`, and Postgres `immutability-trigger-rejects-update` absent.
- The prior session's handover (`runs/01KS9FPZEBNYN2ZA0RYFY7YMXS/01KS9FRWP87Y5ERE80GA1DNW0Z/handover.md`) self-discloses every one of these gaps in its Open questions list and notes the disk-full event that truncated the work.
- PASS: <none>
- FAIL: stage 1 (slice 7a) shipped the engine predicate + tenants/memberships schema + admin REST but explicitly omitted the OAuth tenant-resolution interstitial, the R12 `(tenant_id, owner_id)` immutability triggers on built-in tenant-scoped tables, the `STARTER_AUTHZ_PHASE_7_DEFAULT_TENANT`-gated backfill migration, and the multi-tenant/OAuth/Postgres-trigger smoke tests required by the REVIEW gate.

## Open questions

- (none) — re-enter slice 7a in a follow-up WORK stage to close the five gaps above, then re-run this REVIEW.
