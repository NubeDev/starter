# `ch-explorer-example`

End-to-end demo for the ClickHouse explorer: serves the SPA bundle
from [`examples/ch-explorer/ui/dist`](./ui/) under `/warehouse/explorer/*`
and the read-only API from `starter-warehouse::explorer` under
`/api/warehouse/ch/*`.

The SPA is a thin Vite host (`examples/ch-explorer/ui/`) that wraps
the headless [`@nube/starter-ui-ch-explorer`](../../packages/starter-ui-ch-explorer/)
React components in a `QueryClient` + `StarterClient` + tailwind
shell. The same headless library is the primary explorer surface
inside the rubix admin shell at `/admin/warehouse` → Explorer tab
(see `rubix/docs/design/warehouse/explorer/README.md`). This demo
binary keeps it reachable for non-rubix deployments.

## Prerequisites

1. A reachable ClickHouse — anything that speaks the HTTP interface.
   The defaults assume `http://127.0.0.1:8123` with the `default`
   database and the `default` anonymous user (matches the
   `clickhouse/clickhouse-server` image and `mani run dev-deps`).
2. The SPA bundle built once:

   ```bash
   pnpm install
   pnpm -F ch-explorer-demo-ui build
   ```

## Run

```bash
# (optional) load a small demo fixture so the explorer is non-empty
cargo run -p ch-explorer-example -- seed

# serve UI + API on http://127.0.0.1:3030/warehouse/explorer
cargo run -p ch-explorer-example -- serve
```

Then open <http://127.0.0.1:3030/warehouse/explorer>. You should see:

- **Overview** — `size_on_disk` plus the seeded `demo_*` tables;
- **Tables** — paginated rows from `demo_samples`;
- **Schema** — an ERD for the three demo tables;
- **Query** — Monaco editor: `SELECT 1` returns one row; a
  `DROP TABLE demo_samples` is refused with `400` and
  `{"error":"read_only_violation"}`;
- **Autocomplete** — table + column suggestions populate on first
  focus.

## Environment

| Var | Default | Notes |
|---|---|---|
| `CH_EXPLORER_URL` | `http://127.0.0.1:8123` | ClickHouse HTTP endpoint |
| `CH_EXPLORER_DATABASE` | `default` | Database the explorer queries (`currentDatabase()`) |
| `CH_EXPLORER_USER` | `default` | CH user |
| `CH_EXPLORER_PASSWORD` | *(empty)* | CH password |
| `CH_EXPLORER_BIND` | `127.0.0.1:3030` | HTTP bind for `serve` |
| `CH_EXPLORER_DIST` | `examples/ch-explorer/ui/dist` | Built SPA bundle |
| `RUST_LOG` | `info` | Tracing filter |

## Auth

This example is intentionally **anonymous**: it injects a fixed
`Role::Admin` principal via
[`starter_server::auth::with_anonymous_principal`] and an
[`AllowAll`] policy engine so every `/api/warehouse/ch/*` request
satisfies the `("warehouse", "read")` gate without a login. That
is fine for a single-operator demo on `localhost`, **not** for any
deployment that should distinguish users.

Production binaries should pair `with_principal(authenticator)` (a
real `Authenticator` — `starter-auth-users`, `starter-auth-token`,
`starter-auth-oauth`, …) with a `DbPolicyEngine` and grant the
`("warehouse", "read")` permission to the relevant role. See
`examples/authz-demo/src/server.rs` for the full shape.

## Vite dev proxy

While iterating on the UI, run the explorer binary on `:3030` and
the Vite dev server on `:5173`:

```bash
# terminal 1
cargo run -p ch-explorer-example -- serve

# terminal 2
pnpm -F ch-explorer-demo-ui dev
```

Vite proxies `/api/warehouse/ch/*` → `CH_EXPLORER_API_TARGET`
(default `http://localhost:3030`), so the SPA at
<http://localhost:5173> hits the Rust binary's API surface.

## Sample data

The `seed` subcommand creates and populates `demo_buildings`,
`demo_meters`, and `demo_samples` (24 rows × 4 meters) using
`DROP TABLE IF EXISTS` first so re-running the seed is idempotent.
The fixture is intentionally tiny — it's there so the explorer
isn't an empty database on first boot, not as a benchmark dataset.

`make warehouse-seed` (in `rubix/Makefile`) and
`mani run warehouse-seed` (in `rubix/mani.yaml`) both shell out to
the same subcommand.
