# Starter changes — the upstream PR ledger (index)

This doc is how rubix's [R2](../../SCOPE.md#r2--upstream-first-rubix-specific-stays-in-rubix-reusable-goes-to-starter)
("upstream first") becomes a deliverable, not a slogan. Every
starter capability rubix needs that doesn't yet exist is listed
here, ordered by which rubix phase blocks on it.

The ledger is split per phase. This README is the canonical entry
point; each phase file is the source of truth for the items gating
that phase.

## How this doc is used

- **Before a phase starts:** read the phase file linked below.
  Each item that isn't merged yet must have a draft PR or a filed
  issue with rationale.
- **During a phase:** when rubix code starts to look like a
  re-implementation of a starter capability, file the upstream
  issue *first*, link it into the phase file, then either wait for
  review or ship a temporary rubix impl with the issue link in a
  `TODO(upstream: <issue>)` comment.
- **At phase exit:** the phase file lists every upstream PR filed
  during the phase (merged, in review, or filed-with-rationale).
  A phase with zero PRs is a smell — the reviewer asks "what
  didn't get upstreamed and why?"

This ledger lives in `rubix/` because it is rubix's planning
artifact. The actual code changes ship in starter. Linking is by
PR / issue URL.

## Format for each item

```
### <short title>
- **Crate(s):** starter-foo, starter-bar
- **Blocks rubix phase:** N
- **Why upstream:** one sentence on who else benefits
- **Status:** planned | issue-filed (#NNN) | pr-open (#NNN) | merged (vX.Y.Z)
- **Notes:** any rationale, alternatives considered, or rubix
  fallback if the PR slips
```

## Phases

| Phase | File | Summary |
|---|---|---|
| Phase 1 | [phase-1.md](./phase-1.md) | i18n render API, `DiagnosticParam::Quantity`, `render_diagnostic`, timezone-aware timestamps, `starter-tool-sysdiag`, recorded-LLM harness, flow-node-loop, skills parser, MCP prompts/resources, typed agent events. |
| Phase 2a | [phase-2a.md](./phase-2a.md) | `starter-auth-users` Postgres store impls. **Complete.** |
| Phase 2b | [phase-2b.md](./phase-2b.md) | `starter-mcp` Accept-Language plumbing (U1), real `InMemoryTransport` (U2), `starter-flow-surfaces` `FlowRegistry::resolve` + `FlowAsTool::from_registry` (U3). **All complete.** |
| Phase 2c | [phase-2c.md](./phase-2c.md) | gRPC/CLI rough edges; `starter-i18n` interpolate feature-gate mismatch (latent). |
| Phase 3 | [phase-3.md](./phase-3.md) | `starter-tool-sdui` page-builder primitives, `starter-tool-flow-ops`. |
| Phase 4 | [phase-4.md](./phase-4.md) | `cron-schedule` node kind, `starter-tool-clickhouse`, `clickhouse-query` node kind. |
| Phase 5 | [phase-5.md](./phase-5.md) | `starter-ext-flow` adapter, extension-author ergonomics. |

## Items filed (rolling log)

When a PR or issue is opened against starter on rubix's behalf,
append it here with date + link:

```
- YYYY-MM-DD  [#NNN](https://github.com/.../pull/NNN)  short title
```

(Empty until Phase 1 starts.)
