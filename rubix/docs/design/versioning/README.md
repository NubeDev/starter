# VERSIONING — R9 in full

> Cites: SCOPE [R9](../../SCOPE.md#r9).

## What is versioned

- `rubix-spi` — every public type, every REST DTO, every proto
  message, every MCP tool descriptor field.
- `rubix-client` public API.
- The rubix `block.yaml` contribution shape (when rubix needs new
  contribution kinds; usually inherits starter's shape).
- The six bundled flows' input/output DTOs (callers depend on
  them via MCP/REST).

## Add-only within a major

Within a major (`0.x.*`, `1.x.*`, ...), every change is
**additive**:

- New optional field on a DTO — OK.
- New variant on an enum that's read-only — OK (consumers serialise
  unknown variants as text).
- New tool — OK.
- New flow — OK.

Forbidden within a major:

- Removing a field.
- Renaming a field.
- Changing a field's type.
- Tightening an enum to a previously-unrepresentable value.
- Adding a required field (use Option + default).

## Breaking changes bump the major

A breaking change to **any** of the surfaces above is a major bump
on:

- The `rubix-spi` crate.
- The `rubix-client` crate.
- The `rubix-agent` binary.
- (Phase 5+) the `rubix-extensions-sdk` published version.

All four bump together. A `rubix-spi 2.x` agent does not run
`rubix-client 1.x` callers.

## Examples

| Change | Verdict |
|---|---|
| Add `disk.percent` to `DiskReadingDto` | additive (Option field) |
| Rename `flow.deploy` tool id | breaking |
| Add `enum ProgressKind { Determinate, Indeterminate }` variant `Throttled` | additive if `non_exhaustive` |
| Tag types broaden from `Bool \| Str` to `Bool \| Str \| Int` | **breaking** (R6 locked it) |
| Add a new optional MCP prompt | additive |

## How to ship a breaking change

1. Write the migration note in `docs/sessions/` describing what
   moved and why.
2. Bump the major on every crate listed above.
3. Update [STARTER-CHANGES.md](./STARTER-CHANGES.md) if the break
   was driven by an upstream starter change.
4. Land all the crate bumps in one commit; do not ship a
   `rubix-spi 2.0` without the matching `rubix-agent 2.0`.
