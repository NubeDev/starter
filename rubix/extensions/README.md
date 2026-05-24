# rubix/extensions — rubix-owned extension binaries

This directory is a **sibling cargo workspace** (not a member of the
rubix root workspace — see the repo-root `Cargo.toml` `exclude` list).
It holds the extension binaries that ship as part of rubix itself,
each one a self-contained crate producing a single executable that
`rubix-agent` loads at boot via `starter-ext-host`.

The framework that defines what an extension is, how it is
discovered, supervised, and surfaced over REST lives upstream in
[`starter-extensions/`](../../starter-extensions/). This workspace
**consumes** that framework; the dependency arrow only ever points
rubix → starter-extensions (SCOPE.md R2).

## Layout

```
rubix/extensions/
├── Cargo.toml                       # sibling workspace root
├── README.md                        # this file
└── com.<org>.<name>/
    └── <flavour>/                   # one crate per flavour (process | inproc | wasm)
        ├── Cargo.toml
        └── src/main.rs
```

Today the workspace has one member —
[`com.rubix.example/process/`](com.rubix.example/process/) — the
reference process-flavour extension. Subsequent stages of the
rubix-extensions-wire job add more members alongside it.

## Adding a new extension

1. **Pick an id.** Use reverse-DNS, e.g. `com.acme.weather`.
2. **Copy** `com.rubix.example/process/` to
   `com.<org>.<name>/<flavour>/`.
3. **Edit** the new `Cargo.toml`:
   - rename `[package].name` (kebab-case, e.g. `acme-weather-extension`),
   - rename the `[[bin]].name` to match,
   - confirm the `starter-ext-sdk` path dep still resolves —
     four `..` segments hop up to the repo root, then down into
     `starter-extensions/crates/starter-ext-sdk` (see the comment
     in the example crate's `Cargo.toml`),
   - pick the SDK feature for your flavour (`process`, `inproc`,
     or `wasm`).
4. **Register** the new crate in this workspace's `Cargo.toml`
   `members` array.
5. **Implement** `src/main.rs` against `starter-ext-sdk`. The
   block-author guide in
   [`rubix/docs/design/extensions/README.md`](../docs/design/extensions/README.md)
   covers tools / skills / flows / nodes contributions and the
   `block.yaml` schema.

## Building

The workspace builds independently of the rubix root workspace:

```bash
cargo build --manifest-path rubix/extensions/Cargo.toml
```

To build a single member:

```bash
cargo build --manifest-path rubix/extensions/Cargo.toml -p rubix-example-extension
```

Binaries land under `rubix/extensions/target/debug/<name>` (or
`target/release/<name>` with `--release`). `rubix-agent` discovers
them at boot — point its extensions directory at the parent dir
holding each binary's `block.yaml`. The full host-side discovery
contract is documented in
[`rubix/docs/design/extensions/README.md`](../docs/design/extensions/README.md)
(rewritten in Phase E of the rubix-extensions-wire job).

## CI

The repo-root `.github/workflows/ci.yml` includes a dedicated
`rubix-extensions` job that runs `cargo build` against this
manifest on every PR. It is **separate from** the main rust job
that builds the rubix root workspace — by design, since the two
workspaces are independent cargo invocations.
