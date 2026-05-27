# `com.rubix.example` — reference rubix extension

End-to-end demo of the
[extension-north-star](../../docs/scope/extensions-north-star/README.md)
contributions an extension can declare against `rubix-agent`. Built
on top of the upstream `starter-ext-sdk` crate — per SCOPE R8 it is
the **only** rubix surface this extension depends on.

What this block contributes:

| surface                                     | what it demonstrates                                                                                                     |
|--------------------------------------------|--------------------------------------------------------------------------------------------------------------------------|
| `contributes.tools[]`                       | three MCP-style tools: `echo` (smoke probe), `csv_ingest`, `customer_quality`                                            |
| `contributes.warehouse_tables[]`            | two extension-owned warehouse tables (`customers`, `products`) shaped after the [datablist sample CSVs](https://github.com/datablist/sample-csv-files) |
| `contributes.warehouse_templates[]`         | two named warehouse-read templates (`customers_by_country`, `products_low_stock`) with audit-only SQL bodies             |
| `contributes.anomaly_rules[]`               | wires the `customer_quality` tool into the cleaner's `RuleRegistry` after the three builtins (NaN → Spike → Stuck)       |
| `contributes.ui[]` (Module Federation)      | a `Main` panel rendered into `<ExtensionSlot id="main">` on the rubix-frontend `/extensions` route, **plus** a compact `Sidebar` panel mounted into `<ExtensionSlot id="sidebar">` inside the rubix `AppSidebar` |
| `contributes.skills[]`, `contributes.flows[]`, `contributes.nodes[]` | the original reference layout (one quarantined `SKILL.md`, one flow, one flow node-kind) — kept from the previous demo |

The host:

1. Loads `block.yaml` through `starter-ext-host::Loader`
   (scan → validate_all → commit → seal).
2. Issues `CREATE TABLE IF NOT EXISTS com_rubix_example__customers`
   / `__products` at boot via `boot/extension_tables.rs`, prepending
   a `tenant_id String` column at position 0.
3. Folds every `contributes.warehouse_templates[]` entry into the
   host's `TemplateRegistry` (loading the JSON Schema from
   `params_schema` and capturing the SQL body from `sql_file` for
   audit; R7 — the SQL is never templated at runtime).
4. Walks every `contributes.anomaly_rules[]` entry, looks the
   `tool_id` up against the host's tool registry, wraps the
   dispatch in `ToolAnomalyRule`, and appends it to the cleaner's
   `RuleRegistry` after the three builtins.
5. Composes the extension into the live `NodeKindRegistry` via the
   upstream `starter-ext-flow` adapter.

## Layout

```
com.rubix.example/
├── block.yaml                       contributions declaration (tools,
│                                    warehouse_tables, warehouse_templates,
│                                    anomaly_rules, skills, flows, nodes, ui)
├── data/
│   ├── customers-sample.csv         30-row slice of datablist customers-100.csv
│   └── products-sample.csv          20-row slice of datablist products-100.csv
├── kinds/
│   ├── echo*                        original echo contribution (in/out/md/node)
│   ├── csv_ingest_in.json           input  schema for csv_ingest
│   ├── csv_ingest_out.json          output schema for csv_ingest
│   ├── csv_ingest.md                description loaded by the tool registry
│   ├── customer_quality_in.json     `{ row, window_tail }` cleaner adapter shape
│   ├── customer_quality_out.json    `{ outcome, quality?, note? }` RuleOutcome
│   ├── customer_quality.md          rule documentation
│   ├── customers_by_country_params.json   `params` schema for the template
│   ├── customers_by_country.sql           audit-only SQL body
│   ├── products_low_stock_params.json     `params` schema
│   └── products_low_stock.sql             audit-only SQL body
├── process/src/main.rs              extension binary — three handlers
│                                    (`echo`, `csv_ingest`, `customer_quality`)
│                                    plus pure `evaluate_customer_quality`
│                                    body with 6 unit tests.
├── skills/
│   └── example-skill/SKILL.md       quarantined skill (DOCS/agent/SKILLS.md R-skills-3)
├── flows/
│   └── example-assistant.yaml       extension-shipped flow
└── ui/
    ├── main.tsx                     developer-facing TSX source for the panel
    └── remoteEntry.js               hand-authored MF bundle (no transpile step)
                                     served by starter-ext-server at
                                     /extensions/com.rubix.example/ui/*
```

## Datasets

The CSV samples ship in-repo so the demo works offline. Schemas
follow the upstream
[datablist/sample-csv-files](https://github.com/datablist/sample-csv-files)
generator — `customers-sample.csv` is a 30-row slice of
`customers-100.csv` (Index, Customer Id, First Name, Last Name,
Company, City, Country, Phone 1, Phone 2, Email, Subscription Date,
Website); `products-sample.csv` is a 20-row slice of
`products-100.csv` (Index, Name, Description, Brand, Category,
Price, Currency, Stock, EAN, Color, Size, Availability, Internal
ID). The warehouse columns declared in `block.yaml` are the lower-cased
subset the demo actually inserts.

The UI panel inlines a smaller (12-row / 10-row) slice with three
deliberately-bad customer rows so the data-quality rule preview has
something to highlight.

## Demoing it

```sh
# Build + install the extension binary into the bundle dir, then
# restart the agent so the loader rescans.
make all

# Ingest sample customer rows via the tool.
curl -sS -b /tmp/com.rubix.example.cookies \
  -H 'content-type: application/json' \
  -X POST http://127.0.0.1:8088/api/v1/tools/com.rubix.example.csv_ingest/call \
  -d @- <<'JSON'
{
  "dataset": "customers",
  "rows": [
    { "customer_id": "DD37Cf93aecA6Dc", "first_name": "Sheryl",
      "country": "Chile", "email": "zunigavanessa@smith.info",
      "subscription_date": "2020-08-24" }
  ]
}
JSON
```

The cleaner picks the contributed rule up on its next tick — see
`rubix-tools/src/cleaner/adapter.rs` for the dispatch contract and
`docs/scope/extensions-north-star/PROGRESS.md` for the rollout state.
