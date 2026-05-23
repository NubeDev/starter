# WAREHOUSE — ClickHouse marts, L1→L3, tenancy

> Cites: SCOPE [R6](../../SCOPE.md#r6), [Phase 4 entry gate Q5](../../SCOPE.md).

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

## Tag types are `Bool | Str` only (R6)

Locked at Phase 1 in `rubix-spi` slot schemas. Adding a third tag
type (Int, Float, Json) is a **breaking change** per R9 and bumps
the major across spi + client + binary.

Reason: warehouse ingestion has to be able to fan a single typed
column out across millions of rows without runtime type discovery.

## Tenancy decision (Q5 — Phase 4 entry gate)

Three options:

| Option | How | Pros | Cons |
|---|---|---|---|
| Per-tenant tables | `events_<tenant_uuid>` | strong isolation, easy retention | table sprawl, schema migrations × N |
| Per-row tenant column | `tenant_id UUID` on every table | one schema, easy joins | every query must filter; leak risk |
| Separate ClickHouse DBs | one DB per tenant | strong isolation, separate auth | infra cost, cross-tenant analytics impossible |

**Default pending verification:** per-row tenant column with a
mandatory authz filter at the resolver layer (cheapest, matches
how `starter-authz` already gates Postgres queries). Q5 must be
resolved here before Phase 4 code lands.

## Retention

Each layer has its own retention policy set via
`rubix.clickhouse.retention_set`:

- **L1**: days–weeks (raw, expensive).
- **L2**: months (curated, indexed).
- **L3**: years (mart, narrow, cheap).

Rule: **never set L1 retention longer than L2.** The
`clickhouse-ruler` skill enforces this.

## Mart authoring

Marts are flow rules + the `rubix.clickhouse.mart_create` tool.
A mart that crosses tenants without the authz filter fails CI
(test pending Phase 4).

## Upstream candidates

- `clickhouse-query` node kind in `starter-flow-nodes` if any
  rubix flow YAML calls ClickHouse directly. See
  [STARTER-CHANGES.md](./STARTER-CHANGES.md).
- `starter-tool-clickhouse` — the rule.write / mart.create /
  retention.set tools (any consumer with CH benefits).
