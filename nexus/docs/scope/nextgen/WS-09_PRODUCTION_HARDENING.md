# WS-09 — Production Hardening (Login Fix · Cache · Quotas · Audit · Rate-Limit · HA · OTel)

> **Status:** Not started · **Wave:** 0 (login fix — NOW) + Wave 1 (cache/audit/limit) + Wave 3 (HA/OTel)
> **Owner:** _unassigned_ · **Depends on:** C1+C3 for cache key shape · **Migration:** `0011_audit_log.sql`, `0012_query_cache_meta.sql`
> **Read first:** GAP_ANALYSIS §2.9, `docs/session/backend/TODO-FOR.UI.md`, NEXUS.md §5.3/§11

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
- ❌ **No audit log table/API** (decrypts are *logged* but nothing is queryable), **no query history**.
- ❌ **No OpenTelemetry** (logs only; no trace propagation).
- ❌ **Single-node only**: FlowManager + alert scheduler single-node by design; **in-process SSE
  broadcast can't span nodes** (NEXUS.md §5.3 — needs a shared bus to scale live fan-out).
- 🐞 **Login hang**: argon2 `password::verify` runs **synchronously on the async runtime** (no
  `spawn_blocking`); suspected pool exhaustion on concurrent logins (`TODO-FOR.UI.md`).

## Scope (in priority order)
### P0 — Login-hang fix (do immediately; it's a live bug, Wave 0)
- Wrap argon2 verify/hash in `tokio::task::spawn_blocking` (or a dedicated rayon pool); raise the
  metadata pool size (default 10 → ≥20). Add a concurrent-login test that would have caught it.

### P1 — Query result cache
- In-process LRU (e.g. `moka`) keyed by **`tenant + datasource + interpolated-sql + resolved-time +
  variable-values`** (C1/C3 — coordinate key shape with WS-01/02/03). Short TTL aligned to the
  refresh interval; bypass for `refresh=off`/explicit "run". Optional Redis backend behind the same
  interface for multi-node. Emit hit/miss metrics. `0012_query_cache_meta.sql` only if metadata is
  persisted.

### P1 — Per-tenant quotas & concurrency
- Cap concurrent queries per tenant/user and a fan-out limiter so one dashboard's N panels can't
  exhaust the pool (NEXUS.md §5.2 names this). Reject/queue past the cap with a clear error.

### P1 — Rate limiting
- Per-tenant/user/IP request limits (token bucket) as middleware. Sensible defaults, tenant override.

### P1 — Audit log
- `0011_audit_log.sql`: `{ id, tenant_id, actor, action, resource_type, resource_id, detail, at }`,
  RLS-scoped, append-only. Record: datasource decrypt (already logged → also persist), grant
  changes, dashboard create/update/delete/share, datasource CRUD, login. `GET /api/v1/audit` +
  an admin audit view.

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
- [ ] **P1:** a tenant exceeding its concurrency/rate cap is throttled without affecting other tenants.
- [ ] **P1:** audit log records the listed actions; queryable, RLS-isolated, append-only.
- [ ] **P2:** traces span an end-to-end query with a propagated trace ID.
- [ ] **P3:** a live panel served from node A reaches a subscriber on node B via the shared bus;
  reconnect resumes via `Last-Event-ID`.
- [ ] Tests mirrored; load-ish test shows the cache cuts DB QPS under refresh.

## Out of scope (hand off)
- Cache *key inputs* (time/vars) are defined by WS-01/02/03 — this WS consumes them.
- Backup/restore is Postgres-native ops, not app code (document expectations, don't build).
