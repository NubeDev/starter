# @nube/starter-ui-core

React glue for starter. Three pieces:

- `<AuthProvider>` + `useAuth()` with pluggable strategies (session
  cookie, bearer token, external IdP). Hook surface is identical
  across strategies so app code doesn't branch.
- `starterQueryKey(...)` — every starter-owned react-query key is
  namespaced under `['starter', ...]`.
- (`testing/` deferred — msw + RTL helpers will land with the first
  consumer test that needs them.)

## Install

```bash
pnpm add @nube/starter-ui-core @nube/starter-client-ts @tanstack/react-query
```

## Usage

```tsx
import { StarterClient } from "@nube/starter-client-ts";
import {
  AuthProvider,
  sessionStrategy,
  useAuth,
} from "@nube/starter-ui-core";

const client = new StarterClient({ baseUrl: "/" });

export function App() {
  return (
    <AuthProvider client={client} strategy={sessionStrategy}>
      <Page />
    </AuthProvider>
  );
}

function Page() {
  const { status, user, login, logout } = useAuth();
  if (status === "loading") return <p>Loading…</p>;
  if (status === "unauthenticated") {
    return (
      <button
        onClick={() =>
          login({ kind: "credentials", email: "me@example.com", password: "..." })
        }
      >
        Log in
      </button>
    );
  }
  return (
    <>
      <p>Hello {user!.email}</p>
      <button onClick={logout}>Log out</button>
    </>
  );
}
```

## Strategies

- `sessionStrategy` — POST `/auth/login`, browser session cookie.
- `tokenStrategy({ onTokenChange? })` — bearer token held in memory;
  attached to `client.headers["Authorization"]`.
- `externalStrategy({ loginUrl, logoutUrl })` — redirects to an IdP.

## Query keys

```ts
import { useQuery } from "@tanstack/react-query";
import { starterQueryKey } from "@nube/starter-ui-core";

useQuery({
  queryKey: starterQueryKey("auth", "me"),
  queryFn: () => client.me(),
});
```

Use `isStarterQueryKey(key)` to invalidate starter-owned keys without
touching consumer-owned keys.
