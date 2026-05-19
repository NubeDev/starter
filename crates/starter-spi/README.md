# starter-spi

Service Provider Interface — the contracts crate. Pure traits, error
types, and DTOs. Zero `starter-*` dependencies; every other crate in
the workspace depends on this one.

## What's inside

- `auth::` — `Authenticator` trait, `Principal { subject, role, scopes }`,
  `Role` (Reader / Writer / Admin), `Scope`.
- `secrets::` — `SecretStore` (sync; see the module docs for rationale).
- `ai::` — `AiRunner` trait + `Provider`, `RunnerInput`, `Event`,
  `RunResult`, `Cancel`.
- `paging::` — `Page<T>`, `Cursor`. `Repository<T>` derive is
  deferred to v0.2 (see `paging` module docs).
- `sort::`, `filter::` — `Sort`, `Filter` query primitives.
- `dto::` — `Health`, `Problem` (RFC 7807-style).
- `error::` — the unified `Error` enum.

## Usage

```rust
use starter_spi::auth::{Authenticator, Principal};

struct MyAuth;

#[async_trait::async_trait]
impl Authenticator for MyAuth {
    async fn verify(&self, credential: &str) -> starter_spi::Result<Principal> {
        // parse JWT, etc.
        unimplemented!()
    }
}
```

No features.
