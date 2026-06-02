# `com.nubeio.ce.warehouse_query`

Browser-facing proxy over `ctx.warehouse_read().query(template, params)`.
Refuses any template outside the `com.nubeio.ce.*` namespace. Backs the
device-list and wiresheet panels.
