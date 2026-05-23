# rubix — testing setup

This page covers the **container prerequisites** for rubix integration
tests. Pure-Rust unit tests run without it — `cargo test --workspace`
from the repo root needs nothing extra.

The Postgres and ClickHouse smokes under
`rubix/agent/crates/data-{postgres,clickhouse}/tests/smoke.rs` boot
ephemeral testcontainers via the starter testing seam
(`starter_store_postgres::testing::with_database` /
`starter_store_clickhouse::testing::with_clickhouse`). Per the starter
convention they are marked `#[ignore]`, so a default `cargo test` will
skip them. CI and local dev opt in with `-- --ignored`.

## 1. Prerequisites

You need exactly one thing: **a running Docker daemon the current user
can talk to**. The testcontainers crate auto-discovers the daemon at
the usual sockets (`/var/run/docker.sock` on Linux/macOS, the named
pipe on Windows / Docker Desktop). No images need pre-pulling —
testcontainers pulls on first use.

Quick check that the daemon is up and accessible:

```bash
docker info >/dev/null && echo "docker OK"
```

If that prints `docker OK`, you are done with setup; jump to §3.

## 2. One-command boot for the daemon

If `docker info` fails the daemon is either not installed or not
running. Pick the option for your host:

- **Linux**: install Docker Engine (`https://docs.docker.com/engine/install/`)
  and add your user to the `docker` group, then either log out / back
  in or run `newgrp docker` in your shell.
- **macOS / Windows**: install Docker Desktop and start it from the
  applications menu.
- **CI**: the GitHub-hosted `ubuntu-latest` runner already exposes a
  working daemon — nothing to do.

The rubix smokes do **not** require a long-lived `docker compose`
stack. Each test spins its own container and tears it down on drop.
If you nevertheless want a long-lived Postgres + ClickHouse pair for
ad-hoc poking, the starter repo ships one under `docker/` (root of the
repo) that you can bring up with:

```bash
docker compose -f docker/compose.yaml up -d postgres clickhouse
```

This is **not** a prerequisite for the rubix smokes — they ignore the
long-lived containers and start their own. The compose file is listed
here only because it's the single command an operator has to learn.

## 3. Running the smokes

From the repo root:

```bash
# Postgres smoke (boots an ephemeral pg testcontainer, runs SELECT 1).
cargo test -p rubix-data-postgres -- --ignored

# ClickHouse smoke (boots an ephemeral ch testcontainer, runs SELECT 1).
cargo test -p rubix-data-clickhouse -- --ignored
```

Both should print `test result: ok. 1 passed`. First runs are slow
because Docker pulls the image; subsequent runs reuse the local layer
and finish in a few seconds (Postgres) to about thirty seconds
(ClickHouse, whose first-boot wait is longer).

## 4. Troubleshooting

- **`permission denied while trying to connect to the Docker daemon
  socket`** — your user is not in the `docker` group. Fix per §2.
- **`failed to pull image`** — usually a network or registry-auth
  issue. Try `docker pull postgres:latest` manually to confirm.
- **Tests pass locally but hang in CI** — confirm the CI job actually
  has access to Docker. Self-hosted runners often need
  `DOCKER_HOST=unix:///var/run/docker.sock` in the env.
- **`address already in use`** — testcontainers binds to ephemeral
  ports, so a port conflict is almost always a stuck container from
  a previous abort. `docker ps -a | grep testcontainers` and prune.
