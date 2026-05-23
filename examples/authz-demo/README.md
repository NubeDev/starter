# authz-demo

End-to-end demo of:

- **AuthN** via `starter-auth-users` (email/password users + API tokens).
- **AuthZ** via `starter-authz` — a DB-backed policy engine. Each
  protected route is wrapped with `with_permission(kind, action)`.
- **Per-user grant / revoke** through `Rule` rows with a
  `subject == "<user-id>"` condition. Deny-overrides means a revoke
  always wins over the matching role default.
- **Extension-contributed endpoints** subject to the same policy
  engine as built-in endpoints (`com.acme.weather` ships a manifest;
  the host mounts two REST routes from it and gates each one).

## Endpoints

| Path                  | Resource  | Action    | Comes from         |
|-----------------------|-----------|-----------|--------------------|
| `GET  /reports`       | `reports` | `read`    | server (built-in)  |
| `POST /reports`       | `reports` | `create`  | server (built-in)  |
| `GET  /weather/forecast` | `weather` | `read`    | `com.acme.weather` |
| `POST /weather/refresh`  | `weather` | `refresh` | `com.acme.weather` |

The built-in role defaults (loaded by `default_policy: true`) grant:

- `reader`  → `read` on every kind.
- `writer`  → `read` + `create` + `update` on every kind.
- `admin`   → everything.

`refresh` is intentionally **not** in the default action list — out of
the box no one can call `POST /weather/refresh`. The admin must grant
it explicitly. That's the demo.

## Walkthrough

```bash
# from repo root
cargo build -p starter-authz-demo
BIN=./target/debug/authz-demo
cd examples/authz-demo
rm -f authz-demo.db*

# 1. set up tables
$BIN migrate

# 2. create two users — alice (reader), bob (writer)
ALICE=$($BIN user create alice@acme.test alice-secret-pw1 --role reader)
BOB=$(  $BIN user create bob@acme.test   bob-secret-pw1   --role writer)

# 3. issue API tokens (printed once)
AT=$($BIN user token $ALICE)
BT=$($BIN user token $BOB)

# 4. start the server (any other port if 8080 is busy)
$BIN serve --http-bind 127.0.0.1:8090 &
sleep 1
H=http://127.0.0.1:8090
```

### Defaults in effect

```bash
curl -s -o /dev/null -w "%{http_code}\n"                                  $H/reports                 # 401 — no auth
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $AT"   $H/reports                 # 200 — alice can read
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $AT" \
  -H "Content-Type: application/json" -d '{"title":"x","body":"y"}'       $H/reports                 # 403 — readers cannot create
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $BT" \
  -H "Content-Type: application/json" -d '{"title":"q","body":"r"}'       $H/reports                 # 201 — bob can create
curl -s -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $AT"   $H/weather/forecast        # 200 — extension read is fine
curl -s -X POST -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $BT" $H/weather/refresh   # 403 — `refresh` not in defaults
```

### Per-user grant + revoke

```bash
# explicitly let bob refresh
$BIN grant  $BOB   weather refresh

# explicitly take alice's read away on the extension
$BIN revoke $ALICE weather read

# CLI mutates the DB directly — restart the server so the engine reloads
pkill -f 'authz-demo serve' ; sleep 1
$BIN serve --http-bind 127.0.0.1:8090 &
sleep 1
```

```bash
curl -s -X POST -o /dev/null -w "%{http_code}\n" -H "Authorization: Bearer $BT" $H/weather/refresh    # 200 — granted
curl -s -o /dev/null -w "%{http_code}\n"            -H "Authorization: Bearer $AT" $H/weather/forecast # 403 — revoked
curl -s -o /dev/null -w "%{http_code}\n"            -H "Authorization: Bearer $AT" $H/reports         # 200 — unaffected
```

The `grant` / `revoke` rules show up in `starter_authz_rules` with
`condition = subject == "<uuid>"` — that's how the engine narrows a
broad role rule to a single user.

## Where to look in the code

- [src/server.rs](src/server.rs) — the router composition. Note the
  layer order: `with_principal` outermost, then `Extension(engine)`,
  then per-route `with_permission`.
- [src/reports.rs](src/reports.rs) — built-in resource. Each route
  has its own `with_permission(kind, action)` wrapper.
- [src/weather.rs](src/weather.rs) — extension-contributed routes
  mounted by the host so they can carry policy-engine gates.
- [src/admin.rs](src/admin.rs) — `create_user`, `issue_token`,
  `grant`, `revoke`. Note the `subject == "<id>"` condition is what
  makes a rule per-user.
- [extensions/com.acme.weather/block.yaml](extensions/com.acme.weather/block.yaml) — the
  extension manifest. The loader validates it on boot; the
  `/extensions/*` admin slice lists it.

## Caveats

- `grant` / `revoke` go straight to the DB. The running server's
  engine cache is reloaded by the **CLI** process, not the **server**
  process, so policy changes only take effect after restarting the
  server. A production deployment would use the
  `starter-authz` `/v1/authz/rules` admin REST routes instead, which
  call `engine.reload()` in the server process automatically.
- The extension's two REST endpoints are mounted by hand in
  `weather.rs` instead of via `starter_ext_server::rest_router`. The
  bundled REST adapter only supports static `require_role` /
  `require_scope` gates from the manifest; this demo needs per-user,
  policy-engine gates, so the host owns the mounting decision.
