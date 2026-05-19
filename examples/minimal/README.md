# starter-minimal

Headless single-owner appliance in one binary. Demonstrates the
**recommended consumer layout** for a starter-based app:

- `starter-server` builds the axum app and ships `/health` + `/metrics`
  + `/openapi.json`.
- `starter-store-sqlite` provides the pool + namespaced migration
  runner.
- `starter-auth-token` ships the claim flow (`POST /auth/claim`) and
  the `TokenAuthenticator` that backs `with_principal`.
- `starter-cli` provides the remote-only subcommands (`health`,
  `openapi`); the consumer wires the local subcommands (`serve`,
  `migrate`, `claim-reset`) in their own `main.rs`.

## Run it

```bash
# Apply migrations (creates ./minimal.db).
cargo run -p starter-minimal -- migrate

# First-boot: print a pending claim token.
cargo run -p starter-minimal -- claim-reset --yes
# → ABCDEF…   (save this; it's gone after the next claim)

# Start the server.
cargo run -p starter-minimal -- serve
```

In another shell:

```bash
# Claim with the pending token. Server returns the plaintext
# owner_token exactly once.
curl -s -XPOST http://127.0.0.1:8080/auth/claim \
  -H content-type:application/json \
  -d '{"token":"<paste pending token>"}'
# → {"claim_id":"…","owner_token":"…"}

# Hit the protected route.
curl -s http://127.0.0.1:8080/hello -H "Authorization: Bearer <owner_token>"
# → hello, <claim_id>

# Built-in remote helpers (talk over HTTP via starter-client-rs).
cargo run -p starter-minimal -- health
cargo run -p starter-minimal -- openapi
```

## Why `serve` / `migrate` / `claim-reset` live here, not in starter-cli

Each needs the consumer's `Pool` to do its job. `starter-cli` is
deliberately store-agnostic (SCOPE R8: it talks to a server via
`starter-client-rs`, never directly to the DB) so the same CLI binary
can target a remote starter-server. Local-state commands live next to
the binary they bind to.

## Tests

`cargo test -p starter-minimal` exercises the full claim → bearer →
`/hello` round-trip against an in-memory SQLite + a real listener.
