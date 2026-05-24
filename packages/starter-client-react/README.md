# `@nube/starter-client-react`

React bindings for [`@nube/starter-client-ts`](../starter-client-ts).
Three things, no endpoint code:

1. **Providers** — `StarterClientProvider`, `QueryProvider`,
   `AuthProvider`. Mount once near the root.
2. **`useAuth()`** — the `me` query, `login`, `logout`, and a slot to
   render when no session exists.
3. **`useEventStream()`** — a `useSyncExternalStore` bridge over
   `streamJson` from `starter-client-ts`, with stable identity and
   reconnect.

Endpoint-shaped hooks (users, extensions, system, …) live in sibling
typed packages such as
[`@nube/rubix-client-react`](../../rubix/packages/rubix-client-react).
Keeping this package transport-only is what lets every future
rubix/starter frontend reuse it without copy-paste.

## Provider hierarchy

The three providers nest in a fixed order — outer providers feed inner
ones, never the other way around.

```tsx
import {
  StarterClientProvider,
  QueryProvider,
  AuthProvider,
} from "@nube/starter-client-react";

<QueryProvider>
  <StarterClientProvider client={starter}>
    <AuthProvider unauthenticatedSlot={<LoginRoute />}>
      <RouterProvider router={router} />
    </AuthProvider>
  </StarterClientProvider>
</QueryProvider>
```

Why this order:

- `QueryProvider` is outermost so every descendant — including
  `AuthProvider`, which runs its own `me` query — shares one
  `QueryClient`.
- `StarterClientProvider` carries the transport. `AuthProvider`
  reaches for it via `useStarterClient()`, so it must be inside.
- `AuthProvider` rendering the unauthenticated slot is the layout-level
  auth guard. Routes below it can assume `useAuth().user` is non-null.

A sibling typed package (e.g. `RubixClientProvider`) usually mounts
*inside* `StarterClientProvider` and *outside* `AuthProvider`. See the
[rubix-client-react README](../../rubix/packages/rubix-client-react/README.md)
for the full stack.

## Hook patterns

### `useAuth()`

```tsx
const { user, login, logout, isAuthenticated } = useAuth();
```

- `user` — current `User | null`. Driven by the `me` query keyed by
  `ME_QUERY_KEY` (exported for manual invalidation).
- `login({ email, password })` — mutates, then invalidates `me`.
- `logout()` — calls `/auth/logout`, then resets the cache so no
  stale data survives the session boundary.
- The provider renders `unauthenticatedSlot` itself when the `me`
  query returns 401; descendants never see the unauthenticated tree.

### `useEventStream(factory, opts?)`

```tsx
const { data, status, error, reconnect } = useEventStream(
  () => streamExtensionEvents({ signal }),
  { enabled: isAuthenticated }
);
```

- `factory` returns an `AsyncIterable<T>` (the stream from
  `streamJson` or a wrapper around it). The factory is called once
  per subscription; reconnect re-invokes it.
- `status` is `connecting | open | reconnecting | closed | error`.
- `reconnect` has stable identity — safe to drop into a deps array.
- The hook tears down via the supplied `AbortController` on unmount.

### Query-key convention

This package owns one root key for the `me` query:

```
['starter', 'auth', 'me']    // exported as ME_QUERY_KEY
```

Typed sibling packages own their own roots (e.g. `['rubix', ...]`).
Host apps invalidate by prefix — `['starter']` for auth, `['rubix']`
for the whole rubix surface.

## Defaults that surprise people

`QueryProvider` ships these defaults so consumers don't have to think
about them on day one:

| Option       | Value     |
|--------------|-----------|
| `staleTime`  | 30s       |
| `gcTime`     | 5min      |
| `retry`      | 3 attempts, but `0` on `401` / `403` |

Override by passing a custom `client` prop if your app needs
different numbers.
