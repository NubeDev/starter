# HOW TO ADD CODE — Start Here Every Session

This doc is the single entry point for any coding session on the
**starter** mono-repo. It tells you:

1. **The hard rules** that keep the modular architecture honest.
2. **Where code goes** — the decision tree for placing new work.
3. **What each crate/package is for** and the dependency arrows.
4. **How to build and test.**

> **Prerequisite:** read [SCOPE.md](./SCOPE.md) once. It defines the
> hard rules (R1–R8), crate boundaries, dependency arrows, and
> non-goals. This doc assumes you've internalised it.

---

## 0 — Rule Zero: ONE RESPONSIBILITY PER FILE

**Applies to every language in this workspace: Rust, TypeScript.**

| Limit | Value |
|---|---|
| Max lines per file | **400** |
| Max lines per function / component | **50** |
| Max public items per module | **~10** |
| Max nesting depth | **4** |

When a file approaches **300 lines**, stop and ask: *what are the two
or more responsibilities living here?* Split before you hit 400.

**No `utils`, `helpers`, `common`, `misc` files.** Name the concept.

### Why this matters for AI assistants

- Small focused files → the AI loads 100% of what's relevant.
- File names are searchable concepts → navigation by name alone.
- Edits are surgical → one file = one responsibility = no accidental side-effects.

### Example — wrong vs correct

```
# Wrong
auth.rs          ← 600 lines, does everything

# Correct
auth/
  mod.rs              ← re-exports only, ~10 lines
  login.rs            ← authenticate existing user, issue token
  signup.rs           ← validate + create new user account
  password.rs         ← hashing, verification, reset flow
  session.rs          ← token lifecycle: issue, refresh, revoke
  middleware.rs       ← request guard / extractor (transport layer only)
  error.rs            ← auth-specific error types
```

---

### The split heuristic

1. **Can I describe this file's job in one sentence without "and"?** If not — two files.
2. **If this file changes, what else might break?** If unrelated things — mixed concerns.
3. **Would someone searching by filename find what they expect?** `password.rs` → yes. `utils.rs` → no.

### Naming rules

| Never | Always |
|---|---|
| `utils.rs` / `utils.ts` | Name the concept: `token_cache.rs`, `retry.ts` |
| `helpers.rs` / `helpers.ts` | Name the concept: `slot_coerce.rs`, `url_builder.ts` |
| `common.rs` / `common.ts` | Move shared types to `starter-spi`; name them |
| `misc.rs` | Don't create it |
| `index.ts` with 30 exports | Keep `index.ts` as a re-export barrel only |

---

## 1 — Hard rules (from SCOPE.md)

These are load-bearing. Breaking one collapses the modularity.

### R1 — One responsibility per crate/package

Already covered above (Rule Zero). No file > 400 lines. No crate
whose job needs "and" to describe.

### R2 — `starter-spi` is the contracts crate

Wire types + trait seams. Zero internal deps, zero runtime logic, zero
HTTP, zero SQL. Everything else depends on it; it depends on nothing.

### R3 — Transport never contains domain logic

REST handlers, gRPC handlers, CLI commands, MCP tool handlers are thin:
extract → call domain function → shape result → return. Smoke test: *if
I swap REST for gRPC tomorrow, how much of this file changes?*

### R4 — Storage is typed building blocks, not a universal trait

No `Store` trait in `spi`. Instead, `starter-store-sqlite` and
`starter-store-postgres` ship typed building blocks (pool wrapper,
migration runner, paging helpers, optional `Repository<T>` derive)
that the consumer composes into their own repositories.

### R5 — Default-features minimal; opt-in everything else

Every crate's `default-features = []`. Consumer pays only for what they
enable.

### R6 — TS client has zero React; UI-kit has zero I/O; UI-core owns the brain

- `@nube/starter-client-ts` — plain TS HTTP client, **no React**.
- `@nube/starter-ui-kit` — shadcn primitives + Tailwind tokens, **no I/O**.
- `@nube/starter-ui-core` — hooks, providers, stores. The portable brain.

### R7 — One source of truth for wire types; TS is codegen'd from Rust

`starter-spi` types → OpenAPI doc → `pnpm codegen` → TS client Zod
schemas. Hand-edited TS wire types are forbidden. CI fails on drift.

### R8 — Comments explain *why*, never *what*

Doc-comments on every public item. No `// STAGE-1 done`, no `// FIXED:`,
no emoji. TODOs carry a name or ticket: `// TODO(ap): …`.

---

## 2 — Where does my code go? (decision tree)

Walk top-to-bottom. Stop at the first "yes".

### Q1. Am I changing a wire-level type or trait seam?

*Examples: new error variant, new paging primitive, new trait method.*

