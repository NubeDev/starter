# `com.rubix.example.warehouse_query`

Thin wrapper around `ctx.warehouse_read().query(template, params)` so
the extension's bundled UI panel — served at
`/extensions/com.rubix.example` — can render real data out of the
shared Rubix warehouse without smuggling a SQL gateway through the
extension SPI.

The tool refuses anything outside this extension's own contributed
templates (`com.rubix.example.*`). The host still applies the usual
gates:

- the template must exist in `TemplateRegistry`,
- the template's tables must be inside
  `capabilities.warehouse_read.tables`,
- `$caller_tenant_id` is bound from the operator session — the caller
  cannot spoof a different tenant.

Wire shape:

```json
{ "template": "com.rubix.example.customers_by_country",
  "params":   { "limit": 10, "min_count": 1 } }
```

Response:

```json
{ "template": "com.rubix.example.customers_by_country",
  "rows":     [ { "country": "Chile", "customer_count": 2 }, ... ],
  "count":    7 }
```
