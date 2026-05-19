# Using `starter` as a reusable library

Audience: another AI agent (or engineer) bootstrapping a new product on
top of the `starter-*` Rust crates and `@nube/starter-*` TS packages in
this repo. This file is the entry point — it tells you what `starter`
is, what it is **not**, the rules you must follow, and where the worked
example lives.

> Read [SCOPE.md](../../SCOPE.md) first. It defines the crate
> boundaries, dep arrows, and non-goals. This document is the
> operational complement to that one.

---

## 1. Mental model

`starter` is a **library set**, not a framework.

- The consumer owns: the domain types, the binary, the
  `Cargo.toml`, the migration set, the router composition, the
  CLI entry point, the frontend app.
- `starter-*` owns: contracts (`starter-spi`), an axum
  `ServerBuilder`, store building blocks, an MCP scaffold,
  auth backends, observability, a `CommandRegistry` for CLI,
  and a typed HTTP client.

There are **no plugin hooks, no required traits on the consumer side,
no inversion of control beyond standard Rust traits**. If you cannot
do it by calling a public function from a `starter-*` crate, you do
not do it. You do **not** edit a `starter-*` crate to add a feature
to your product — that is the hard rule the example exists to prove.

---

## 2. The canonical example

[`examples/notes/`](../../examples/notes/) is the reference consumer.
**Read it before writing any code.** Every surface (`REST`, `MCP`,
`CLI`, `gRPC`, `UI`, auth, storage, migrations) is exercised there
without a single edit to `crates/starter-*` or `packages/`.

Map by surface — open these files in order if you're a fresh agent:

| Surface           | File                                                              | What to copy                                  |
|-------------------|-------------------------------------------------------------------|-----------------------------------------------|
| Domain (your code)| [examples/notes/src/domain.rs](../../examples/notes/src/domain.rs) | A plain Rust store. No `starter-*` types in signatures. |
| REST router       | [examples/notes/src/rest.rs](../../examples/notes/src/rest.rs)     | An axum `Router<S>` you hand to `ServerBuilder`. |
| Server composition| [examples/notes/src/server.rs](../../examples/notes/src/server.rs) | `ServerBuilder::new(...).merge_router(...).with_openapi(...).with_metrics(...).build()` |
| MCP tool          | [examples/notes/src/mcp.rs](../../examples/notes/src/mcp.rs)       | `impl Tool` then `ToolRegistry::new().register(...)`. |
| CLI subcommand    | [examples/notes/src/cli.rs](../../examples/notes/src/cli.rs)       | `impl starter_cli::Command`, register into `CommandRegistry`. |
| Binary entry      | [examples/notes/src/main.rs](../../examples/notes/src/main.rs)     | `register_starter_defaults().register(...)`, plus `serve` / `migrate` / `claim`. |
| Migrations        | [examples/notes/src/migrations.rs](../../examples/notes/src/migrations.rs) | One namespaced `MigrationSource` per package; run them all through one `migrate(&pool)` chain. |
| gRPC (consumer)   | [examples/notes/src/grpc.rs](../../examples/notes/src/grpc.rs)     | Bring your own tonic; starter has no opinion here. The same `Authenticator` gates it. |
| E2E test          | [examples/notes/tests/e2e.rs](../../examples/notes/tests/e2e.rs)   | How to spin the real router in-process. |
| Frontend          | [examples/notes/frontend/src/](../../examples/notes/frontend/src/) | Compose `StarterClient`; reuse `<AuthProvider>` and `tokenStrategy`. |

If a problem you're solving is already solved in `examples/notes/`,
mimic the pattern. If it is not, check `SCOPE.md` and the crate's own
README before inventing something new.

---

## 3. Picking dependencies

A new consumer's `Cargo.toml` pulls only what it uses. The notes
example pulls this set (see
[examples/notes/Cargo.toml](../../examples/notes/Cargo.toml)):

```toml
starter-spi           = { workspace = true }   # contracts only
starter-server        = { workspace = true }   # axum builder
starter-store-sqlite  = { workspace = true }   # or starter-store-postgres
starter-auth-token    = { workspace = true, features = ["sqlite"] }
starter-mcp           = { workspace = true, features = ["http"] }
starter-observability = { workspace = true }
starter-cli           = { workspace = true }
starter-client-rs     = { workspace = true }   # only if you need it
```

