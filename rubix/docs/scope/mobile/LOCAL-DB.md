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

CREATE UNIQUE INDEX connection_base_url_unique ON connection(base_url);
```

### `0002_auth_token.sql`

```sql
CREATE TABLE auth_token (
  connection_id   TEXT PRIMARY KEY
                  REFERENCES connection(id) ON DELETE CASCADE,
  token           TEXT NOT NULL,               -- bearer
  issued_at       INTEGER NOT NULL,
  expires_at      INTEGER                       -- null = no known expiry
);
```

The token column is **not** raw on the device — see
[Secret handling](#secret-handling) below.

### `0003_active_connection.sql`

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

### `0004_per_connection_state.sql`

```sql
CREATE TABLE connection_state (
  connection_id        TEXT PRIMARY KEY
                       REFERENCES connection(id) ON DELETE CASCADE,
  last_opened_page_ref TEXT,
  last_synced_at       INTEGER
);
```

## File layout

`rubix/mobile/src/local-db/` mirrors the verb-per-file pattern
from [FILE-LAYOUT.md §2](../../../FILE-LAYOUT.md#2-the-verb-per-file-pattern):

```
src/local-db/
  open.ts                  ← opens expo-sqlite db, runs migrations
  migrations/
    index.ts               ← ordered list of migration sources
    0001_connections.sql
    0002_auth_token.sql
    0003_active_connection.sql
    0004_per_connection_state.sql
  connection/
    list.ts                ← list connections
    get.ts                 ← get by id
    create.ts              ← add a new server
    update.ts              ← rename / recolour / re-base-url
    delete.ts              ← cascade clears token + state
    set-active.ts          ← write active_connection_id
    active.ts              ← read active connection (one row)
  token/
    get.ts                 ← read token for connection
    put.ts                 ← write token after login
    clear.ts               ← logout
  state/
    last-page.ts           ← read/write last_opened_page_ref
    last-sync.ts           ← read/write last_synced_at
  errors.ts                ← local-db error types
```

`open.ts` is the only file that touches `expo-sqlite` directly;
every verb file takes a `db` argument so verbs are unit-testable
against an in-memory database.

## Secret handling

A bearer token in a plaintext SQLite file is fine for dev, weak
for prod. The token column stores a ciphertext; the key lives in
the platform keychain via `expo-secure-store`:

- First launch: `expo-secure-store` generates and stores a 32-byte
  random key under `rubix.localdb.key`.
- `token/put.ts` AES-GCM-encrypts the token with that key before
  insert; `token/get.ts` decrypts on read.
- If the key is missing (post-restore on a new device), tokens
  decrypt to null and the connection requires re-login. That's
  the correct outcome — auth tokens should not survive a device
  restore.

Encryption lives in `rubix/mobile/src/local-db/crypto.ts` (one
file, two functions: `encrypt`, `decrypt`). No `utils.ts`.

> An alternative is **SQLCipher** (whole-file encryption via
> `op-sqlite` or `@op-engineering/op-sqlite`). It's stronger but
> drags in a custom native module and breaks the "stays on Expo
> managed" promise from [ADR 0004](../../adr/0004-react-native-mobile-app.md#alternatives-considered).
> Defer until a security review demands it; the per-token AES-GCM
> approach is enough for v1.

## How the rest of the app sees it

The local store is **invisible to React-Query, SDUI, and the
clients** — they only see the *active connection's* `StarterClient`
+ `RubixClient`. The `ConnectionProvider` (one file,
`rubix/mobile/src/connection/provider.tsx`) is what bridges them:

```
ConnectionProvider
  ├─ reads active connection id from app_state
  ├─ reads its row from connection
  ├─ reads its token from auth_token (decrypts)
  ├─ constructs StarterClient with { baseUrl, authHeader }
  ├─ constructs RubixClient(starterClient)
  └─ provides both via context
```

Switching server = `setActive(id)` → provider re-derives clients
→ React-Query is reset (`queryClient.clear()`) so cached data
from server A never bleeds into server B. The reset is the
*entire* reason the clients aren't singletons in
[APP-SHELL.md](./APP-SHELL.md#provider-stack); update that file
to call out the swap-on-active-change semantics when this design
lands.

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
`auth_token`.

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
- One e2e (Detox or Maestro) covers the round-trip: add server →
  login → render dashboard → switch to a second server → render
  its dashboard → delete the first server → confirm its token is
  gone.

## Promotion

This file promotes to `docs/design/mobile/local-db.md` once
`src/local-db/` is on `master` with the schema above and the
provider wired in. Until then it lives here, in `docs/scope/`,
and is **not** referenced from source code.
