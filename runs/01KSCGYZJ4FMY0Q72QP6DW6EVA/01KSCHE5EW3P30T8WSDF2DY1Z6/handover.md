## Done

- Ran `git fetch origin master` + `git log --oneline -20`; confirmed goals-2-4-3 merged as PR #32 (merge commit `c1186f9`)
- Audited `rubix/crates/rubix-agent/src/` for Axum handlers vs `#[utoipa::path]` attributes
- Confirmed `utoipa` is not yet a dep in `rubix/crates/rubix-agent/Cargo.toml` and zero `utoipa::path` attrs exist crate-wide
- Verified goals-2-4-3 added zero new in-crate Axum handlers (all new goal verbs ride the existing `POST /api/v1/tools/{tool_id}` dispatcher as `Tool` impls in `rubix-tools`)

## Next

- B.2 (next stage): add `utoipa` workspace dep to `rubix/crates/rubix-agent/Cargo.toml`, add `#[utoipa::path(...)]` to the two handlers below, and create `rubix/crates/rubix-agent/src/openapi.rs` assembling the `OpenApi` document — single coherent commit per SCOPE §B
- Handlers needing annotation in B.2:
- `rubix/crates/rubix-agent/src/routes/tools.rs::dispatch` → `POST /api/v1/tools/{tool_id}` (tags: dispatcher; request body `serde_json::Value`; response either tool result `Value` or `{error: string}`; 200/400/401/403/404/409/500)
- `rubix/crates/rubix-agent/src/health.rs::healthz` → `GET /healthz` (tag: meta; 200 returns `{"status":"ok"}` as `text/plain`-shaped JSON literal)
- `rubix/crates/rubix-agent/src/routes/auth.rs` is a pure re-export of `starter_auth_users::routes::auth_router`; its handlers live upstream and are out of scope for this crate's annotation pass

## What you need to know

- No commit produced this stage. WORKFLOW.md §B step 2 explicitly forbids combining the inventory and implementation: "B.1 inventories what's missing. B.2 implements them. Don't combine". The stage prompt's "otherwise commits one feat(rubix-agent)" clause was overridden by this more specific job directive. The inventory above IS the B.1 deliverable.
- The two handlers identified are the entire in-crate REST surface of `rubix-agent` post-#32. There is no hidden router merge elsewhere — `src/main.rs:35` is `async fn main`, not a handler, and `src/health.rs::serve` is the bind/listen helper, not an HTTP handler.
- `rubix-spi` DTOs already carry `utoipa::ToSchema` (per SCOPE.md line 140), so B.2 only needs to wire path-level attrs + assemble the document; schema discovery is already in place.

## Open questions

- Stage-prompt vs WORKFLOW.md conflict on whether B.1 commits the attrs itself. Resolved here in favour of WORKFLOW.md; flag for operator if they prefer the other reading, in which case re-run this stage with explicit instruction to combine B.1+B.2.
