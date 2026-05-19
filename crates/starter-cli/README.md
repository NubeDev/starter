# starter-cli

CLI scaffolding. Ships **only the remote subcommands** (`health`,
`openapi`) that talk to a running starter-server via
`starter-client-rs`. Local-state subcommands (`serve`, `migrate`,
`admin create`, …) live in the consumer's binary — they need the
consumer's `Pool` and DI, and `starter-cli` is deliberately
store-agnostic (SCOPE R8).

## Usage

```rust
use starter_cli::registry::CommandRegistry;

let registry = CommandRegistry::new().register_starter_defaults();

let app = clap::Command::new("my-tool")
    .subcommands(registry.subcommands())
    .subcommand(clap::Command::new("serve") /* local */);

let matches = app.get_matches();
match matches.subcommand() {
    Some(("serve", sub)) => /* consumer's serve impl */,
    _ => registry.dispatch(&matches).await?,
}
```

See [`examples/minimal`](../../examples/minimal) for the recommended
pattern — wires `serve`, `migrate`, and `claim-reset` next to the
remote defaults.

## Subcommand template

`crates/starter-cli/src/commands/admin_create.rs` is a copy-paste
template for the `admin create` bootstrap path against
`starter-auth-users`. Not registered by `register_starter_defaults` on
purpose.

No features.