→ **`crates/starter-spi/`**

Then: `pnpm codegen` to regenerate `packages/starter-client-ts/`.

### Q2. Is this server infrastructure? (routing, middleware, OpenAPI)

→ **`crates/starter-server/`**

### Q3. Is this storage building blocks? (pool, migrations, query helpers)

→ **`crates/starter-store-sqlite/`** or **`crates/starter-store-postgres/`** or **`crates/starter-store-warehouse/`**

### Q4. Is this authentication or authorization?

→ **`crates/starter-auth-token/`** (headless single-owner)
→ **`crates/starter-auth-users/`** (multi-user, sessions + API tokens)
→ **`crates/starter-auth-oauth/`** (OAuth2 flows)
→ **`crates/starter-authz/`** (permission enforcement)

### Q5. Is this secrets management?

→ **`crates/starter-secrets-keyring/`** (OS keychain, desktop)
→ **`crates/starter-secrets-file/`** (age-encrypted file, headless)

### Q6. Is this AI provider integration?

→ **`crates/starter-ai/`** — unified AiRunner with per-provider features.

### Q7. Is this MCP / tool protocol?

→ **`crates/starter-mcp/`** (MCP stdio server scaffold)
→ **`crates/starter-grpc/`** (gRPC tool surface)
→ **`crates/starter-jsonrpc-stdio/`** (JSON-RPC over stdio)

### Q8. Is this flow / automation engine?

→ **`crates/starter-flow-spi/`** (flow trait contracts)
→ **`crates/starter-flow/`** (engine runtime)
→ **`crates/starter-flow-nodes/`** (built-in node types)
→ **`crates/starter-flow-surfaces/`** (flow editor data)
→ **`crates/starter-flow-watch/`** (file-watch triggers)

### Q9. Is this blob / file storage?

→ **`crates/starter-blob-memory/`** (in-memory, tests)
→ **`crates/starter-blob-fs/`** (local filesystem)
→ **`crates/starter-blob-s3/`** (S3-compatible)
→ **`crates/starter-blob-garage/`** (Garage-specific)
→ **`crates/starter-blob-compose/`** (composite backend)
→ **`crates/starter-blob-axum/`** (HTTP upload/download routes)

### Q10. Is this a CLI building block?

→ **`crates/starter-cli/`** — clap subcommands as a library.

### Q11. Is this the Rust HTTP client?

→ **`crates/starter-client-rs/`** — shares `starter-spi` types only.

### Q12. Is this a Tauri desktop integration?

→ **`crates/starter-tauri/`**

### Q13. Is this a UI/theming concern on the Rust side?

→ **`crates/starter-ui-ir/`** (server-driven UI intermediate representation)
→ **`crates/starter-ui-theme/`** (theme definitions)
→ **`crates/starter-ui-builder/`** (UI builder logic)
→ **`crates/starter-sdui-routes/`** (SDUI HTTP routes)

### Q14. Is this a TS/React concern?

→ **`packages/starter-client-ts/`** — generated HTTP client, zero React.
→ **`packages/starter-ui-kit/`** — shadcn primitives, zero I/O.
→ **`packages/starter-ui-core/`** — hooks, stores, providers.
→ **`packages/starter-sdui-react/`** — SDUI React renderer.

### Q15. Is this a cross-cutting concern?

→ **`crates/starter-config/`** — layered config (env > file > default)
→ **`crates/starter-observability/`** — tracing + prometheus + middleware
→ **`crates/starter-cache/`** — caching primitives
→ **`crates/starter-i18n/`** — internationalization
→ **`crates/starter-prefs/`** — user preferences
→ **`crates/starter-tags/`** — tagging system
→ **`crates/starter-changelog/`** — change tracking
→ **`crates/starter-audit/`** — audit log
→ **`crates/starter-export/`** — data export
→ **`crates/starter-insights/`** — analytics / insights
→ **`crates/starter-undo/`** — undo/redo

### Q16. Is this a consumer application (example)?

→ **`examples/minimal/`** — sqlite + auth-token + MCP
→ **`examples/notes/`** — full notes app with frontend
→ **`examples/gh-report/`** — GitHub reporting tool
→ **`examples/flow-agent/`** — flow engine demo
→ **`examples/blobs/`** — blob storage demo
→ **`examples/iot-anomaly-detector/`** — IoT example
→ **`examples/authz-demo/`** — authorization demo

### Still unsure?

→ Read [SCOPE.md](./SCOPE.md) for the full dependency arrow, then ask.

---

## 3 — Crate cheat-sheet

