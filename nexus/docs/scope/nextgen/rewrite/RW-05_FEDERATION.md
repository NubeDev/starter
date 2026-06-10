# RW-05 — Federation: DataFusion across datasources + file kinds

> Verified: 2026-06-10 against master (6b6f16d2). §0: re-grep every file:line below first.
> Depends on RW-04.

## Current state

- Query path is single-datasource push-down: user/kind SQL executes in the datasource's
  own DB (`nexus-api/src/routes/**` query routes → QueryRunner), guarded by read-only
  role, timeout, forced LIMIT, caps. This stays the hot path — do not slow it down.
- DataFusion is now a direct engine dep (RW-02). No cross-datasource join capability
  exists; file/object-store datasources don't exist.

## Scope

1. `nexus-engine/src/federation/` — a `FederatedQuery` runner:
   - table discovery via DataFusion's OWN catalog/planning resolution, NOT manual SQL
     scraping as the authority — register a custom `CatalogProvider`/`SchemaProvider`
     keyed by datasource alias so DataFusion resolves `ds_<alias>.<table>` during
     planning (manual parsing breaks on CTEs, quoted identifiers, schemas, aliases).
     The request carries an explicit alias→datasource-id map used for authz BEFORE
     planning; an alias resolving outside that map is an error;
   - register a DataFusion `TableProvider` per referenced table. FIRST ACTION of this
     step: evaluate `datafusion-table-providers` AND `datafusion-federation` TOGETHER
     (datafusion-contrib; the provider README describes federation-pushdown integration,
     so they're designed as a pair) — Postgres/MySQL/SQLite providers with
     predicate+projection pushdown already done.
     Adopt it if it supports our DataFusion major and our read-only-role/envelope-creds
     path can be threaded through its pool machinery; if rejected, READ its pushdown code
     before hand-writing one (remote pushdown has sharp edges: operator support tables,
     decimal/timestamp casts) and record the rejection reason in the session log.
     File kinds (Parquet/CSV via `object_store`) get DataFusion natively either way;
   - execute with the same caps/timeout/truncation contract QueryRunner enforces, AND a
     real input-side memory bound: caps cover output, but a join can pull two huge inputs
     before producing one capped row — configure DataFusion's `RuntimeEnv` with a
     `MemoryPool` limit (and a per-table fetch row limit where the provider allows).
2. Dispatch seam in the query route: one datasource referenced → existing push-down path
   untouched; multiple datasources OR a file kind → federation runner. The request shape
   stays backward compatible (single-`datasource` requests behave exactly as today);
   add the multi-source form to DTOs (DTO-first: openapi + codegen).
3. New datasource kinds in the WS-08b declarative pack: `parquet` / `csv`
   (path/object-store config, no creds or creds=object-store keys via envelope).
4. Tenancy: a federated query may only touch datasources the caller's tenant owns —
   enforce at resolution time, test the cross-tenant denial.

## Non-goals

Distributed execution, query result caching changes (WS-09's cache sits above and keys on
the request — verify the key covers the new fields, append if not), writeback via
federation.

## Acceptance

- E2E: join a docker-Postgres table against a Parquet file in one SQL statement → correct
  rows, capped, truncation flag honored.
- Single-datasource requests produce byte-identical responses to pre-RW-05 (fixture test).
- Cross-tenant datasource reference → error, no data leak (test).
- `GET /api/v1/datasources/kinds` lists the new file kinds.
