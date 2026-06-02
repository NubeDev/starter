# rubix — the backend product

Backend-only product built on `starter`. One binary, every transport
(REST + SSE + gRPC + MCP + CLI), Postgres + TimescaleDB, six concrete
operator-facing goals:

1. Build dashboards
2. Manage users + teams + tenants
3. Program flows
4. Write warehouse rules
5. Background system checks
6. Analytics + reports

**Read first:** [SCOPE.md](./SCOPE.md). Every rule (R1–R13), every
phase, every smoke test. Then [docs/design/OVERVIEW.md](./docs/design/OVERVIEW.md)
for the repo map.

## Layout

```
rubix/
├── SCOPE.md                          load-bearing rules + phases
├── Cargo.toml                        workspace members
├── mani.yaml                         build / test / run tasks
│
├── crates/
│   ├── rubix-spi                     contracts: DTOs, descriptors, events
│   ├── rubix-tools                   impl Tool for the six goals
│   ├── rubix-skills                  bundled SKILL.md (include_dir!)
│   ├── rubix-flows                   bundled flow YAML (include_dir!)
│   ├── rubix-client                  HTTP client for the agent
│   └── rubix-agent                   THE BINARY
│
├── extensions/
│   └── com.rubix.example/            reference block layout
│
└── docs/
    ├── design/                       15 authoritative architecture docs
    │                                 (STARTER-CHANGES.md is the upstream PR list)
    ├── sessions/                     working notes — NOT design
    └── testing/                      walkthroughs
```

## Build (Phase 0)

```bash
# from the starter workspace root
mani run build --all                  # or:
cargo build -p rubix-agent

# boot the Phase 0 skeleton
cargo run -p rubix-agent
# in another shell:
curl -sf http://127.0.0.1:8080/healthz
```

## Local demo

The six-step thin-slice path (see
[docs/scope/THIN-SLICE.md §Success criterion](./docs/scope/THIN-SLICE.md)).
On a fresh dev machine with only `docker` + `cargo` installed:

```bash
# One-line setup — brings up Postgres + TimescaleDB, seeds the
# bootstrap operator, then boots the rubix-agent against
# rubix/dev/agent.toml. Tear down later with `mani run dev-deps-down`.
mani run demo
```

Once the binary is up, walk the six smoke steps by hand:

```bash
# 1. Boot
mani run run

# 2. Log in as the bootstrap operator (default password from
#    `mani run bootstrap`; override with --password if you reseed).
curl -c cookies.txt -X POST http://127.0.0.1:8088/api/v1/auth/login \
     -d '{"email":"op@example.com","password":"rubix-dev-passwd"}'

# 3. Call the disk-check via REST with Spanish + Paris prefs
curl -b cookies.txt -H "Accept-Language: es-AR" \
     http://127.0.0.1:8088/api/v1/tools/rubix.system.disk
# → JSON with Diagnostic { code, params: { percent, free }, message_en }

# 4. Connect Claude Desktop to the MCP endpoint
# → "What's the disk situation?" returns Spanish prose with EU date format

# 5. Inspect the audit trail
psql -c "SELECT actor, action, kind FROM changelog ORDER BY at DESC LIMIT 5"

# 6. Inspect the history
clickhouse-client -q "SELECT * FROM system_disk_history ORDER BY at DESC LIMIT 5"
```

If all six steps work, the arch is real.

## How rubix uses starter

Rubix consumes ~28 `starter-*` crates listed in
[SCOPE.md §"Starter crates rubix consumes"](./SCOPE.md). The
upstream-first rule (R2) is load-bearing: if a capability is
missing in starter, the fix is in starter, not a parallel rubix
crate. The committed upstream PR list lives in
[docs/design/STARTER-CHANGES.md](./docs/design/STARTER-CHANGES.md).

## Extension supervisor: process-group lifecycle + orphan reaper

Process-flavour extensions run as child processes under
`starter-ext-supervisor`. To stop child (and grandchild) processes leaking
across agent restarts — the failure mode where a `SIGKILL`ed agent (e.g.
`make reload`, OOM) leaves orphaned `*-extension` processes reparented to
init — the supervisor:

- spawns every child in its **own process group** and tears the whole group
  down with `killpg` (SIGTERM → grace → SIGKILL), so a grandchild the
  extension forked dies with its parent (`kill_on_drop` alone only reaches
  the direct child);
- bounded-`wait()`s on every crash path so a child is actually reaped before
  the supervisor respawns it;
- writes each child's process-group id to a **pidfile** under
  `$RUBIX_DATA_ROOT/supervisor-pids/`, and on the next boot **reaps** any
  group still alive from a prior, hard-killed instance.

Operators see this on two surfaces:

- `GET /api/v1/extensions/overview` (and `/extensions/<id>/metrics`) carries
  `group_kills_total` per extension — a non-zero, rising value flags an
  extension that leaks descendants or ignores `SIGTERM`.
- `GET /api/v1/admin/supervisor/health` reports the boot reaper's results
  (which process groups were reclaimed at startup). The admin UI renders
  both at **Admin → Supervisor**.

## Non-goals (this scope)

- **No frontend.** A future UI is a *client* of this backend.
- **No second agent runtime.** Starter's `ai-agent` node kind is
  the agent (see starter's `DOCS/agent/SCOPE.md`).
- **No SQLite.** Postgres only.
- **No domain verticals** (devices, drivers, schedules, alarms).
  The six goals above are the scope; verticals can ship as
  extensions later.
