# products CRUD

Three thin proxy tools the bundled UI calls to manage rows in
`com_rubix_example__products` through the host's
`WarehouseWriteHandle`:

| tool                                 | op     | wire                            |
| ------------------------------------ | ------ | ------------------------------- |
| `com.rubix.example.products_create`  | INSERT | `{ row: { ... } }`              |
| `com.rubix.example.products_update`  | UPDATE | `{ row: { internal_id, ... } }` |
| `com.rubix.example.products_delete`  | DELETE | `{ internal_ids: ["...", ...] }`|

All three return `{ operation, affected }`. The host stamps
`tenant_id` from the caller before issuing SQL — extensions cannot
spoof cross-tenant writes. Reads use the
`com.rubix.example.products_list` template via the existing
`warehouse_query` tool.
