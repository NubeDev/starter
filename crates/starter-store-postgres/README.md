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

- `testing` — placeholder; `testing::with_database` is deferred to
  v0.2 (needs Docker on the dev machine; SCOPE punts the
  `testcontainers` version pin). Use `starter-store-sqlite::testing`
  for unit-style coverage in the meantime.

Migration source names must match `^[a-z][a-z0-9_]{0,30}$`.
