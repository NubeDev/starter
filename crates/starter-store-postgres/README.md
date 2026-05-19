# starter-store-postgres

Postgres twin of `starter-store-sqlite`. Same shape, same fluent
migration runner, same cursor codec — pick this when the consumer
already runs Postgres.

## Usage

```rust
use starter_store_postgres::{migrate, pool, MigrationSource};

static APP: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/app");

let pool = pool::connect("postgres://postgres@localhost/app").await?;

migrate(&pool)
    .with_source(MigrationSource { name: "app", migrator: &APP })
    .run()
    .await?;
```

## Features

- `testing` — `testing::with_database()` spins up an ephemeral
  Postgres container via `testcontainers-modules` (0.23 / 0.11) and
  returns `(Pool, ContainerGuard)`. Requires Docker on the host;
  integration tests are marked `#[ignore]` so plain `cargo test`
  skips them. Run explicitly:

  ```bash
  cargo test -p starter-store-postgres --features testing -- --ignored
  ```

  CI runs this on every PR (GitHub-hosted runners ship Docker).

Migration source names must match `^[a-z][a-z0-9_]{0,30}$`.
