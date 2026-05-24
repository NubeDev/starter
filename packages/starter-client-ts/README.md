# @nube/starter-client-ts

Thin TS HTTP client for starter-server. Wire types codegen'd from the
workspace-root `openapi.json` via `openapi-typescript`. Zero React.

## Install

```bash
pnpm add @nube/starter-client-ts
```

## Usage

```ts
import { StarterClient } from "@nube/starter-client-ts";

const client = new StarterClient({ baseUrl: "http://localhost:8080" });

// Cookie-session flow (the browser handles the session cookie):
await client.login({ email: "me@example.com", password: "..." });
const user = await client.me();
await client.logout();   // echoes the starter_csrf cookie as X-CSRF-Token
```

`logout()` reads the non-httpOnly `starter_csrf` cookie set by
`/auth/login` and echoes it as `X-CSRF-Token` automatically.

## Codegen

```bash
pnpm --filter @nube/starter-client-ts run codegen
```

Reads `../../openapi.json`, writes `./src/generated/index.ts`. The CI
`openapi-drift` job fails if the snapshot is stale.

`src/generated/` is **never** hand-edited. Endpoint files
(`src/endpoints/*.ts`) import types from there via
`components["schemas"]["..."]`.

## Error type

Methods throw `StarterError` (subclass of `Error`) on non-2xx. It
carries `status: number` and `problem: Problem | undefined` parsed
from the RFC 7807 body when present.

## Streaming

`streamJson<T>(starter, path, opts?): AsyncIterable<T>` is the
package's SSE primitive — one verb file, no React, no provider.
Endpoint packages wrap it to type the event shape:

```ts
import { streamJson } from "@nube/starter-client-ts";

for await (const evt of streamJson<MyEvent>(starter, "/api/v1/things/events", { signal })) {
  // ...
}
```

Behaviour:

- Uses `EventSource` (with `withCredentials: true`) when available,
  falls back to `fetch` + `ReadableStream` so it works in Node test
  runners. Either way the same async iterable is returned.
- Exponential reconnect — base 1s, cap 30s, 10% jitter. A synthetic
  `{ kind: "reconnecting" }` envelope is yielded so consumers can
  surface a UI state without parsing transport details.
- `opts.signal` aborts cleanly; the iterator returns and the
  underlying connection closes.
- CSRF is intentionally not threaded through. The browser sends the
  session cookie on the `EventSource` request — the same-site cookie
  flow is enough for read-only streams.

React consumers shouldn't iterate `streamJson` by hand. Use
`useEventStream()` from
[`@nube/starter-client-react`](../starter-client-react), which bridges
the iterable into `useSyncExternalStore` with stable reconnect
identity and lifecycle teardown.