| Crate | Owns | Depends on |
|---|---|---|
| `starter-spi` | Wire types, traits (Authenticator, SecretStore, Tool, AiRunner), DTOs | Nothing |
| `starter-config` | Layered config via figment | `starter-spi` |
| `starter-observability` | Tracing, prometheus, middleware | `starter-spi` |
| `starter-server` | Axum app builder, OpenAPI, SSE | `starter-spi` |
| `starter-store-sqlite` | sqlx+sqlite building blocks | `starter-spi` |
| `starter-store-postgres` | sqlx+postgres building blocks | `starter-spi` |
| `starter-mcp` | MCP stdio server scaffold | `starter-spi` |
| `starter-grpc` | tonic gRPC tool surface | `starter-spi` |
| `starter-auth-token` | Single-owner bearer auth | `starter-spi` |
| `starter-auth-users` | Multi-user: sessions + API tokens | `starter-spi` |
| `starter-ai` | AI runner (multi-provider, feature-gated) | `starter-spi` |
| `starter-client-rs` | Rust HTTP client | `starter-spi` |
| `starter-cli` | clap building blocks (library) | `starter-spi`, `starter-client-rs` |

| Package | Owns |
|---|---|
| `starter-client-ts` | TS HTTP client. Codegen'd. **Zero React.** |
| `starter-ui-kit` | shadcn primitives + Tailwind + theme. **Zero I/O.** |
| `starter-ui-core` | Hooks, stores, providers. The brain. |

---

## 4 — Library boundary rules (MUST / MUST NOT)

| Crate/Package | MUST | MUST NOT |
|---|---|---|
| `starter-spi` | Be standalone. Types + traits only. | Depend on anything internal. Contain runtime logic. |
| `starter-client-ts` | Be a thin HTTP client. Validate with Zod. | Import React or any UI library. |
| `starter-ui-kit` | Ship Shadcn primitives. Accept data via props. | Import react-query, zustand, or do any I/O. |
| `starter-ui-core` | Own every hook/store that talks to the server. | Contain app-specific pages or routing. |
| `starter-server` | Accept consumer-built Routers via composition. | Contain consumer domain logic. |
| `starter-store-*` | Ship typed building blocks. | Contain consumer-specific SQL. |
| `starter-auth-*` | Implement the `Authenticator` trait from `spi`. | Depend on each other. |
| `examples/*` | Demonstrate consumer usage of starter crates. | Contain reusable library code (move it to a crate). |

---

## 5 — Building and testing

```bash
# Build the entire workspace
cargo build

# Run all tests
cargo test

# Build a specific example
cargo build -p notes

# Run the notes example
cargo run -p notes -- serve --database-url "sqlite:notes.db?mode=rwc"

# Regenerate TS client from OpenAPI
pnpm codegen

# Run TS package tests
pnpm test

# Lint
cargo clippy --workspace --all-targets
```

### Smoke tests

The `crates/smoke-tests/` crate contains integration tests that spin up
a real server and exercise the full stack. Run with:

```bash
cargo test -p smoke-tests
```

---

## 6 — Layer separation

Within any consumer binary (and within starter crates themselves):

```
transport (REST / gRPC / CLI / MCP)
    ↓ calls
domain logic (pure functions, business rules)
    ↓ calls
data (storage, external APIs)
```

Never the other way. No SQL in handlers, no HTTP in domain.

Transport handlers do four things:
1. Extract inputs
2. Call a domain function
3. Map the result to a DTO
4. Return

If your handler has business logic — it's in the wrong layer.

---

## 7 — Comments

- Doc-comments (`///`) on every public item: purpose, defaults, edge cases.
- Explain *why*, not *what*. Skip obvious comments.
- No session-progress markers (`// STAGE-1 done`, `// FIXED:`).
- No emojis or ASCII art banners.
- `// TODO(name): …` — never bare TODOs.
- Keep comments current. Stale comment → fix it in the same diff.

---

## 8 — Commit etiquette

- Commit only when the user explicitly asks.
- Never amend; create a new commit.

## 8 — Commit etiquette

- Commit only when the user explicitly asks.
- Never amend; create a new commit.
- Never skip hooks (`--no-verify`) unless the user explicitly says so.
- Never force-push to `main`.
- Commit message focuses on **why**, not what. One or two sentences.

---

## 9 — What to do when stuck

Ask. Don't guess at:

- **Crate placement** — getting the decision tree wrong means moving
  code later.
- **Trait seam changes** — a change to `starter-spi` cascades to every
  consumer crate.
- **Feature-gate decisions** — an accidentally-default feature pulls
  transitive deps consumers didn't ask for.

One sentence of "which of these two did you want?" beats two hours of
refactoring the wrong direction.

---

## One-line summary

**Start here → pick the right crate via the decision tree → write it
with layer separation → `cargo build && cargo test` → commit when
asked. Modular libraries are the product, not an implementation
detail.**