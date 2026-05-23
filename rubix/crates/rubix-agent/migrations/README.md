# rubix-agent migrations

Rubix-specific Postgres migrations, run **after** starter crate
migrations at boot. Registered with the namespaced runner from
`starter-store-postgres` under `source = "rubix"`.

Layout (planned):

```
migrations/
├── 0001_init/
│   ├── up.sql
│   └── down.sql
├── 0002_<concept>/
│   ├── up.sql
│   └── down.sql
└── ...
```

See [docs/design/migrations/](../../docs/design/migrations/README.md)
for the boot ordering rules and the "never cross-tree FK" rule.
