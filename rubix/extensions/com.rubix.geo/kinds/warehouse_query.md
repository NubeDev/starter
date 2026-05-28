# `com.rubix.geo.warehouse_query`

Thin proxy over `ctx.warehouse_read().query(template, params)`.
Refuses any template name outside `com.rubix.geo.*` — the host would
also refuse a cross-extension template via the grant gate, but the
pre-check is a friendlier error.

The UI cannot reach `WarehouseReadHandle` directly from the browser,
so it calls `/api/v1/tools/com.rubix.geo.warehouse_query` instead.
