# Quickstart — Stack Up, Data Flowing, in One Pass

> Verified: nexus-rewrite tip on 2026-06-10. Commands assume CWD `nexus/`.

Goal: from a clean machine to **rows landing in Postgres from generated
telemetry**, with the API healthy and a token in hand. ~5 minutes. Every step has
a ✅ check — do not proceed past a failed check; go to
[../feedback-loop/TRIAGE.md](../feedback-loop/TRIAGE.md).

Ports (project 47xx block): DB `4770`, API `4780`, UI `4790`, MQTT `1883`,
NATS `4222`, Zenoh `7447`. All overridable — see [STACK.md](STACK.md).

---

## 0. Prereqs

- Docker, Rust toolchain, `pnpm`, `curl`, `jq`.

✅ `docker ps` works, `cargo --version` prints, `jq --version` prints.

---

## 1. Database + admin seed

```bash
make db          # starts dev Postgres (container nexus-dev-pg) on :4770
make seed        # applies migrations, creates admin tenant+user+grants
```

Admin creds default to `admin@nexus.local` / `change-me-admin`
(`ADMIN_EMAIL` / `ADMIN_PASSWORD`).

✅ `docker ps` shows `nexus-dev-pg` healthy.
✅ `make seed` exits 0 and logs the admin user created/refreshed.

---

## 2. Backend

This Quickstart uses the **verified Zenoh path**. The `zenoh` ingest source is
behind a cargo feature, so build the backend with it on:

```bash
make dev-be FEATURES=zenoh    # cargo run --bin nexus-api --features zenoh, binds :4780
```

(Leave it running; use another terminal for the rest. `make dev` also starts the
UI on :4790. Without `FEATURES=zenoh` the `zenoh` source won't be registered and
the flow start in step 5 fails.)

```bash
export BASE=http://127.0.0.1:4780
curl -s $BASE/health        # → {"status":"ok","version":"0.1.0",…}
```

✅ API answers on `:4780`. Backend log shows migrations applied + `listening`.

---

## 3. Log in (cookie jar + CSRF)

Auth is a **session cookie + CSRF token**, not a bearer (see
[../reference/API_CHEATSHEET.md](../reference/API_CHEATSHEET.md)):

```bash
export JAR=$(mktemp)
login=$(curl -s -c "$JAR" -X POST $BASE/auth/login -H content-type:application/json \
  -d '{"email":"admin@nexus.local","password":"change-me-admin"}')
csrf=$(printf '%s' "$login" | sed -n 's/.*"csrf_token":"\([^"]*\)".*/\1/p')
post() { curl -s -b "$JAR" -X POST "$BASE$1" -H content-type:application/json -H "X-CSRF-Token: $csrf" -d "$2"; }
```

✅ `$csrf` is non-empty; `curl -s -b "$JAR" $BASE/api/v1/me | jq` returns your
subject + `"tenant_id":"nexus"` + `"role":"admin"`.

---

## 4. Zenoh router + target table

```bash
make zenoh-install            # router on tcp/127.0.0.1:7447
```

The postgres sink binds JSON strings as `text`, so make the `timestamp` column
`text` (see FLOWS_MQTT_INGEST "Known issues"):

```bash
docker exec nexus-dev-pg psql -U nexus -d nexus -c '
CREATE TABLE IF NOT EXISTS telemetry_raw (
  tenant_id text, site_id text, host_uuid text, point_uuid text, meter_id text,
  kind text, secondary_tag text, value double precision, unit text, "timestamp" text);'
```

✅ Zenoh container up (`nc -z 127.0.0.1 7447`); table exists.

---

## 5. Create + start the ingest flow

```bash
cat > /tmp/flow-zenoh.json <<'JSON'
{ "name":"zenoh-telemetry-ingest",
  "input":{"type":"zenoh","endpoints":["tcp/127.0.0.1:7447"],"key_expr":"rubix/testing/**","mode":"client"},
  "pipeline":[{"type":"json_to_arrow"}],
  "output":{"type":"postgres","uri":"postgres://nexus:nexus@127.0.0.1:4770/nexus","table":"telemetry_raw"},
  "enabled":false }
JSON
FLOW=$(post /api/v1/flows "$(cat /tmp/flow-zenoh.json)" | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
post /api/v1/flows/$FLOW/start '' | python3 -c 'import sys,json;d=json.load(sys.stdin);print("running:",d["running"],"err:",d["metrics"]["last_error"])'
```

✅ Prints `running: True err: None`. (Confirm the `zenoh` source exists first:
`curl -s -b "$JAR" $BASE/api/v1/flows/node-types | grep -o zenoh`.)

---

## 6. Pump data

```bash
cd testing/datapump && cargo build
./target/debug/nexus-datapump --transport zenoh --zenoh-endpoint tcp/127.0.0.1:7447 \
  --path-prefix rubix/testing --path-tenant all --tenant-id nexus \
  --sites 3 --meters-per-kind 4 --interval-ms 100 --count 200 --seed 42
```

`--tenant-id nexus` matches the admin tenant so RLS doesn't hide the rows.

✅ datapump exits 0. Flow metrics `batches_in` / `rows_written` climb.

---

## 7. Confirm rows landed

```bash
curl -s -b "$JAR" -X POST $BASE/api/v1/query -H content-type:application/json \
  -H "X-CSRF-Token: $csrf" -d '{"sql":"SELECT count(*) AS n FROM telemetry_raw"}'
# → {"columns":[…],"rows":[{"n":200}],…}

curl -s -b "$JAR" $BASE/api/v1/flows/$FLOW \
  | python3 -c 'import sys,json;m=json.load(sys.stdin)["metrics"];print("batches_in",m["batches_in"],"rows_written",m["rows_written"],"write_errors",m["write_errors"])'
# → batches_in 200 rows_written 200 write_errors 0
```

✅ Count equals `--count` (200); `write_errors: 0`.

**If the count is 0 but `rows_written` > 0:** RLS tenant mismatch. See
[../feedback-loop/TRIAGE.md](../feedback-loop/TRIAGE.md) → "Data written but
invisible".

---

## You now have a live stack

Next: open a feature runbook in [../features/](../features/), or run an
end-to-end script in [../scenarios/](../scenarios/).

## Teardown

```bash
make kill                       # frees dev ports / stops dev processes
docker rm -f nexus-test-mqtt    # + nexus-test-nats / nexus-test-zenoh if used
make db-stop                    # stops + removes dev Postgres
```
