# @nube/starter-ui-authz

Admin UI for the starter auth/authz stack — tenants, teams, members,
authz rules, role assignments, the resource registry, a dry-run
check tool, and the paged decisions audit feed.

Every panel is a thin React view over `@nube/starter-client-ts`
methods plus React Query, gated by the same Admin role + CSRF
double-submit that the backend already enforces. The package owns
no transport — mount it inside `<StarterClientProvider>` +
`<QueryProvider>` from `@nube/starter-client-react`.

## Install

```sh
pnpm add @nube/starter-ui-authz @nube/starter-client-ts @nube/starter-client-react @nube/starter-ui-kit @tanstack/react-query
```

`peerDependencies`: `@nube/starter-client-react`, `@nube/starter-client-ts`,
`@nube/starter-ui-kit`, `@tanstack/react-query`, React 18 or 19.

## Quick start

```tsx
import {
  QueryProvider,
  StarterClient,
  StarterClientProvider,
} from "@nube/starter-client-react";
import { AuthzAdmin } from "@nube/starter-ui-authz";

const client = new StarterClient({ baseUrl: "/api" });

export function AdminPage() {
  return (
    <StarterClientProvider client={client}>
      <QueryProvider>
        <AuthzAdmin />
      </QueryProvider>
    </StarterClientProvider>
  );
}
```

## What's in the box

| Surface | Backend route |
|---|---|
| `<TenantsPanel>` | `GET/POST /v1/tenants`, `PATCH /v1/tenants/{id}` |
| `<MembersPanel>` | `POST/PATCH/DELETE /v1/tenants/{id}/members[/{user}]` |
| `<TeamsPanel>` | `GET/POST/DELETE /v1/tenants/{id}/teams[/…]` + team members |
| `<RulesPanel>` | `/v1/authz/rules` CRUD |
| `<AssignmentsPanel>` | `/v1/authz/assignments` CRUD |
| `<ResourcesPanel>` | `GET /v1/authz/resources` |
| `<CheckPanel>` | `POST /v1/authz/check` (dry-run) |
| `<DecisionsPanel>` | `GET /v1/authz/decisions` (paged) |
| `<AuthzAdmin>` | All of the above in a tabbed shell. |

Mount panels individually for custom layouts:

```tsx
import { RulesPanel, AssignmentsPanel } from "@nube/starter-ui-authz/panels";
```

Hooks are exposed for hosts wanting to embed values elsewhere:

```tsx
import { useTenants, useAuthzDecisions } from "@nube/starter-ui-authz/hooks";
```

## i18n

The package is `react-intl`-free. Hosts derive `AuthzMessages`
from their own translation hook and pass it via
`<AuthzAdmin i18n={…}>` (or per-panel `i18n?: Partial<AuthzMessages>`).

```tsx
import {
  AuthzAdmin,
  DEFAULT_AUTHZ_MESSAGES,
  mergeAuthzMessages,
  type AuthzMessages,
} from "@nube/starter-ui-authz";

const fr: Partial<AuthzMessages> = {
  shell: { title: "Contrôle d’accès" /* … */ },
};

<AuthzAdmin i18n={fr} />;
```

For full localisation pass a wrapped `<AuthzI18nProvider value={...}>`
yourself and skip the prop.

## Server requirements

The package assumes the starter backend mounts
`tenants_router` (from `starter-auth-users`) and `authz_router`
(from `starter-authz`). Both are admin-gated server-side, so the
panels assume the operator already has an Admin session — the
package does not perform a client-side role check.

## Backend routes that are *not* wrapped

* User CRUD over REST — the backend exposes only `/auth/signup`
  + `/auth/me` + `/auth/login`; there is no global user list
  endpoint today. `<MembersPanel>` accepts a host-supplied
  membership list via the optional `members` prop so a host can
  back it with its own users API (see Rubix's
  `useUserList()`).

## License

MIT OR Apache-2.0.