Rules:
- **Pick exactly one store** (`sqlite` **or** `postgres`).
- **Pick exactly one auth backend**: `starter-auth-token` (single owner,
  headless appliances) **or** `starter-auth-users` (multi-user, cookie
  sessions + API tokens). Don't mix.
- **Pick at most one secrets backend**: `starter-secrets-keyring` (OS
  keychain) **or** `starter-secrets-file` (age-encrypted file).
- `starter-spi` is zero-dep contracts; depend on it freely.
- `starter-cli` is a **library**, not a binary — you build your own
  binary and register commands into `CommandRegistry`.

Frontend (if any) — from
[examples/notes/frontend](../../examples/notes/frontend/):

```json
"@nube/starter-client-ts": "workspace:*",
"@nube/starter-ui-kit":    "workspace:*",
"@nube/starter-ui-core":   "workspace:*"
```

---

## 4. The five extension points

Each is **one public call**. There is no other way. If you find
yourself wanting to add a "hook" inside a `starter-*` crate, stop and
re-read this section.

### 4.1 REST — merge a `Router<S>` into `ServerBuilder`

```rust
let router = ServerBuilder::<AppState>::new(AppState)
    .merge_router(my_router)          // your axum Router<AppState>
    .merge_router(starter_auth_token::routes::claim_router(claim_state))
    .with_openapi(MyApi::openapi())   // utoipa-generated
    .with_metrics(registry, metrics)
    .build();
```

Your router is generic over the parent state `S`. See
[`notes_router`](../../examples/notes/src/rest.rs).

### 4.2 Auth — wrap routes with `with_principal`

```rust
let auth: Arc<dyn Authenticator> =
    Arc::new(TokenAuthenticator::new(SqliteClaimStore::new(pool)));
let protected = starter_server::auth::with_principal(my_router, auth_for_http);
```

Inside handlers, extract `Option<Extension<Principal>>`. Same
`Authenticator` instance feeds MCP and (in the demo) gRPC.

### 4.3 MCP tool — `impl Tool`, register into `ToolRegistry`

```rust
#[async_trait]
impl Tool for MySearchTool {
    fn definition(&self) -> ToolDefinition { /* name + JSON schema */ }
    async fn invoke(&self, input: Value) -> SpiResult<Value> { /* ... */ }
}

let tools = Arc::new(ToolRegistry::new().register(MySearchTool { ... }));
let mcp  = mcp_router::<AppState>(tools, McpHttpOptions::new().with_auth(auth));
```

See [src/mcp.rs](../../examples/notes/src/mcp.rs).

### 4.4 CLI — `impl Command`, register into `CommandRegistry`

```rust
#[async_trait]
impl Command for NoteAdd {
    fn name(&self) -> &'static str { "add" }
    fn subcommand(&self) -> clap::Command { ... }
    async fn run(&self, m: &ArgMatches) -> Result<(), CommandError> { ... }
}

let registry = CommandRegistry::new()
    .register_starter_defaults()   // gives you `health`, `openapi`
    .register(NoteAdd)
    .register(NoteList);

// in clap:
let app = Command::new("myapp").subcommands(registry.subcommands());
// dispatch what wasn't your own subcommand:
registry.dispatch(&matches).await?;
```

See [src/cli.rs](../../examples/notes/src/cli.rs) and the
`main.rs` wiring.

### 4.5 Storage / migrations — namespaced sources, one runner

```rust
static AUTH_TOKEN: sqlx::migrate::Migrator =
    sqlx::migrate!("../../crates/starter-auth-token/migrations/starter_auth_token");
static MY_TABLES:  sqlx::migrate::Migrator = sqlx::migrate!("./migrations/mine");

pub fn sources() -> [MigrationSource; 2] {
    [
        MigrationSource { name: "starter_auth_token", migrator: &AUTH_TOKEN },
        MigrationSource { name: "mine",               migrator: &MY_TABLES },
    ]
}

// in `migrate` subcommand:
let mut chain = migrate(&pool);
for s in sources() { chain = chain.with_source(s); }
chain.run().await?;
```

Each source gets its own `_sqlx_migrations_<name>` table — version
counters never collide between starter crates and your code. See
[src/migrations.rs](../../examples/notes/src/migrations.rs).

---

## 5. Surfaces `starter` does NOT ship

These are explicit non-goals. Bring them in yourself if you need
them — the example shows the pattern for one:

