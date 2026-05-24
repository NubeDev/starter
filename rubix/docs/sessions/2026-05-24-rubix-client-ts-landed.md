# 2026-05-24 — `@nube/rubix-client-ts` landed

Session note for the `codeless/rubix-client-ts` branch. Promote to a
design doc once the PR merges (most of this is already in
[`docs/design/client-ts/README.md`](../design/client-ts/README.md)).

## New package

`packages/rubix-client-ts/` — a hand-curated TypeScript HTTP client
over rubix-agent's REST surface, mirroring the shape of
`@nube/starter-client-ts` and the Rust `rubix-client` crate.

- `RubixClient(starter: StarterClient)` — wraps a `StarterClient` and
  exposes it read-only as `.starter`.
- `RubixError extends StarterError` — adds `.code` parsed from
  `body.summary.code` (rubix Diagnostic envelope) with a fallback to
  the bare `body.code`.
- `src/generated/index.ts` — committed codegen output from
  `openapi-typescript`, driven by `rubix/openapi.json`.
- Nine endpoint files (one per goal area): `system`, `alert`, `user`,
  `team`, `tenant`, `clickhouse`, `flow_ops`, `undo`, `mcp`. Audit
  reads stay on `@nube/starter-client-ts` (SCOPE OQ-3).

## Upstream uplifts (`@nube/starter-client-ts`)

Common primitives moved upstream so both clients share them:

- `client/csrf.ts` — `readCsrfHeader(cookieName = "starter_csrf")`.
  Used by every mutating verb on both clients.
- `client/fetch_json.ts`, `fetch_void.ts`, `fetch_bytes.ts` — the
  shared URL-build + `credentials: "include"` + `res.ok` + error-throw
  helpers. Endpoint methods are now ~5 lines.
- `StarterError.is(err, status?)` — type guard for callers narrowing
  on a specific status.
- `packages/starter-client-ts/bin/codegen.mjs` — generalised codegen
  script taking `--input` and `--output`. Defaults preserve the
  existing `pnpm codegen` behaviour; `@nube/rubix-client-ts` calls it
  with `--input ../../rubix/openapi.json`.

## Test counts (final)

| Package | Tests | Files |
|---|---|---|
| `@nube/starter-client-ts` | typecheck green, vitest green | refactored 4 endpoint files |
| `@nube/rubix-client-ts`   | 33 passed (10 files)          | 9 endpoint files + 1 round-trip |

Last run (worktree HEAD = `120b8d0`):

```
Test Files  10 passed (10)
     Tests  33 passed (33)
```

Round-trip test (`tests/round-trip.test.ts`) exercises one method per
endpoint family against a `fetch`-mock; an operator may re-run it
against a live agent with `RUBIX_BASE_URL=... pnpm --filter
@nube/rubix-client-ts test`.

## Manual round-trip flow (operator)

```bash
# In one shell — boot the agent:
cargo run -p rubix-agent --bin rubix-agent

# In another — run a tiny consumer:
cat > /tmp/probe.mjs <<'EOF'
import { StarterClient } from "@nube/starter-client-ts";
import { RubixClient } from "@nube/rubix-client-ts";
const rubix = new RubixClient(new StarterClient({ baseUrl: "http://localhost:8080" }));
console.log(await rubix.disk());
await rubix.undoLast();
EOF
node --experimental-vm-modules /tmp/probe.mjs
```

## Drift CI

`.github/workflows/rubix-openapi-drift.yml` mirrors the existing
`openapi-drift` job:

1. Boots `rubix-agent` via `bash rubix/scripts/snapshot-openapi.sh`
   and `git diff --exit-code rubix/openapi.json`.
2. Re-runs `pnpm --filter @nube/rubix-client-ts run codegen` and
   `git diff --exit-code packages/rubix-client-ts/src/generated/`.

First green run will be the PR build on
`codeless/rubix-client-ts` → `master`.

## Commits by phase

- **A.1** `b9f4c4a` upstream-first dependency check (analysis only)
- **A.2** `778fb95` starter-client-ts extract csrf + fetch helpers
- **A.3** `ced477e` starter-client-ts StarterError.is + codegen generalisation
- **B.1** `473c438` utoipa::path attribute audit
- **B.2** `d628239` rubix-agent emit /openapi.json
- **B.3** `091ab5b` snapshot openapi.json + regen script
- **C.1** `e72e211` rubix-client-ts package scaffold + RubixClient + RubixError + generated types
- **C.2** `55f4493` rubix-client-ts system + alert + audit endpoints
- **C.3** `b3b926c` rubix-client-ts user + team + tenant endpoints
- **C.4** `b94d966` rubix-client-ts clickhouse + flow-ops + undo endpoints
- **C.5** `a1a697c` rubix-client-ts MCP endpoint + round-trip test
- **D**   (this commit) `chore(ci+docs)` close out rubix-client-ts + open PR
