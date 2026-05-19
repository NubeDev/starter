# starter-gh-report (skeleton)

A **shape demonstration** of a consumer-domain CLI built on
`starter-cli`. The `report` subcommand currently prints a stubbed
JSON body — replace [`report::generate`](src/report.rs) with a real
`octocrab` call when the consumer's reporting contract is finalised.

## What this example locks in

- `register_starter_defaults()` + a domain `Command` impl is the
  canonical pattern for a CLI on `starter-cli`. The same pattern as
  [`examples/minimal`](../minimal), minus the server/storage halves.
- Tracing init through `starter-observability` so logs match the rest
  of the starter ecosystem (one filter, one format).
- Secrets handling stays under `starter-secrets-*`. When the GitHub
  API call lands, fetch the PAT from `SecretStore::get("github:pat")`
  with an env-var fallback.

## Run

```bash
cargo run -p starter-gh-report -- report --repo NubeDev/starter
```

Add real GitHub integration by:

1. Dropping `octocrab` into `Cargo.toml`.
2. Resolving the PAT via `SecretStore`.
3. Rewriting `report::generate` to call the live API.

Everything outside `report.rs` stays as-is.
