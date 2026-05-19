# starter-store-sqlite

Typed building blocks for SQLite-backed apps. **No `Store` trait** —
consumers compose their own repositories from these primitives (see
SCOPE.md R4).

## What's inside

- `pool::{Pool, connect}` — cloneable ref-counted handle around
  `sqlx::SqlitePool`.
- `migrate::{migrate, MigrationSource}` — namespaced migration runner.
  Each source records progress in `_sqlx_migrations_<name>` so consumer
  + starter migrations never collide.
- `paging::` — cursor encode/decode helpers.

## Usage

```rust
use starter_store_sqlite::{migrate, pool, MigrationSource};

static APP: sqlx::migrate::Migrator = sqlx::migrate!("./migrations/app");

let pool = pool::connect("sqlite:./app.db?mode=rwc").await?;

migrate(&pool)
    .with_source(MigrationSource { name: "app", migrator: &APP })
    .run()
    .await?;
```

## Features

- `testing` — `testing::ephemeral()` returns an in-memory `Pool` for
  unit tests.

Migration source names must match `^[a-z][a-z0-9_]{0,30}$` — the name
is concatenated into a SQL identifier and the pattern check is what
makes that safe.
