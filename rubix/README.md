# rubix — the backend product

Backend-only product built on `starter`. One binary, every transport
(REST + SSE + gRPC + MCP + CLI), Postgres + ClickHouse, six concrete
operator-facing goals:

1. Build dashboards
2. Manage users + teams + tenants
3. Program flows
4. Write ClickHouse rules
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

## How rubix uses starter

Rubix consumes ~28 `starter-*` crates listed in
[SCOPE.md §"Starter crates rubix consumes"](./SCOPE.md). The
upstream-first rule (R2) is load-bearing: if a capability is
missing in starter, the fix is in starter, not a parallel rubix
crate. The committed upstream PR list lives in
[docs/design/STARTER-CHANGES.md](./docs/design/STARTER-CHANGES.md).

## Non-goals (this scope)

- **No frontend.** A future UI is a *client* of this backend.
- **No second agent runtime.** Starter's `ai-agent` node kind is
  the agent (see starter's `DOCS/agent/SCOPE.md`).
- **No SQLite.** Postgres only.
- **No domain verticals** (devices, drivers, schedules, alarms).
  The six goals above are the scope; verticals can ship as
  extensions later.
