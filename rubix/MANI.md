# Using `mani` in the rubix tree

`mani` is the task runner and repo manager we use to drive the daily
build/test/lint loop across the rubix crates. The config lives next to
this file at [mani.yaml](mani.yaml).

## Install

```sh
# Clone and build from source
git clone https://github.com/NubeDev/rubix-repos-cli.git /tmp/rubix-repos-cli
cd /tmp/rubix-repos-cli
go build -o mani .
install -m 0755 mani ~/.local/bin/mani

# Verify
mani --version
```

Alternative: prebuilt releases via the upstream installer —
`curl -sSL https://raw.githubusercontent.com/NubeDev/repos-cli/main/install.sh | sh`.

`~/.local/bin` must be on your `PATH`. Otherwise drop the binary in
`/usr/local/bin` (needs sudo) or `~/go/bin`.

## Daily commands

Run these from `rubix/` (where `mani.yaml` lives):

| Command | What it does |
|---|---|
| `mani run build --all` | `cargo build` every rubix crate |
| `mani run test --all` | `cargo test` every rubix crate |
| `mani run fmt --all` | `cargo fmt` the rubix workspace |
| `mani run clippy --all` | `cargo clippy -- -D warnings` |
| `mani run status --all` | `git status --short` for the rubix tree |
| `mani run run --all` | Boot the Phase 0 `rubix-agent` locally |
| `mani run healthz --all` | Hit `http://127.0.0.1:8080/healthz` |
| `mani run bootstrap-user --all` | Create/reconcile the first admin user (reads `RUBIX_DSN`, `RUBIX_BOOTSTRAP_EMAIL`, `RUBIX_BOOTSTRAP_PASSWORD`) |
| `mani run lint-doc-refs --all` | Enforce the doc-tier rule on rubix sources |

The `--all` flag fans out across every project in `mani.yaml`. With one
project (`rubix`) it's equivalent to targeting that project explicitly:
`mani run build -p rubix`.

## Discovery

```sh
mani list tasks            # all tasks defined in mani.yaml
mani list projects         # all projects
mani describe task build   # show the command a task runs
mani check                 # validate mani.yaml
```

## Adding a task

Edit [mani.yaml](mani.yaml) and add under `tasks:`:

```yaml
tasks:
  my-task:
    desc: One-line description.
    cmd: cargo something -p rubix-foo
```

Then `mani run my-task --all`. Per [SCOPE.md](SCOPE.md), new workflows
go in `mani.yaml` first and are documented after.

## Troubleshooting

- **`mani: command not found`** — `~/.local/bin` isn't on `PATH`. Add
  `export PATH="$HOME/.local/bin:$PATH"` to your shell rc.
- **Wrong config picked up** — pass `-c ./mani.yaml` explicitly, or run
  from the directory containing `mani.yaml`.
- **Need full command tree for scripting/LLMs** — `mani introspect`
  prints the entire CLI as JSON.

Upstream docs: https://manicli.com
