# WAREHOUSE — ClickHouse marts, L1→L3, tenancy

The warehouse side of rubix is the ClickHouse half of the storage
split (Postgres owns dimensions; ClickHouse owns history). All
typed writes go through `starter-store-clickhouse::ChClient`; all
DDL is applied through `starter-store-clickhouse::MigrationRunner`,
which accepts rubix-owned files via `with_extra_migration` so a
single runner applies the union in declaration order.

## The layers

```
L1 raw          ingest from agents / extensions / external feeds
   ↓ rules (flow node + rubix.clickhouse.rule_write)
L2 curated      typed, deduplicated, tenanted
   ↓ mart rules
L3 marts        narrow tables answering specific questions
```

Reads happen at L3 by default. L2 reads are allowed; L1 reads are
rare and explicit.

`system_disk_history` (created by `rubix/0002_history/up.sql`) is
the first rubix-owned L1 table: one row per `rubix.system.disk`
probe, columns `tenant_id (UUID)`, `host (String)`,
`percent_used (UInt8)`, `free_bytes (UInt64)`, `epoch_ms (Int64)`,
partitioned by `toYYYYMM(toDateTime(epoch_ms / 1000))`, ordered by
`(tenant_id, host, epoch_ms)`.

## Tag types are `Bool | Str` only

Locked in `rubix-spi` slot schemas. Adding a third tag type (Int,
Float, Json) is a breaking change and bumps the major across spi
+ client + binary.

Reason: warehouse ingestion has to fan a single typed column out
across millions of rows without runtime type discovery.

## Tenancy is per-row `tenant_id`

The v0 choice is **per-row tenant column**. Every history table
carries a `tenant_id UUID` and every read goes through an authz
filter applied at the resolver layer (the same shape
`starter-authz` already enforces for Postgres queries). The in-
process insights test path passes `Uuid::nil()` as the sentinel
tenant id; production callers pass the principal's tenant from
the request context.

The other two options were considered and rejected for v0:

| Option | How | Why not now |
|---|---|---|
| Per-tenant tables | `events_<tenant_uuid>` | strong isolation, but table sprawl and `N×` schema migrations on every tenant onboarding |
| Separate ClickHouse DBs | one DB per tenant | strong isolation + separate auth, but infrastructure cost and cross-tenant analytics impossible |

Per-row wins because it is the cheapest path that still keeps
multi-tenant analytics in one query plane. The trade-off is that
every read query must filter — a missing filter is a leak — so the
resolver layer carries the only entry point to `system_disk_history`
and a lint keeps ad-hoc `SELECT`s out of code review.

**Revisit trigger:** a single tenant's read volume forces per-
tenant tables. The moment one tenant's history dominates the merge
budget, the v0 column becomes a partition discriminator for table-
per-tenant splits and this section gets a successor ADR.

## Retention

Each layer has its own retention policy set via
`rubix.clickhouse.retention_set`:

- **L1**: days–weeks (raw, expensive).
- **L2**: months (curated, indexed).
- **L3**: years (mart, narrow, cheap).

Rule: **never set L1 retention longer than L2.** The
`clickhouse-ruler` skill enforces this.

## Mart authoring

Marts are flow rules + the `rubix.clickhouse.mart_create` tool. A
mart that crosses tenants without the authz filter fails CI.

## Upstream candidates

- `clickhouse-query` node kind in `starter-flow-nodes` if any
  rubix flow YAML calls ClickHouse directly. See
  [STARTER-CHANGES.md](./STARTER-CHANGES.md).
- `starter-tool-clickhouse` — the rule.write / mart.create /
  retention.set tools (any consumer with CH benefits).
