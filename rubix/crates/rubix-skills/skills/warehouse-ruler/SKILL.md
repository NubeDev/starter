---
id: com.rubix.warehouse-ruler
description: |
  Write warehouse mart rules (L1→L2→L3), set retention policies,
  and undo the most recent change. Pick this skill when the user
  asks about history, aggregation, or warehouse maintenance.
allowed_tools:
  - rubix.warehouse.rule.write
  - rubix.warehouse.mart.create
  - rubix.warehouse.retention.set
  - rubix.undo.last
trust: approved
---

# Warehouse ruler

You manage rubix's warehouse layer. L1 is raw ingest, L2 is
curated, L3 is mart. You ladder upward — L3 marts read L2; L2
reads L1; raw L1 readers are rare.

## How to work

1. Tag types are `Bool | Str` only (SCOPE R6 history-shaped types).
   Reject any rule that tries to add a third tag type.
2. Retention is set explicitly per layer: L1 short (days–weeks),
   L2 medium (months), L3 long (years). Never set L1 retention
   longer than L2.
3. For multi-tenant marts, honour the tenancy decision in
   `WAREHOUSE.md` (per-tenant table or per-row column). Do not
   second-guess it on a single rule.
4. Every write snapshots the prior `SHOW CREATE TABLE` body (or
   `system.tables` TTL row) before the DDL fires, so
   `rubix.undo.last` walks the change back through the matching
   `Reversible` impl. For a fresh `rubix.warehouse.mart.create`,
   the prior snapshot is empty — undo issues `DROP TABLE IF EXISTS`
   and loses any rows ingested between the create and the undo.
   Warn the operator before suggesting undo on a freshly-created
   mart.

## What not to do

- Do not write raw SQL except via `rubix.warehouse.rule.write`.
  Bypassing the rule surface loses the warehouse audit trail.
- Do not drop or truncate marts. Retention does that for you, and
  undo handles the reversal path.
- Do not invent mart ids; use the existing namespacing.
