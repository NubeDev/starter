# ADR 0002 — Backend only (no frontend in this tree)

**Status:** superseded by [ADR 0004](./0004-react-native-mobile-app.md), 2026-05-25.
  The `rubix/frontend/` SPA was added before this ADR's superseding ADR was written; ADR 0004 records the formal reversal and extends the policy to mobile.
**Cites:** [SCOPE one-line summary](../../SCOPE.md), Non-goals

## Decision

The `rubix/` tree ships **no frontend code**. No Studio, no React, no
Tauri, no mobile. A future UI is a *client* of this backend and
lives elsewhere.

## Context

- Past rubix iterations bundled frontend (`rubix-old/`) and the
  scope sprawled — `ui-kit`, `ui-core`, `extension-ui-sdk`, Studio
  shell, mobile admin, Spanish catalogues — none of which had
  shipped before the backend was usable.
- The original SCOPE explicitly framed rubix as a backend product
  ("the agent backs the dashboards / users / flows / ClickHouse
  / jobs / analytics goals; the UI is a separate consumer").
- Frontend rot is hard to reverse: once `ui-core` exists,
  every backend change is "and update the hook." Removing
  frontend forces backend correctness first.

## Consequences

- Rubix consumes zero `starter-ui-*` packages.
- The MCP surface (R12) becomes the canonical UX surface for v0
  — Claude Desktop is the test client.
- A future UI lives in a separate repo / package and depends on
  `rubix-client` + the OpenAPI snapshot.

## Alternatives considered

- **Ship a minimal Studio with the backend.** Splits attention;
  inevitably grows. Rejected.
- **Ship a TS client only (no UI).** Already covered by future
  codegen from `rubix-spi`'s OpenAPI snapshot; doesn't justify a
  TS package living in `rubix/`.
