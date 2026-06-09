# WS-09 — Production Hardening (Login Fix · Cache · Quotas · Audit · Rate-Limit · HA · OTel)

> **Status:** Not started · **Wave:** 0 (login fix — NOW) + Wave 1 (cache/audit/limit) + Wave 3 (HA/OTel)
> **Owner:** _unassigned_ · **Depends on:** C1+C3 for cache key shape · **Migration:** block `12xx` (e.g. `1201_query_cache_meta.sql`, `1202_quotas.sql`; audit log → WS-12)
> **Read first:** GAP_ANALYSIS §2.9, ROADMAP §0, `docs/session/backend/TODO-FOR.UI.md`, NEXUS.md §5.3/§11
> **Verified:** `82a6a19a` on 2026-06-09 — login-hang location re-verified (it's in `starter-auth-users`, not nexus — §P0). Re-grep before building (ROADMAP §0).

## Goal
Make Nexus safe to run at scale. The user wants "scale and into production asap" — today there's no
result cache (the DB melts under refresh load), no rate limiting (one tenant can starve others), no
audit trail, no query history, no multi-node live, and a **known login-hang bug**. Close the
operational gaps in priority order.

## Current state (evidence)
- ✅ `/health`, `/metrics` (Prometheus), structured `tracing` logs (`main.rs`, `serve.rs`).
- ✅ Query guards (read-only/timeout/row+byte caps) and per-`(tenant,datasource)` pool cache
  (`datasource_pools.rs`).
- ❌ **No query-result cache** — every panel hits the DB live (a 20-panel dashboard @10s refresh ≈
  120 QPS *per viewer*). No `cache`/`redis`/`memo` anywhere.
- ❌ **No rate limiting**, **no per-tenant quotas/concurrency caps**.
- ❌ **No query history.** *(Audit log is also absent, but it now lives in [WS-12](./WS-12_AUDIT_AND_UNDO.md) — audit + undo share one changelog substrate.)*
- ❌ **No OpenTelemetry** (logs only; no trace propagation).
- ❌ **Single-node only**: FlowManager + alert scheduler single-node by design; **in-process SSE
  broadcast can't span nodes** (NEXUS.md §5.3 — needs a shared bus to scale live fan-out).
- 🐞 **Login hang**: argon2 `password::verify` runs **synchronously on the async runtime** (no
  `spawn_blocking`), starving the executor under concurrent logins (`TODO-FOR.UI.md`).
  **Verified location (2026-06-09):** the synchronous call is in **`starter-auth-users`**, not nexus
  — `crates/starter-auth-users/src/routes/login.rs:110` and `.../src/token/verify.rs:33`, via
  `.../src/password/mod.rs` (argon2id). nexus does not own this code.

## Scope (in priority order)
### P0 — Login-hang fix (do immediately; it's a live bug, Wave 0)
- **It's an upstream/shared-crate fix, not a nexus fix.** The root cause is CPU-bound argon2 verify on
  the async runtime in `starter-auth-users` (`routes/login.rs:110`, `token/verify.rs:33`). The fix:
  wrap the verify/hash in `tokio::task::spawn_blocking` (or a small dedicated blocking pool) inside
  `password/{verify,hash}.rs` so it can't starve the executor.
- **Ownership/coordination:** this lives in a shared crate used by **every** app on it — it needs an
  upstream PR + sign-off, not a nexus-local patch. Raising the nexus metadata pool size is a
  *symptom* mitigation only (more connections doesn't un-block a starved runtime); do the
  `spawn_blocking` fix, and treat any pool bump as a separate, secondary tuning knob.
- Add a **concurrent-login load test** (in `starter-auth-users`) that would have caught it.
- ⚠️ Re-verify the file:line citations before starting — shared-crate code drifts (see §0 freshness note).

### P1 — Query result cache
> ⚠️ **Ordering hazard (peer-review #5): this cache ships in Wave 1, but WS-11 units land in Wave 2.**
> If the key omits the units/locale dimension now and WS-11 turns conversion on later, the cache
> **silently serves cross-unit-poisoned entries** until someone remembers to bump the key. **Fix:
> bake the full C3 cache-key tuple from day one** — including a `units_locale_tz` field that is a
> **constant placeholder** until WS-11 populates it. Cheap now (one extra key field); prevents a
> nasty correctness bug later. This is a hard requirement, not a nice-to-have.
- In-process LRU (e.g. `moka`) keyed by the **full C3 tuple**: `tenant + datasource + query_id
  (interpolated-sql | kind+bound-params) + resolved-time + variable-values + units_locale_tz`
  (coordinate the exact shape in Wave 0 — ROADMAP §6 C3). Short TTL aligned to the refresh interval;
  bypass for `refresh=off`/explicit "run". Optional Redis backend behind the same interface for
  multi-node. Emit hit/miss metrics. `1201_query_cache_meta.sql` (WS-09 `12xx` block) only if metadata is persisted.
- **Adopt the rubix `.cache.yaml` sidecar spec ([WS-10](./WS-10_KINDS_EXTENSIBILITY.md) §2/§4) as the
  declarative cache config** rather than inventing one: per-kind `ttl`, `scope: user|tenant`,
  `invalidate_on.tables: [...]`, and (later) the `time_series` bucket decomposition (closed buckets
  cache long, the open tail short). A write to a declared table drops the matching cached entries. The
  `tables:` list on a kind drives invalidation. This is a *designed, battle-tested* format — port it.
  Single-flight/coalescing (one backing load per key; the rest wait) is the rubix-proven companion —
  include it to kill the thundering-herd that motivated this whole row.
- **Two-layer scope for units/locale ([WS-11](./WS-11_UNITS_AND_PREFS.md)).** Once values are
  converted server-side per user prefs, the cache MUST account for it or two users with different
  units share a wrong entry. Mirror rubix: cache the **canonical** query result at **tenant** scope
  (the DB hit), and the **converted/rendered** output at **user** scope — one DB load serves the
  whole tenant; per-user conversion is paid once per TTL. The user-scope key includes the resolved
  `{units, locale, timezone}`.

### P1 — Per-tenant quotas & concurrency
- Cap concurrent queries per tenant/user and a fan-out limiter so one dashboard's N panels can't
  exhaust the pool (NEXUS.md §5.2 names this). Reject/queue past the cap with a clear error.

### P1 — Rate limiting
- Per-tenant/user/IP request limits (token bucket) as middleware. Sensible defaults, tenant override.

### P1 — Audit log → **moved to [WS-12](./WS-12_AUDIT_AND_UNDO.md)**
The audit log is no longer built here. Audit and undo/redo share **one append-only changelog**
(`starter_changes`) — the repo already has the substrate (`starter-changelog-postgres` +
`starter-undo`), so building a separate `audit_log` table would duplicate it. WS-12 owns the
changelog-on-PG, the `record_if_reversible` recording convention (C6), `GET /api/v1/audit`, and the
audit UI. **What stays in WS-09:** make sure WS-09's own privileged actions (rate-limit overrides,
quota changes, cache purges) emit a `ChangeDraft` via the C6 convention so they show up in the audit
log. (Migration `0011` is freed; audit tables are `0016/0017` under WS-12.)

### P2 — Query history (overlaps WS-03)
- If WS-03 hasn't shipped it, the table + endpoint live here; otherwise coordinate to avoid dupes.

### P2 — OpenTelemetry
- OTel traces (tracing-opentelemetry) with request/trace IDs propagated through query + alert paths;
  export to OTLP. Keep the existing logs/metrics.

### P3 — Multi-node / HA
- **SSE shared bus** (NATS or Redis pub/sub) so a live stream on node A reaches a subscriber on node
  B — lifts the single-node live constraint (NEXUS.md §5.3). Registry key stays
  (spec+datasource+tenant+perm).
- **Alert scheduler** already uses `FOR UPDATE SKIP LOCKED` (multi-node-safe claim) — verify and
  document the multi-replica story; FlowManager needs a "which node runs this flow" story (leader or
  partitioned ownership).
- Stream lifecycle hardening (NEXUS.md §5.3): heartbeat/keepalive, `Lagged` slow-subscriber policy,
  reconnect + `Last-Event-ID` resume, teardown-on-deploy (today live panels die silently on restart).

## Design notes
- **Cache correctness is everything** — the key MUST include resolved time + variable values, or a
  shared time picker serves stale/cross-context rows. Snap `now` to the refresh tick (WS-01) so the
  key is stable within a tick. This is why P1 cache waits on C1/C3.
- **Don't loosen query guards** — quotas/limits sit *in front of* the existing read-only/timeout/cap
  governance, never replace it.
- **HA is genuinely larger** — the in-process broadcast is a deliberate v1 boundary (NEXUS.md §5.3).
  Treat P3 as its own sub-project; P0–P1 deliver most of the "won't fall over in prod" value.

## Acceptance criteria
- [ ] **P0:** concurrent logins no longer hang; argon2 off the async runtime; regression test added.
- [ ] **P1:** repeated identical panel queries within a refresh tick hit the cache (metrics prove it);
  changing time/variables busts it correctly.
- [ ] **P1:** the cache key carries a `units_locale_tz` field **from day one** (constant pre-WS-11);
  a test asserts two callers with different (placeholder→real) units never share a converted entry —
  so enabling WS-11 later cannot serve poisoned cache.
- [ ] **P1:** a tenant exceeding its concurrency/rate cap is throttled without affecting other tenants.
- [ ] **P1:** WS-09's own privileged actions (rate-limit/quota/cache-purge) emit a `ChangeDraft` so
  they appear in the WS-12 audit log. *(The audit log itself is acceptance-tested in WS-12.)*
- [ ] **P2:** traces span an end-to-end query with a propagated trace ID.
- [ ] **P3:** a live panel served from node A reaches a subscriber on node B via the shared bus;
  reconnect resumes via `Last-Event-ID`.
- [ ] Tests mirrored; load-ish test shows the cache cuts DB QPS under refresh.

## Out of scope (hand off)
- Cache *key inputs* (time/vars) are defined by WS-01/02/03 — this WS consumes them.
- Backup/restore is Postgres-native ops, not app code (document expectations, don't build).
