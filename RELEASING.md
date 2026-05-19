# Releasing

This workspace publishes lockstep majors across all `starter-*` crates
on crates.io and all `@nube/starter-*` packages on npm (SCOPE.md §144).
Major bumps move every published artifact together; minor / patch
bumps can be independent.

## Pre-release checklist

Run from a clean working tree on `master`:

```bash
# Workspace must be green on the same toolchain CI uses.
cargo fmt --all -- --check
cargo check --workspace --all-features
cargo clippy --workspace --all-features --tests -- -D warnings
cargo test --workspace --no-fail-fast

# Backend-feature suites the default `--workspace` invocation misses.
cargo test -p starter-auth-token  --features sqlite  --no-fail-fast
cargo test -p starter-auth-users  --features sqlite  --no-fail-fast
cargo test -p starter-server      --features testing --no-fail-fast
cargo test -p starter-mcp         --features http    --no-fail-fast

# TS side.
pnpm install --frozen-lockfile
pnpm -r run typecheck
pnpm -r run test
pnpm -r run build

# OpenAPI / TS codegen must be up to date (CI drift gate).
cargo test -p starter-auth-users --features sqlite --test openapi_snapshot
pnpm --filter @nube/starter-client-ts run codegen
git diff --exit-code openapi.json packages/starter-client-ts/src/generated/
```

If any step fails, fix it before continuing. The CI workflow at
[.github/workflows/ci.yml](.github/workflows/ci.yml) runs the same
gates on every PR, so a green CI run on the release commit is the
authoritative signal.

## Cutting the release

1. Update [CHANGELOG.md](CHANGELOG.md): move `Unreleased` content
   under a new `[X.Y.Z] — YYYY-MM-DD` heading; add the compare /
   tag links at the bottom.
2. Bump the workspace version in [Cargo.toml](Cargo.toml) under
   `[workspace.package]`. Every Rust crate inherits this via
   `version.workspace = true`.
3. Bump the npm packages. Each `packages/*/package.json` carries its
   own `version` field; bump in lockstep with the Rust workspace
   for major releases.
4. Commit: `git commit -am "release vX.Y.Z"`.
5. Tag: `git tag -a vX.Y.Z -m "vX.Y.Z"`.
6. Push: `git push origin master --tags`. CI on the tag run is the
   last green light before publishing.

## Publishing

Rust — publish in dependency order so each crate's deps are already
on crates.io when it goes up. `starter-spi` has no `starter-*` deps;
the store / secret crates depend on it; the server / auth crates
depend on those; the CLI / client / examples sit at the top.

```bash
# Bottom of the dep graph first.
cargo publish -p starter-spi
cargo publish -p starter-observability
cargo publish -p starter-config

# Storage.
cargo publish -p starter-store-sqlite
cargo publish -p starter-store-postgres

# Secrets.
cargo publish -p starter-secrets-file
cargo publish -p starter-secrets-keyring

# AI.
cargo publish -p starter-ai

# Auth.
cargo publish -p starter-auth-token
cargo publish -p starter-auth-users

# Server / MCP / client / CLI.
cargo publish -p starter-server
cargo publish -p starter-mcp
cargo publish -p starter-client-rs
cargo publish -p starter-cli
```

Each `cargo publish` runs its own verify build; a failure here means
the published artifact would have been broken. Don't skip with
`--no-verify`.

npm — publish all packages from the workspace root:

```bash
pnpm -r --filter "@nube/starter-*" publish --access public
```

`@nube/starter-ui-core` depends on `@nube/starter-client-ts` via
`workspace:*`. pnpm rewrites `workspace:*` to the resolved version
on publish, so order doesn't matter — but if `pnpm publish` fails
on the dependent package, re-publish the dependency first.

## Post-release

1. GitHub release: `gh release create vX.Y.Z --notes-from-tag` (or
   paste the CHANGELOG section into the body).
2. If anything went wrong with a crate publish, yank it
   (`cargo yank --version X.Y.Z starter-foo`) — never delete.
3. Open a follow-up PR moving CHANGELOG `Unreleased` back to an
   empty section, ready for the next cycle.
