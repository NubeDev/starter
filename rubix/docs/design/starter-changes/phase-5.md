# Starter changes — Phase 5 gates

Extension framework gates.

See [README.md](./README.md) for the index and per-item format.

## `starter-ext-flow` — the flow/skill/tool/node adapter

- **Crate:** `starter-ext-flow` (new)
- **Blocks rubix phase:** 5
- **Why upstream:** required by starter's agent SCOPE for
  extensions to contribute `flows`, `skills`, `tools`, `nodes`. Not
  rubix-specific in any way.
- **Status:** planned (referenced in starter `DOCS/agent/SCOPE.md`
  but not yet in `starter-extensions/crates/`)
- **Notes:** rubix Phase 5 cannot ship without this. If the
  upstream PR slips, rubix Phase 5 slips — no fallback is
  acceptable (a rubix-only extension adapter would fork the
  starter extension framework).

## Extension-author ergonomics (10-minute scaffold)

- **Crate(s):** `starter-extensions/*`, `starter-ext-sdk`,
  possibly a new `starter-ext-scaffold` CLI helper.
- **Blocks rubix phase:** 5 (ergonomic exit criterion)
- **Why upstream:** the rubix Phase 5 exit criterion is "a fresh
  extension author scaffolds a new tool/skill/flow in ≤10
  minutes." Anything that makes this fail is a starter ergonomics
  gap, not a rubix one.
- **Status:** measure first, file second. The walkthrough
  surfaces the gaps; each gap becomes an upstream issue.
