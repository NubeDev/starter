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
   - parse the incoming SQL's table references (DataFusion sqlparser);
   - resolve each `ds_<alias>.<table>` (define and document the naming convention)
     to a tenant datasource;
   - register a DataFusion `TableProvider` per referenced table — first impls:
     postgres/timescale (fetch via the existing read-guarded connection path, predicate
     + projection pushdown where easy; correctness first), and file kinds (Parquet/CSV
     via `object_store` — these get DataFusion natively);
   - execute with the same caps/timeout/truncation contract QueryRunner enforces.
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