- **gRPC** — not in any `starter-*` crate. The notes demo brings
  `tonic` + `prost` + `tonic-build` at the consumer level and stands
  up its own service that calls into the same `NoteStore` and same
  `Authenticator`. See [src/grpc.rs](../../examples/notes/src/grpc.rs).
- **Background job runners, queues, schedulers.**
- **Email / SMS / push providers.**
- **A domain.** `starter` does not know what a "user", a "note", or
  an "order" is. That's yours.

---

## 6. Hard rules (do not break these)

1. **Never edit a `crates/starter-*` file from a consumer.** If you
   feel you have to, you're holding it wrong. Re-check the public
   API. The notes demo proves zero edits are needed for every shipped
   surface.
2. **Never depend on a `starter-*` internal module path.** Only use
   what's re-exported from the crate root or a documented submodule.
3. **Do not put `starter-*` types in domain function signatures.**
   Your `Store`, your errors, your domain structs — plain Rust.
   Starter types appear only at the edge (router composition, tool
   registration, auth wiring, migration sources). Compare
   [domain.rs](../../examples/notes/src/domain.rs) vs
   [server.rs](../../examples/notes/src/server.rs).
4. **One store, one auth backend, one secrets backend per binary.**
   Mixing is not supported.
5. **`starter-cli` is a library.** You write the `main.rs`. You own
   the clap `Command`. Starter only contributes subcommands via the
   registry.
6. **Migrations are namespaced.** Always use `MigrationSource { name,
   migrator }` — never run a starter migrator directly against a
   shared `_sqlx_migrations` table.
7. **OpenAPI is consumer-owned.** Annotate your handlers with
   `#[utoipa::path(...)]`, define a `#[derive(OpenApi)]` doc struct,
   and pass it to `ServerBuilder::with_openapi`. Starter does not
   merge multiple docs for you.
8. **The same `Authenticator` should gate every surface** (HTTP, MCP,
   and any you add like gRPC). Don't invent a second auth layer.

---

## 7. Build a new consumer — minimal recipe

Goal: an HTTP + MCP + CLI app on SQLite with token auth. Cargo
manifest mirrors [examples/notes/Cargo.toml](../../examples/notes/Cargo.toml)
(drop tonic/prost if you don't need gRPC).

1. **Domain.** Write `domain.rs` with your types and a `Store` that
   owns a `SqlitePool`. No `starter-*` imports.
2. **REST.** Write a `Router<S>` of axum routes. Annotate with utoipa.
   Define a `NotesApi`-style `#[derive(OpenApi)]` struct.
3. **MCP** (optional). Implement `Tool` for any operation an LLM
   should be able to call. Register into `ToolRegistry`.
4. **CLI** (optional). Implement `Command` for each subcommand you
   want to ship. They're typically thin HTTP clients against your own
   server. Register into `CommandRegistry`.
5. **Migrations.** Put SQL under `./migrations/<your-name>/`. Build a
   `sources()` function returning your migrator alongside any
   starter migrators you need (e.g. `starter_auth_token`).
6. **Server build.** Compose with `ServerBuilder`. Wrap protected
   routes with `with_principal`. Add MCP via `mcp_router`. Wire
   metrics via `with_metrics`.
7. **`main.rs`.** clap top-level with `migrate`, `serve`, `claim`,
   plus `registry.subcommands()` from `CommandRegistry`.
   Initialize tracing with `starter_observability::tracing::init`.
8. **E2E test.** Use `starter-server`'s `testing` feature; spin the
   real router in-process. See
   [tests/e2e.rs](../../examples/notes/tests/e2e.rs).

If a step doesn't match the corresponding file in `examples/notes/`,
prefer the example — this document can drift, the example is
compiled and tested.

---

## 8. Where to read next

- [SCOPE.md](../../SCOPE.md) — boundaries, dep arrows, non-goals.
- [RELEASING.md](../../RELEASING.md) — version + publish flow for
  `starter-*` crates (only relevant if you're modifying starter
  itself, which you shouldn't be as a consumer).
- [examples/notes/README.md](../../examples/notes/README.md) — the
  worked example, surface-by-surface.
- [DOCS/extensions/](../extensions/) — deeper notes on production
  and scope concerns.

If something here disagrees with the example, the example wins. If
something here disagrees with `SCOPE.md`, `SCOPE.md` wins.
