# `com.rubix.example.customer_quality`

Per-row data-quality detector for the `customers` table.

Contributed to the cleaner's `RuleRegistry` via
`contributes.anomaly_rules[]` and wrapped by the host's
`ToolAnomalyRule` adapter so it plugs into the same pipeline as the
in-process `builtin.nan` / `builtin.spike` / `builtin.stuck`
detectors (Phase B5).

Rules applied to `row`:

| condition                                                | outcome | quality       | note                              |
|----------------------------------------------------------|---------|---------------|-----------------------------------|
| `email` missing / empty                                  | `flag`  | `MissingEmail`| `customer_id=<id>`                |
| `email` lacks `@`                                        | `flag`  | `InvalidEmail`| `email=<value>`                   |
| `country` missing / empty                                | `flag`  | `MissingCountry` | `customer_id=<id>`             |
| `subscription_date` parses outside `2000-01-01..today`   | `flag`  | `BadDate`     | `subscription_date=<value>`       |
| otherwise                                                | `ok`    | —             | —                                 |

The cleaner short-circuits any builtin flag (NaN/Spike/Stuck) before
reaching this rule, so a row this rule sees has already passed the
numeric-quality gates. A misbehaving rule degrades to `ok` by design —
see `rubix-tools/src/cleaner/adapter.rs` for the contract.
