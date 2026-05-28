# Pin CRUD tools

- `pin_create` — INSERT one pin row. `pin_id` (operator-supplied) is
  required.
- `pin_update` — UPDATE one row by `pin_id`.
- `pin_delete` — DELETE one or more pins by `pin_id`.

All three go through `ctx.warehouse_write()`; the host stamps
`tenant_id` from the caller session (R6) before issuing the SQL.

Wire shape for create/update is `{ "row": { "pin_id": ..., ... } }`.
Delete takes `{ "pin_ids": ["...", ...] }`.
