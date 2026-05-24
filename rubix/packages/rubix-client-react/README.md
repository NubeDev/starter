# @nube/rubix-client-react

React bindings for [`@nube/rubix-client-ts`](../rubix-client-ts). Provides:

- `RubixClientProvider` — mounts a sibling `StarterClientProvider` under the hood
  using the wrapped client's `.starter` instance, so hooks from both
  `@nube/starter-client-react` and this package resolve against the same
  long-lived transport.
- Typed hooks per rubix-agent endpoint family (system, users, mcp,
  extensions, teams, tenants, clickhouse, flow_ops, undo) plus an audit
  read hook against starter-server.

## Query-key convention

Every hook in this package writes its TanStack Query key under a single root
namespace so the host app can invalidate by prefix:

```
['rubix', <family>, ...<discriminator>]
```

Rules:

1. **Root** is always the literal string `'rubix'`. Lets the host invalidate
   *every* rubix query with `invalidateQueries({ queryKey: ['rubix'] })`.
2. **Family** is the second segment and matches the hook module name in
   `src/hooks/` — `'system'`, `'users'`, `'mcp'`, `'extensions'`, `'teams'`,
   `'tenants'`, `'clickhouse'`, `'flow_ops'`, `'undo'`, `'audit'`. Each family
   exports a `*_KEY` constant (`USERS_KEY`, `TEAMS_KEY`, …) you can spread
   to build longer keys.
3. **Discriminator** is appended after the family. Common patterns:
   - `'list'` for the read-all hook of the family (`['rubix','users','list']`).
   - A request scalar (`request.mount`, `request.dsn`) for filterable reads.
   - The full filter object for paged reads with non-trivial query strings
     (e.g. audit: `['rubix','audit','list', filter]`).
4. **Mutations** invalidate their family prefix on success — e.g.
   `useUserCreate` invalidates `['rubix','users']`, which catches every
   `['rubix','users', ...]` query. `useUndoLast` is the one exception: it
   invalidates the whole `['rubix']` root because an undo can ripple across
   every family.
5. **Note on `flow_ops`**: the family segment is the underscored form
   (`'flow_ops'`) to match the Rust tool id (`rubix.flow_ops.*`) — not
   `'flowOps'` — even though the hook module file is `flow-ops.ts`. Consistency
   with the wire contract wins over JS casing.

### Examples

```ts
import { useUserCreate, USERS_KEY } from "@nube/rubix-client-react";
import { useQueryClient } from "@tanstack/react-query";

// Read hook
const { data } = useUserList();           // key: ['rubix','users','list']

// Manual invalidation
const qc = useQueryClient();
await qc.invalidateQueries({ queryKey: USERS_KEY }); // hits all ['rubix','users', ...]

// Cross-family invalidation
await qc.invalidateQueries({ queryKey: ['rubix'] }); // hits every hook in the package
```

## Audit (`/v1/audit`)

The audit read route lives on starter-server, not rubix-agent, so it is not
part of the `@nube/rubix-client-ts` endpoint barrel (see SCOPE OQ-3).
`useAudit` calls `fetchJson` directly against the wrapped starter client. When
a typed `audit*` method lands on `@nube/starter-client-ts`, swapping the hook
body to call it is a one-line change at the call site.
