# Contributing

This document collects workspace-wide rules that don't belong to any
single crate. Crate-specific guidance lives in the crate's `README.md`
or its module docs.

## Logs are always canonical SI

Every starter crate stores and **logs** physical quantities in their
canonical SI unit (°C, kPa, m/s, m, kg, …). Unit conversion happens
**exclusively at the response edge** — typed serialisers call
`UnitsCtx::convert` (Phase 2 of `DOCS/user/scope/SCOPE.md`, R6) when
emitting a value into a wire response body. The middleware never
rewrites bodies; handlers never log the converted value.

Concretely:

- **Do** log `temperature_c = 23.0` (canonical Celsius, name carries
  the unit so a reader can decode without context).
- **Don't** log `"temperature = 73.4 °F"` or interpolate a converted
  value the handler is about to write to the response body.
- **Don't** log `"pressure = 14.7 psi"`, `"speed = 65 mph"`, or
  `"mass = 5 lb"` — same reason.

### Why it matters

Logs feed observability pipelines that fan out to dashboards, alerts,
and downstream consumers — every one of those consumers has to be
able to compare values across hosts, accounts, and time zones. If two
hosts log the same sensor reading in different units (because the
operators have different preferences) the comparison breaks silently
and aggregation lies.

The "store canonical, convert at the edge" rule keeps the
observability surface stable: there is exactly one unit per quantity
in the log stream, baked into the column name, regardless of which
operator's preferences the response body was rendered with.

### The audit

`crates/starter-server/tests/canonical_logs.rs` installs a capturing
`tracing` subscriber, drives the Accept-Units middleware in both
`preferred` and `canonical` modes, and asserts that no captured log
line contains the substrings `"°F"`, `" psi"`, `" mph"`, or `" lb"`.
This is the SCOPE Smoke-tests "Canonical-only logs" test
(`DOCS/user/scope/SCOPE.md`).

Run it:

```bash
cargo test -p starter-server --test canonical_logs
```

If you add a new starter crate that emits unit-bearing log lines,
extend the harness's request battery there so the audit covers your
crate too.

### Reviewing PRs

When reviewing code that touches handlers, middleware, or any module
that does unit conversion, watch for:

- `tracing::*!("…{}…", converted_value)` after a `UnitsCtx::convert`
  call — that's the canonical-rule violation.
- Logging fields named like `temp_f`, `pressure_psi`, `speed_mph` —
  the unit suffix in the name means the value is non-canonical.
- Format strings that hard-code a unit symbol (`"°F"`, `"psi"`,
  `"mph"`, `"lb"`) — these belong only in response serialisation
  paths, never in log macros.

If you're unsure whether a value is canonical, the safe default is to
log it with an explicit canonical-unit field name (`temperature_c`,
`pressure_kpa`, `length_m`) and let the canonical_logs audit confirm.
