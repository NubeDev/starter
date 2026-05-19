# starter-auth-token

Single-owner `Authenticator`. Designed for headless / appliance
deployments where there is exactly one operator and no concept of
multi-user login.

**Mutually exclusive** with `starter-auth-users` — consumer picks one.

## Lifecycle

1. First boot: server generates a 32-byte base64url `claim_token` in
   `starter_auth_token_pending`. Surface it via logs or a sibling
   `SecretStore` entry at key `auth-token:pending`.
2. Operator hits `POST /auth/claim` carrying the token. Server
   consumes pending, generates a fresh `owner_token`, stores its
   SHA-256 digest, returns the plaintext exactly once.
3. Every request must present `Authorization: Bearer <owner_token>`.
   `TokenAuthenticator` constant-time compares against the digest and
   yields `Principal { subject: claim_id, role: Admin, scopes: [] }`.
4. `regenerate_claim_pending(store)` wipes claimed + pending and bumps
   the auth epoch.

## Usage

```rust
use starter_auth_token::{
    regenerate_claim_pending, store::SqliteClaimStore, routes::claim_router,
    TokenAuthenticator,
};

let store = SqliteClaimStore::new(pool.clone());
let pending = regenerate_claim_pending(&store).await?;  // first boot
let auth = TokenAuthenticator::new(SqliteClaimStore::new(pool));

let router = claim_router::<AppState>(std::sync::Arc::new(store));
```

See [`examples/minimal`](../../examples/minimal) for the full wiring.

## Features

- `sqlite` — ships `SqliteClaimStore` over `starter-store-sqlite::Pool`.
- `postgres` — ships `PostgresClaimStore` over
  `starter-store-postgres::Pool`.

Migrations live under `migrations/starter_auth_token/`; register them
via the namespaced runner.

## Secrets integration

`regenerate_claim_pending_with_secrets(store, secrets)` additionally
writes the pending plaintext to `SecretStore` at key
`auth-token:pending` (constant `PENDING_SECRET_KEY`).
