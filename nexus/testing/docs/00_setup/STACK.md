# Stack Reference — Processes, Ports, Env, Teardown

> Verified: nexus-rewrite tip on 2026-06-10. Source of truth: `nexus/Makefile`.

Everything the testing stack runs, how to start/stop it, and the knobs. When a
process won't come up, this is the reference; for *why* it behaves a certain way
see [../reference/ARCHITECTURE.md](../reference/ARCHITECTURE.md).

---

## Process / port map

| Component | Up | Down | Port | Container / bin |
|-----------|----|----|------|-----------------|
| Postgres (dev) | `make db` | `make db-stop` | `4770` | `nexus-dev-pg` |
| Backend API | `make dev-be` | `make kill` | `4780` | `nexus-api` |
| UI (optional) | `make dev-ui` | `make kill` | `4790` | vite |
| Both BE+UI | `make dev` | Ctrl-C / `make kill` | 4780/4790 | — |
| MQTT broker | `docker run … eclipse-mosquitto:2` | `docker rm -f nexus-test-mqtt` | `1883` | `nexus-test-mqtt` |
| NATS broker | `make nats-install` | `make nats-stop` | `4222` (mon `8222`) | `nexus-test-nats` |
| Zenoh router | `make zenoh-install` | `make zenoh-stop` | `7447` (rest `8000`) | `nexus-test-zenoh` |
| Both brokers | `make brokers` | `make brokers-stop` | — | — |
| Data generator | `cd testing/datapump && cargo run -- …` | Ctrl-C | — | — |

One-command fresh start: **`make dev-all`** (bootstrap + seed + run).

---

## Environment variables (Makefile defaults)

These are exported by the Makefile for the dev targets. Override on the CLI for
anything beyond local testing — the key material below is **not secret**.

| Var | Default | Purpose |
|-----|---------|---------|
| `NEXUS_METADATA_URL` | `postgres://nexus:nexus@127.0.0.1:4770/nexus` | control-plane DB |
| `NEXUS_DATASOURCE_URL` | same as metadata | telemetry / query DB |
| `NEXUS_MASTER_KEY` | 32-byte dev hex | secret encryption (must be exactly 32 bytes) |
| `NEXUS_STREAM_TOKEN_KEY` | 32-byte dev hex | stream tokens (≥32 bytes) |
| `NEXUS_KINDS_DIR` | `crates/nexus-api/kinds` | query-kinds pack (relative to `backend/`) |
| `NEXUS_DATASOURCE_KINDS_DIR` | `crates/nexus-api/datasource-kinds` | datasource-kinds pack |
| `NEXUS_EXTENSIONS_DIR` | `crates/nexus-api/extensions` | read-only in-repo extensions |
| `NEXUS_EXTENSIONS_INSTALLS_DIR` | `.nexus-ext/installs` | uploaded installs (scratch) |
| `NEXUS_EXTENSIONS_PIDFILE_DIR` | `.nexus-ext/pids` | supervisor pidfiles (scratch) |
| `ADMIN_EMAIL` | `admin@nexus.local` | seed admin |
| `ADMIN_PASSWORD` | `change-me-admin` | seed admin |
| `BE_PORT` / `UI_PORT` / `DB_PORT` | 4780 / 4790 / 4770 | remap dev ports |
| `SIM_ROWS` | `200` | rows per profile for `make seed-sim` |

> Gotcha: dev targets run with CWD `backend/`, so the kinds/extensions dirs are
> resolved relative to `backend/`. If the Explore "Kind" picker shows "No kinds",
> the dir env vars didn't take.

---

## Seeding data

| Command | Bin | Writes |
|---------|-----|--------|
| `make seed` | `seed-admin` | admin tenant + user + grants (idempotent) |
| `make seed-sim` | `nexus-seed` | `sim_hvac` / `sim_energy` / `sim_door`, `SIM_ROWS` deterministic rows each |

`seed-sim` gives you query/dashboard data **without** standing up a broker —
useful for testing dashboards/insights/alerts in isolation before the ingest
path is wired.

---

## Brokers — which one, when

- **MQTT (Mosquitto)** — what `datapump` speaks today. Needs a bridge to reach
  Nexus (no MQTT source). Simplest broker to run.
- **Zenoh** — Nexus has a native `zenoh` source (feature-gated). The cleanest
  no-bridge path once datapump's Zenoh transport lands.
- **NATS** — broker image present; no native NATS source yet. Reserved for future
  transport work.

Dockerfiles: `docker/testing-nats.Dockerfile`, `docker/testing-zenoh.Dockerfile`.

---

## Clean teardown (avoid orphaned ports / containers)

```bash
make kill            # dev processes (frees 4780/4790)
docker rm -f nexus-test-mqtt nexus-test-nats nexus-test-zenoh 2>/dev/null
make brokers-stop    # equivalent for the make-managed brokers
make db-stop         # stops + removes nexus-dev-pg
```

If a port is still held: `make kill` first; it reaps the dev process group.
