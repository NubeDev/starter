# Evidence Capture — The Standard Bundle

> When a ✅ check fails, capture this **before** theorizing. A consistent bundle
> makes triage mechanical and lets a fresh AI session reason without re-running.

Output dir: `testing/.evidence/<scenario>/<timestamp>/` (git-ignored). Use one dir
per failure so before/after fixes are comparable.

---

## What goes in the bundle

| Artifact | How | Why |
|----------|-----|-----|
| `symptom.md` | one paragraph: which doc, which step, expected vs actual | the question being answered |
| `backend.log` | the `nexus-api` stdout/stderr around the failure | the primary signal |
| `flows.json` | `GET /api/v1/flows` (jq) | ingest health: metrics + errors |
| `query_count.json` | the `count(*)` query result | did data land |
| `pg_state.txt` | row counts per table **with the right tenant set** | RLS-aware ground truth |
| `request.txt` | the exact curl that failed + full response (status + body) | reproduction |
| `env.txt` | `make`-relevant env vars (redact keys) + git commit | what was running |
| `openapi_slice.json` | the relevant path from `backend/openapi.json` | contract vs reality |

---

## One-shot capture (adapt to your shell)

```bash
SCN=${1:-adhoc}; TS=$(date +%Y%m%d-%H%M%S)
DIR=testing/.evidence/$SCN/$TS; mkdir -p "$DIR"
BASE=${BASE:-http://127.0.0.1:4780}

# context
( git -C . rev-parse HEAD; echo "---"; env | grep -E '^NEXUS_|^ADMIN_|^BE_|^DB_' \
  | sed 's/\(KEY=\).*/\1<redacted>/' ) > "$DIR/env.txt"

# api state
curl -s $BASE/api/v1/flows -H "authorization: Bearer $TOKEN" | jq > "$DIR/flows.json"
curl -s $BASE/api/v1/query -X POST -H "authorization: Bearer $TOKEN" \
  -H content-type:application/json \
  -d '{"sql":"SELECT count(*) FROM '"${TABLE:-telemetry_raw}"'"}' | jq > "$DIR/query_count.json"

# postgres ground truth WITH the tenant set (RLS!)
docker exec nexus-dev-pg psql -U nexus -d nexus -c \
  "SET app.tenant_id = '${TENANT:-*}'; SELECT count(*) FROM ${TABLE:-telemetry_raw};" \
  > "$DIR/pg_state.txt" 2>&1

echo "bundle → $DIR"
```

Then drop in `backend.log` (copy the relevant window), `request.txt` (the failing
curl + response), and write `symptom.md`.

> A reusable script can live at `testing/scripts/capture.sh` — create it from the
> above when you first run the loop, and reference it here.

---

## Quality bar for the bundle

- The failing request is **reproducible** from `request.txt` alone.
- `pg_state.txt` was taken **with `app.tenant_id` set** — an unset/wrong tenant
  makes RLS hide rows and sends triage down the wrong path.
- `backend.log` includes the error *and* the few lines before it (the cause is
  usually upstream of the stack trace).
- The git commit is recorded — a fix is meaningless without knowing the baseline.
