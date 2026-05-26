# Stage 1 — MartSpec audit

Generated as part of Stage 1 of
[`warehouse-engine-swap.md`](warehouse-engine-swap.md). For each
in-tree `MartSpec` we check three cagg constraints (proposal
§"The mart / continuous aggregate translation"):

1. **Single source hypertable.** No `UNION ALL`; one source table.
2. **No subqueries or CTEs in the `SELECT`** body.
3. **Late-arrival window** — does the mart need data older than
   the cagg `start_offset` (proposal default: 3 days)?

The current `MartSpec` shape (see
`crates/starter-warehouse/src/catalog/mart_spec.rs`) is already
narrow: a single `source_table`, one `time_bucket_secs`, a flat
`group_by` array, and an aggregation list. By construction it
cannot express a subquery or a CTE, so constraint (2) is
satisfied for every `MartSpec` the codebase can express today.
Constraint (1) is enforced by the schema. The remaining axis is
constraint (3) and aggregation-function support.

Verdict legend:

* **clean** — translates to a cagg verbatim in Stage 2.
* **reshaped** — modified in this commit so Stage 2 can adopt it.
* **blocked** — needs discussion before Stage 2 can land.

## Inventory

| Location | Source table | Aggregations | Late-arrival | Verdict |
|---|---|---|---|---|
| `crates/starter-warehouse/src/ddl/mart.rs` test fixture (`order_by_promotes_first_group_by`) | `samples` | `sum(value_num)` | n/a (unit test) | clean |
| `crates/starter-warehouse/src/ddl/dialect.rs` test fixture | `samples` | `sum(value_num)` | n/a (unit test) | clean |
| `crates/starter-warehouse/tests/with_stack.rs::sample_spec` | `samples` | `sum(value_num)` | n/a (integration fixture) | clean |
| `crates/starter-warehouse/tests/unit_invariants.rs::fixture` | `samples` | `sum(value_num)` | n/a (unit fixture) | clean |
| `examples/iot-anomaly-detector` — `mart_iot_1m` (60s bucket) | `samples` | `avg`, `stddevPop`, `count` over `value_num` | bucket = 60s, refresh cadence ≤ 1min; tolerates the default 3-day late-arrival window | clean (Stage 2 must wire `timescaledb_toolkit::stats_agg` for `stddevPop` — see Risks in the proposal) |
| `examples/iot-anomaly-detector` — `mart_iot_1h` (3600s bucket) | `samples` | `avg`, `stddevPop`, `count` over `value_num` | hourly buckets; default 3-day `start_offset` covers the realistic IoT replay latency | clean (same `stddevPop` note) |

## Findings

* **Zero blocked marts.** Every in-tree mart targets `samples`
  as its sole source, contains no subquery or CTE (the
  `MartSpec` schema cannot express either), and the
  aggregations are commutative / time-window safe.
* **`stddevPop` requires toolkit.** The IoT example uses
  ClickHouse's `stddevPop`; TimescaleDB has no built-in
  equivalent. Stage 2's `TimescaleDbDialect` will route this
  through `timescaledb_toolkit::stats_agg` and post-process the
  state with `stddev_pop(stats_agg)` at read time. Captured
  here so Stage 2 does not rediscover it at implementation time.
* **No `quantile` aggregations** present today, so the
  `percentile_cont` / toolkit path noted in the proposal is not
  load-bearing yet — but adding `quantile` post-Stage 2 will
  need the toolkit (already in the dev compose).
* **No `entities_dict` dependence in `MartSpec` definitions.**
  The `entities_dict` dictGetOrNull join lives in `mart.rs::read_query`
  rather than in any `MartSpec` body, so Stage 2 can replace the
  read-time join with a direct PG join without touching `MartSpec`
  authoring surface.

## Reshape log

None required. All in-tree marts translate as written; no
`MartSpec` was modified by this audit.
