# com.rubix.example — extension build + lifecycle + demo seeders.
#
# The canonical lifecycle targets (build / pack / install / reload /
# uninstall / test / status / data-root / cleanup-preview) come from
# ../extension.mk. See `docs/scope/extensions/installed-only-model.md`.

EXT_ID    := com.rubix.example
BIN_NAME  := rubix-example-extension
HAS_UI    := 1

include ../extension.mk

# ---------------------------------------------------------------------------
# Demo-ingest — load the bundled datablist CSV samples into the warehouse
# via the `com.rubix.example.csv_ingest` tool. Idempotent at the warehouse
# level (MergeTree dedup by `(tenant_id, customer_id|internal_id)`); safe to
# re-run.
#
# Why python: parsing CSV in pure bash is a footgun (quoted fields, commas
# inside values). The bundled csv module is in every distro's python3.
# Why batches of 25: keeps the request payload tiny and lets a row-level
# validation error name a specific batch.
# ---------------------------------------------------------------------------
DATA_DIR       := $(BUNDLE_DIR)/data
CUSTOMERS_CSV  := $(DATA_DIR)/customers-sample.csv
PRODUCTS_CSV   := $(DATA_DIR)/products-sample.csv
BATCH_SIZE     ?= 25

.PHONY: demo-ingest demo-ingest-customers demo-ingest-products demo-query

demo-ingest: demo-ingest-customers demo-ingest-products

demo-ingest-customers: login
	@command -v python3 >/dev/null || { echo "demo-ingest needs python3"; exit 1; }
	@test -f $(CUSTOMERS_CSV) || { echo "missing $(CUSTOMERS_CSV)"; exit 1; }
	@echo "==> POST $(AGENT_BASE)/api/v1/tools/$(EXT_ID).csv_ingest  (dataset=customers, file=$(notdir $(CUSTOMERS_CSV)))"
	@python3 -c "$$CSV_TO_JSON_PY" customers $(CUSTOMERS_CSV) $(BATCH_SIZE) \
	  | while IFS= read -r batch; do \
	        resp=$$(curl -sS -b $(COOKIE_JAR) -H 'content-type: application/json' \
	            -X POST $(AGENT_BASE)/api/v1/tools/$(EXT_ID).csv_ingest \
	            -d "$$batch"); \
	        echo "    $$resp"; \
	    done

demo-ingest-products: login
	@command -v python3 >/dev/null || { echo "demo-ingest needs python3"; exit 1; }
	@test -f $(PRODUCTS_CSV) || { echo "missing $(PRODUCTS_CSV)"; exit 1; }
	@echo "==> POST $(AGENT_BASE)/api/v1/tools/$(EXT_ID).csv_ingest  (dataset=products, file=$(notdir $(PRODUCTS_CSV)))"
	@python3 -c "$$CSV_TO_JSON_PY" products $(PRODUCTS_CSV) $(BATCH_SIZE) \
	  | while IFS= read -r batch; do \
	        resp=$$(curl -sS -b $(COOKIE_JAR) -H 'content-type: application/json' \
	            -X POST $(AGENT_BASE)/api/v1/tools/$(EXT_ID).csv_ingest \
	            -d "$$batch"); \
	        echo "    $$resp"; \
	    done

demo-query: login
	@echo "==> GET  $(AGENT_BASE)/api/v1/extensions/$(EXT_ID)  (contributions only)"
	@curl -sS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq '{warehouse_tables: (.manifest.contributes.warehouse_tables // [] | map(.name)), warehouse_templates: (.manifest.contributes.warehouse_templates // [] | map(.name)), anomaly_rules: (.manifest.contributes.anomaly_rules // [] | map(.id))}'

# Tiny python shim — reads a CSV, emits one JSON envelope per BATCH_SIZE rows
# to stdout. Each envelope is `{ dataset, rows: [{ <lower_snake_col>: <val> }] }`
# so it lines up with the warehouse_tables[] schema declared in block.yaml.
# Numeric columns (price, stock) are coerced; everything else stays a string.
define CSV_TO_JSON_PY
import csv, json, sys
dataset, path, batch_size = sys.argv[1], sys.argv[2], int(sys.argv[3])
NUM_FIELDS = {"price": float, "stock": int}
MAPPINGS = {
    "customers": {
        "Customer Id":       "customer_id",
        "First Name":        "first_name",
        "Last Name":         "last_name",
        "Company":           "company",
        "City":              "city",
        "Country":           "country",
        "Email":             "email",
        "Subscription Date": "subscription_date",
        "Website":           "website",
    },
    "products": {
        "Internal ID":  "internal_id",
        "Name":         "name",
        "Brand":        "brand",
        "Category":     "category",
        "Price":        "price",
        "Currency":     "currency",
        "Stock":        "stock",
        "Availability": "availability",
        "Color":        "color",
        "Size":         "size",
    },
}
map = MAPPINGS[dataset]
rows = []
with open(path, newline="") as f:
    for r in csv.DictReader(f):
        out = {}
        for src, dst in map.items():
            v = r.get(src, "")
            if dst in NUM_FIELDS and v != "":
                try: v = NUM_FIELDS[dst](v)
                except ValueError: pass
            out[dst] = v
        rows.append(out)
        if len(rows) >= batch_size:
            print(json.dumps({"dataset": dataset, "rows": rows}))
            rows = []
if rows:
    print(json.dumps({"dataset": dataset, "rows": rows}))
endef
export CSV_TO_JSON_PY
