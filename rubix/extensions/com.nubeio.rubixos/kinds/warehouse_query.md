# `com.nubeio.rubixos.warehouse_query`

Thin wrapper around `ctx.warehouse_read().query(template, params)`
so the federated UI panel — served at `/extensions/com.nubeio.rubixos`
— can render real data out of the shared Rubix warehouse without
smuggling a SQL gateway through the extension SPI.

Refuses anything outside `com.nubeio.rubixos.*`. The host still
applies the usual gates:

- the template must exist in `TemplateRegistry`,
- the template's tables must be inside
  `capabilities.warehouse_read.tables`,
- `$caller_tenant_id` is bound from the operator session — the caller
  cannot spoof a different tenant.
