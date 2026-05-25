# Mobile — local store (connections to remote agents)

The mobile app is a **multi-instance client**: one operator phone
talks to many rubix-agent servers (home lab, site A, site B,
demo, …). The local store owns that connection list, the
per-connection auth token, and the "which one is active right now"
selection. Nothing else.

**Engine:** SQLite via [`expo-sqlite`](https://docs.expo.dev/versions/latest/sdk/sqlite/).
Already part of the Expo SDK; no native module config beyond the
default plugin entry.

> SQLite is the right size for this. The data is small (tens of
> rows), relational (connection ↔ token ↔ last-used dashboard),
> and needs cheap transactional updates. AsyncStorage was floated
> in [APP-SHELL.md](./APP-SHELL.md#storage) — it's still used for
> theme + locale, but **not** for connections.

## What the local store owns

| Concern | Why local |
|---|---|
| Saved connections (server URL, label, colour, last-seen version) | The whole point of a multi-instance client. |
| Per-connection auth token | Bound to a specific server; can't live on any one of them. |
| Active connection id | Which server "this app" is currently pointed at. |
| Per-connection last-opened dashboard | Resume where the operator left off when switching sites. |
| Per-connection cache hints (etag, last-sync) | Reduce cold-start latency; never authoritative. |

## What the local store does NOT own

| Concern | Where it lives |
|---|---|
| Dashboard definitions | The remote agent — fetched via `useSduiResolve`. |
| User identity / profile | The remote agent — `/api/v1/auth/me`. |
| Theme + layout preferences | AsyncStorage via zustand `persist` (see [APP-SHELL.md](./APP-SHELL.md#storage)). |
| Locale | Same. |
| Tenants, teams, authz state | The remote agent. |
| React-Query cache | In-memory; persistence is a follow-up. |

The split is: **AsyncStorage holds preferences (the app's own
state); SQLite holds the multi-instance ledger.** Mixing the two
makes the connection list awkward to query and turns preferences
into per-server overrides we don't want.

## Schema

One migration file per change; numbered, monotonic, never
edited after merge. Lives at
`rubix/mobile/src/local-db/migrations/`. One responsibility per
file per [FILE-LAYOUT.md §4](../../../FILE-LAYOUT.md).

### `0001_connections.sql`

```sql
CREATE TABLE connection (
  id              TEXT PRIMARY KEY,            -- ulid
  label           TEXT NOT NULL,               -- "Home lab", "Site A"
  base_url        TEXT NOT NULL,               -- https://rubix.example.com
  colour          TEXT NOT NULL DEFAULT '',    -- hex, optional UI tag
  created_at      INTEGER NOT NULL,            -- unix ms
  last_seen_at    INTEGER,                     -- unix ms, null until first probe
  agent_version   TEXT,                        -- from /healthz, optional
  notes           TEXT NOT NULL DEFAULT ''
);
```

**No uniqueness constraint on `base_url`.** Operators legitimately
add the same URL twice with different labels (one rubix-agent
behind two tunnels, dev/prod toggle, single hostname serving two
logical tenants). The `connection/create.ts` verb warns at the
app layer if a duplicate `base_url` exists but doesn't reject.

### `0002_active_connection.sql`

```sql
CREATE TABLE app_state (
  k TEXT PRIMARY KEY,
  v TEXT NOT NULL
);
-- seeded row: ('active_connection_id', '')
```

`app_state` is a one-row-per-key bag for app-wide singletons. Keep
it tiny; once it grows beyond five keys, promote each to its own
table.

### `0003_per_connection_state.sql`

```sql
CREATE TABLE connection_state (
  connection_id        TEXT PRIMARY KEY
                       REFERENCES connection(id) ON DELETE CASCADE,
  last_opened_page_ref TEXT,
  last_synced_at       INTEGER
);
```

### Deferred (not in v1)

- **`auth_token` table.** Bearer tokens live in `expo-secure-store`
  (platform Keychain / Keystore) keyed by `connection_id`, **not**
  in SQLite. See [Secret handling](#secret-handling).
- **TLS pinning column.** When mobile gains self-signed support
  (today a [non-goal](./NON-GOALS.md#technical)), add a migration
  with `tls_pinned_fingerprint TEXT` on `connection`.
- **Token expiry metadata.** If tracked separately from the token
  itself, lives next to it in `expo-secure-store` as a JSON
  envelope `{ token, issued_at, expires_at }`. SQLite stays
  secret-free.

## File layout

`rubix/mobile/src/local-db/` mirrors the verb-per-file pattern
from [FILE-LAYOUT.md §2](../../../FILE-LAYOUT.md#2-the-verb-per-file-pattern):

```
src/local-db/
  open.ts                  ← opens expo-sqlite db, runs migrations
  migrations/
    index.ts               ← ordered list of migration sources
    0001_connections.sql
    0002_active_connection.sql
    0003_per_connection_state.sql
  connection/
    list.ts                ← list connections
    get.ts                 ← get by id
    create.ts              ← add a new server; warns on duplicate base_url
    update.ts              ← rename / recolour / re-base-url
    delete.ts              ← cascade clears state; also deletes secure-store token
    set-active.ts          ← write active_connection_id
    active.ts              ← read active connection (one row)
  token/                   ← thin wrapper over expo-secure-store
    get.ts                 ← read token for connection id
    put.ts                 ← write token after login
    clear.ts               ← logout
  state/
    last-page.ts           ← read/write last_opened_page_ref
    last-sync.ts           ← read/write last_synced_at
  errors.ts                ← local-db error types
```

`open.ts` is the only file that touches `expo-sqlite` directly;
every verb file takes a `db` argument so verbs are unit-testable
against an in-memory database. `token/*` files take a `secureStore`
argument so they're unit-testable against an in-memory mock.

## Secret handling

Bearer tokens live in **`expo-secure-store`** — the platform
Keychain on iOS and Keystore on Android — keyed by
`rubix.token.<connection_id>`. SQLite stores no secrets.

**Why direct, not AES-GCM-in-SQLite:** the alternative scheme
(AES key in secure-store, ciphertext in SQLite) shares the same
trust boundary as the token directly in secure-store, requires a
third-party native crypto module (Hermes has no `crypto.subtle`),
and contradicts the Expo-managed promise of
[ADR 0004 §Alternatives considered](../../adr/0004-react-native-mobile-app.md#alternatives-considered)
where the full rejection lives. This doc records the decision;
the long-form rebuttal stays in the ADR.

**Threat model covered:** offline SQLite read by another app or
backup extractor reveals the connection list but not the tokens.
**Threat model NOT covered:** rooted/jailbroken device with
keychain access — explicitly out of scope.

> **SQLCipher** (whole-file encryption via `op-sqlite`) remains an
> escape hatch if a security review later demands defence-in-depth.
> Deferred until then; it would require ejecting Expo managed for
> a custom native module.

## Health probe

`connection.last_seen_at` and `connection.agent_version` are
written in exactly three places, all via
`src/local-db/connection/touch.ts`:

1. **On add** (`connection/create.ts` after a successful
   `GET <base_url>/healthz`).
2. **On dashboard mount** (`<SduiPage>` boot path — piggybacks on
   the first `/api/v1/ui/resolve` response; the agent already
   returns version in the response headers).
3. **On manual refresh** of the connections screen
   (`connections/index.tsx` pull-to-refresh).

No background poll in v1 — "is this site up?" indicators on the
connections list show the value from the last natural touch with
a human-readable relative age ("seen 4m ago"). Background polling
is a follow-up once we understand drain on cellular.

## How the rest of the app sees it

> **Promotion note:** on promotion to
> `docs/design/mobile/`, this section splits out into its own
> `connection-provider.md` (sibling of `app-shell.md` and
> `local-db.md`). The `ConnectionProvider` design is a separate
> concern from the SQLite schema; co-locating them here is a
> scope-doc convenience, not a design intent.

The local store is **invisible to React-Query, SDUI, and the
clients** — they only see the *active connection's* `StarterClient`
+ `RubixClient`. The `ConnectionProvider` (one file,
`rubix/mobile/src/connection/provider.tsx`) is what bridges them:

```
ConnectionProvider
  ├─ reads active connection id from app_state
  ├─ reads its row from connection
  ├─ reads its token from expo-secure-store
  ├─ constructs StarterClient with { baseUrl, authHeader }
  ├─ constructs RubixClient(starterClient)
  ├─ publishes the active connection id so starterQueryKey
  │    namespaces every React-Query cache key by it
  └─ provides both clients via context
```

Switching server = `setActive(id)` → provider re-derives clients
→ the namespaced query keys change → active hooks refetch under
the new prefix; stale entries from server A are simply unreachable
and GC'd on the normal React-Query `gcTime`. A plain
`queryClient.clear()` was rejected because it aborts in-flight
queries and causes visible flicker.

See [APP-SHELL.md §Provider stack](./APP-SHELL.md#provider-stack)
for where this provider sits in the boot sequence.

## Screens this enables (out of scope for THIN-SLICE)

These ship after [THIN-SLICE.md](./THIN-SLICE.md) lands and the
local-db is in place. Each is one screen file under
`rubix/mobile/src/navigation/`:

- `connections/index.tsx` — list saved servers, tap to activate,
  swipe to delete.
- `connections/new.tsx` — add a server (URL + label + optional
  colour), probe `/healthz`, then push to login.
- `connections/[id].tsx` — edit metadata, force re-login, view
  agent version + last-seen.

The login screen in [APP-SHELL.md](./APP-SHELL.md) is
**per-connection**: after adding a server you're routed straight
into its login. Bearer-token auth (already mandated by
[ADR 0004](../../adr/0004-react-native-mobile-app.md#consequences))
makes this clean — one token per connection, stored in
`expo-secure-store` (see [Secret handling](#secret-handling)).

## Backup, export, sync

Not in v1. The local DB is the source of truth and lives only on
the device. Two follow-ups, each its own ADR when needed:

- **Export / import** a connection list as a JSON file (without
  tokens) so an operator can seed a second phone.
- **iCloud / Google Drive backup** of the encrypted DB. Requires
  the SQLCipher path above; defer.

## Migration discipline

Same rules as the backend
([docs/design/migrations/](../../design/migrations/)): one SQL
file per migration, monotonic numbering, never edit a merged
migration, always add a new one. `open.ts` runs them in order on
boot inside a single transaction and records the version in
`PRAGMA user_version`.

## Testing

- Each verb file has a sibling test (`connection/create.test.ts`)
  running against an in-memory `expo-sqlite` opened by a shared
  fixture.
- One e2e (Maestro — see
  [THIN-SLICE Block 5](./THIN-SLICE.md#block-5--dashboardspageidtsx--the-slice-itself))
  covers the round-trip: add server →
  login → render dashboard → switch to a second server → render
  its dashboard → delete the first server → confirm its token is
  gone.

## Promotion

This file promotes to `docs/design/mobile/local-db.md` once
`src/local-db/` is on `master` with the schema above and the
provider wired in. Until then it lives here, in `docs/scope/`,
and is **not** referenced from source code.
