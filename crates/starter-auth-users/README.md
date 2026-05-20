# starter-auth-users

Multi-user `Authenticator`: two credential paths, one `Principal`.
**Mutually exclusive** with `starter-auth-token`.

- Browser → cookie sessions (`/auth/login`, `/auth/logout`,
  `/auth/me`). Passwords hashed with argon2id; sessions backed by
  `starter_auth_users_sessions`; CSRF via double-submit cookie.
- Machine → API tokens (`Authorization: Bearer …`). Token format
  `sak_<public_id>.<secret>` — secret half is argon2id-hashed at rest,
  public id is the table key (O(1) lookup).

`AuthAuthenticator` routes by string prefix (`sak_` → token,
`sas_` → session) so `with_principal` doesn't need to know.

## Usage

```rust
use std::sync::Arc;
use starter_auth_users::{
    routes::{auth_router, AuthState},
    store::{SqliteUserStore, SqliteSessionStore, SqliteTokenStore},
    AuthAuthenticator,
};

let users    = Arc::new(SqliteUserStore::new(pool.clone()));
let sessions = Arc::new(SqliteSessionStore::new(pool.clone()));
let tokens   = Arc::new(SqliteTokenStore::new(pool));
let state    = Arc::new(AuthState::new(users.clone(), sessions.clone(), tokens.clone()));
let auth     = Arc::new(AuthAuthenticator::new(users, sessions, tokens));

let router = auth_router(state);
```

## Features

- `sqlite` — sqlx-backed store impls over `starter-store-sqlite`.
- `postgres` — sqlx-backed store impls over `starter-store-postgres`.

Three migrations under `migrations/starter_auth_users/`: users,
sessions, tokens.

## Bootstrap

`admin::create_admin(store, email, password, role)` creates the first-
run user. CLI wiring is consumer-owned — see
[`crates/starter-cli/src/commands/admin_create.rs`](../starter-cli/src/commands/admin_create.rs)
for the recommended shape.

## OpenAPI

`openapi::openapi()` returns the canonical `utoipa::OpenApi` document
for the `/auth/*` surface (login + logout + me + their DTOs). Locked
to the workspace-root `openapi.json` snapshot; see
`tests/openapi_snapshot.rs`.

## Account-page settings (frontend, Phase 4)

The `/account/settings` route is consumer-mounted. The
`@nube/starter-ui-core/i18n` package ships a ready-made `<SettingsPage />`
bound to `<PreferencesProvider>` — drop it into your account surface:

```tsx
import { SettingsPage } from "@nube/starter-ui-core/i18n";

// router
<Route path="/account/settings" element={<SettingsPage onToast={pushToast} />} />
```

The Rust side (this crate) owns the auth shell; the React side owns
the form chrome. See `packages/starter-ui-core/src/preferences/SettingsPage.tsx`.
