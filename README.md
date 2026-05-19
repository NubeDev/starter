# starter

Reusable libraries — Rust crates + React/TS packages — for the
**cli + server + storage + MCP + admin UI** plumbing that every new
product re-implements.

**Start here:** [SCOPE.md](./SCOPE.md). The whole design (hard rules,
crate boundaries, dep arrows, smoke tests, non-goals) lives there.
Read it before touching code.

## Workspace at a glance

```
crates/
  starter-spi              contracts. Zero deps. R2.
  starter-config           layered config.
  starter-observability    tracing + prometheus + middleware factories.
  starter-server           axum app builder + OpenAPI + SSE.
  starter-store-sqlite     sqlx-on-sqlite typed building blocks.
  starter-store-postgres   sqlx-on-postgres typed building blocks.
  starter-mcp              MCP stdio server scaffold.
  starter-auth-token       single-owner bearer auth (headless appliance).
  starter-auth-users       multi-user: cookie sessions + API tokens.
  starter-secrets-keyring  SecretStore over the OS keychain.
  starter-secrets-file     SecretStore over an age-encrypted file.
  starter-ai               AiRunner for 5 providers (all feature-gated).
  starter-client-rs        reqwest client mirroring the server surface.
  starter-cli              clap building blocks. Library, not a binary.

packages/
  starter-client-ts        TS client. Codegen'd from OpenAPI. No React.
  starter-ui-kit           shadcn primitives + Tailwind + theme switch.
  starter-ui-core          react-query + zustand hooks. The brain.

docker/                    parameterized Dockerfile + reference compose.
examples/                  minimal, full, and gh-report walkthroughs.
```

## Two installs, the consumer's choice

A new product picks the pieces it needs:

```toml
# Cargo.toml
starter-spi            = "0.1"
starter-server         = "0.1"
starter-store-sqlite   = "0.1"
starter-cli            = "0.1"
```

```json
// package.json
"@nube/starter-client-ts": "^0.1",
"@nube/starter-ui-kit":    "^0.1",
"@nube/starter-ui-core":   "^0.1"
```

The product owns its domain, its router(s), and its CLI binary.
`starter` is a dependency, not a parent.
