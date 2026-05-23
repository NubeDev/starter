# OVERVIEW — repo map + dependency arrow

This doc is the single map of the rubix tree. Read it before
[SCOPE.md](../../SCOPE.md) if you want the picture; read SCOPE first
if you want the rules.

## The shape

`rubix` is a backend product on top of `starter`. Six rubix crates,
one binary, every transport, no frontend.

```
starter/
├── crates/                      ~60 starter-* crates (the platform)
├── starter-extensions/          sibling workspace; the extension host
└── rubix/                       ← THIS TREE
    ├── SCOPE.md                 the load-bearing rules (R1–R13)
    ├── Cargo.toml               workspace members + dep aliases
    ├── mani.yaml                build/test/run tasks
    │
    ├── crates/
    │   ├── rubix-spi            contracts + descriptors + events
    │   ├── rubix-tools          impl starter_spi::Tool for the six goals
    │   ├── rubix-skills         SKILL.md bundles via include_dir!
    │   ├── rubix-flows          flow YAML bundles via include_dir!
    │   ├── rubix-client         thin extension of starter-client-rs
    │   └── rubix-agent          THE BINARY — wires everything
    │
    ├── extensions/
    │   └── com.rubix.example/   reference block (Phase 5 placeholder)
    │
    └── docs/
        ├── design/              authoritative architecture docs
        ├── sessions/            working notes, not design
        └── testing/             walkthroughs
```

## Dependency arrow (Rust)

```
starter-spi
   ↑
rubix-spi
   ↑
   ├── rubix-client                (HTTP client; zero agent dep)
   ├── rubix-tools                 (impl Tool for the rubix actions)
   ├── rubix-skills                (SKILL.md bundles via include_dir)
   ├── rubix-flows                 (flow YAML bundles via include_dir)
   │       │
   │       └── consumed by ──┐
   │                          ▼
   │                    rubix-agent  (the binary)
   │
   └── starter-*  (via cargo features the binary chooses)
```

Never the other way. See [SCOPE R5](../../SCOPE.md#r5).

## The six goals

The rubix backend exists for six concrete operator-facing goals:

1. **Build dashboards** — `rubix-tools::dashboard` + SDUI
2. **Manage users** — `rubix-tools::user` + `starter-auth-*`
3. **Program flows** — `rubix-tools::flow_ops` + `starter-flow`
4. **Write ClickHouse rules** — `rubix-tools::clickhouse`
5. **Background system checks** — `rubix-tools::system`
6. **Analytics + reports** — `rubix-tools::analytics`

Each goal ships a bundled flow rooted at an `ai-agent` node, a
matching SKILL.md, an MCP prompt + resource, and Rust tool impls.
By the end of Phase 4 all six work end-to-end over MCP.

## Where to read next

- [SCOPE.md](../../SCOPE.md) — the rules and phases.
- starter's [DOCS/agent/SCOPE.md](../../../DOCS/agent/SCOPE.md) — the
  authoritative `ai-agent` node-kind spec.
- [STARTER-CHANGES.md](./STARTER-CHANGES.md) — the upstream PR list.
