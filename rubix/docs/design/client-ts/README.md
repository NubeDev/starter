# `@nube/rubix-client-ts` — design

A thin TypeScript HTTP client over rubix-agent's REST surface, mirroring
the shape of the Rust `rubix-client` crate and the upstream
`@nube/starter-client-ts` package.

## What the client exposes

`RubixClient` is constructed from an existing `StarterClient` and reads
its transport configuration (baseUrl, fetch override, credentials,
default headers) through the `.starter` readonly field. Endpoint
modules hang verb methods off `RubixClient` via TypeScript
declaration-merging — one file per goal area, one method per verb:

| File | Methods | Goal |
|---|---|---|
| `endpoints/system.ts`     | `disk`, `db`, `flowErrors`              | system reads |
| `endpoints/alert.ts`      | `send`                                  | alerts |
| `endpoints/user.ts`       | `userCreate`, `userDisable`, `userList` | user admin (Goal 2) |
| `endpoints/team.ts`       | `teamCreate`, `teamAssign`              | team admin (Goal 2) |
| `endpoints/tenant.ts`     | `tenantList`                            | tenant admin (Goal 2) |
| `endpoints/clickhouse.ts` | `ruleWrite`, `martCreate`, `retentionSet` | clickhouse ruler (Goal 4) |
| `endpoints/flow_ops.ts`   | `flowDeploy`, `flowLint`, `flowList`, `flowDuplicate` | flow programmer (Goal 3) |
| `endpoints/undo.ts`       | `undoLast`                              | undo |
| `endpoints/mcp.ts`        | `mcpToolsList`, `mcpToolsCall`          | MCP |

Audit reads are intentionally absent — the `/v1/audit` route lives on
starter-server (see `crates/starter-audit/src/routes.rs`), so audit
belongs on `@nube/starter-client-ts`, not here.

## Relationship to `@nube/starter-client-ts`

`@nube/rubix-client-ts` is a *consumer* of `@nube/starter-client-ts`,
not a fork of it. Shared primitives live upstream and are imported:

- `StarterClient` — transport, baseUrl, fetch, credentials.
- `readCsrfHeader()` — reads the `starter_csrf` cookie and returns
  `{ "X-CSRF-Token": ... }`. Every mutating rubix verb threads this
  through its `headers`.
- `fetchJson`, `fetchVoid`, `fetchBytes` — the URL-build +
  `credentials: "include"` + `res.ok` + error-throw helpers. Endpoint
  methods stay ~5 lines because of this.
- `StarterError.is(err, status?)` — the type guard used by callers
  who want to narrow on a specific status code.

`RubixError` extends `StarterError` and adds `.code` parsed from the
rubix Diagnostic envelope (`body.summary.code`, with the bare
`body.code` legacy form as a fallback). `RubixError.fromResponse` is
the override that endpoint helpers use in place of
`StarterError.fromResponse`.

## Codegen flow

Wire types are generated from `rubix/openapi.json` — a *committed*
snapshot of the document rubix-agent serves at `GET /openapi.json`.
Codegen must not depend on a live agent process at build time.

```
rubix-agent (#[utoipa::path])
        │
        │  bash rubix/scripts/snapshot-openapi.sh
        ▼
rubix/openapi.json                 ← committed
        │
        │  pnpm --filter @nube/rubix-client-ts codegen
        │  (delegates to packages/starter-client-ts/bin/codegen.mjs
        │   --input ../../rubix/openapi.json
        │   --output ./src/generated/index.ts)
        ▼
packages/rubix-client-ts/src/generated/index.ts   ← committed
        │
        │  endpoint files import components["schemas"]["..."]
        ▼
packages/rubix-client-ts/src/endpoints/*.ts
```

Two CI gates enforce that the chain never drifts:

1. `rubix-openapi-drift` workflow re-runs `snapshot-openapi.sh` and
   `git diff --exit-code rubix/openapi.json`, then re-runs codegen
   and `git diff --exit-code packages/rubix-client-ts/src/generated/`.
2. The existing `openapi-drift` workflow does the same for the
   starter snapshot.

Regen instructions live in [`rubix/HOW-TO-CODE.md`](../../../HOW-TO-CODE.md)
§OpenAPI snapshot regen.

## Error type

```ts
import { RubixError } from "@nube/rubix-client-ts";

try {
  await rubix.diskInfo();
} catch (e) {
  if (RubixError.is(e, 503)) {
    // .code is e.g. "rubix.system.disk.unavailable"
    console.error(e.code, e.problem?.detail);
  }
  throw e;
}
```

`RubixError extends StarterError`, so `StarterError.is(e)` and
`e instanceof Error` both match. The `.problem` field carries the
RFC 7807 body when present; `.code` carries the rubix Diagnostic code.

## Worked example

```ts
import { StarterClient } from "@nube/starter-client-ts";
import { RubixClient } from "@nube/rubix-client-ts";

const starter = new StarterClient({ baseUrl: "http://localhost:8080" });
await starter.login({ email: "ap@nube-io.com", password: "..." });

const rubix = new RubixClient(starter);

// Read disk pressure on the agent host.
const disk = await rubix.disk();
console.log("free bytes", disk.freeBytes);

// Deploy a flow, then undo if the lint fails.
await rubix.flowDeploy({ id: "flow-42", graph: { /* ... */ } });
const lint = await rubix.flowLint({ id: "flow-42" });
if (lint.errors.length > 0) {
  await rubix.undoLast();
}

// Drive the agent via MCP with a per-request locale.
const tools = await rubix.mcpToolsList();
const out = await rubix.mcpToolsCall(
  "system.disk_summary",
  {},
  { acceptLanguage: "es-AR" },
);
console.log(out.structuredContent);
```

The manual round-trip flow (boot rubix-agent, run a tiny TS consumer,
verify the response shape) is documented in
`packages/rubix-client-ts/tests/round-trip.test.ts`. An operator runs
it against a live agent with `pnpm --filter @nube/rubix-client-ts test`
once `RUBIX_BASE_URL` is set.
