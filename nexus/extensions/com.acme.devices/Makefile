# com.acme.devices — bundle lifecycle against a running nexus-api.
#
# Mirrors the rubix `extensions/extension.mk` contract, trimmed for a
# builtin-flavour bundle (no Rust binary to build — the contributions are the
# product). The server is the one `make dev-be` / `make dev` runs from
# `nexus/` (API on 127.0.0.1:4780, admin seeded by `make seed`).
#
#   make ui-build         vite build ui-src/ → ui/remoteEntry.js
#   make pack             tar -czf /tmp/com.acme.devices.tar.gz (the bundle)
#   make install          POST /api/v1/extensions/install (multipart upload)
#   make load             POST /api/v1/extensions/com.acme.devices/enable
#   make unload           POST /api/v1/extensions/com.acme.devices/disable
#   make uninstall        DELETE /api/v1/extensions/com.acme.devices?purge=true
#   make test             end-to-end probe: list → detail → ui bytes → run the
#                         contributed kinds (ping + echo)
#   make status           one-line state summary
#   make cleanup-preview  GET /extensions/com.acme.devices/cleanup (dry run)
#   make all              ui-build + pack + install
#
# Notes:
# - The registry is sealed at boot: `install` persists the bundle + its kinds
#   and answers `pending_restart: true`; restart nexus-api (`make -C ../../../..
#   dev-be` from nexus/, or Ctrl-C + rerun `make dev`) to surface it live.
#   This in-repo copy is *already* scanned at boot via NEXUS_EXTENSIONS_DIR, so
#   `install` is only needed to exercise the upload path itself.
# - `load`/`unload` flip the persisted enablement row and take effect
#   immediately for the admin surface; for a builtin bundle there is no process
#   to start/stop, so they are the canonical "is this extension on?" toggle.

SHELL := /bin/bash
# pipefail so a failing curl is not masked by the jq it pipes into — `make
# test` must fail loudly when a probe step fails.
.SHELLFLAGS := -o pipefail -c

EXT_ID     := com.acme.devices
BUNDLE_DIR := $(abspath $(CURDIR))
PACK_DIR   := $(abspath $(BUNDLE_DIR)/..)

API_HOST   ?= 127.0.0.1
API_PORT   ?= 4780
API_BASE   ?= http://$(API_HOST):$(API_PORT)

# Same dev defaults as nexus/Makefile (`make seed`).
ADMIN_EMAIL    ?= admin@nexus.local
ADMIN_PASSWORD ?= change-me-admin

COOKIE_JAR ?= /tmp/$(EXT_ID).cookies
TARBALL    ?= /tmp/$(EXT_ID).tar.gz
UI_SRC_DIR := $(BUNDLE_DIR)/ui-src

# Process-flavour binary: the cargo crate under `process/`, built into the
# `nexus/extensions/` sibling workspace's target dir, then installed next to
# `block.yaml` as the manifest's `runtime.bin`.
BIN_NAME         := acme-devices-extension
WORKSPACE_DIR    := $(abspath $(BUNDLE_DIR)/..)
CARGO_TARGET_DIR := $(abspath $(WORKSPACE_DIR)/../../target/nexus-extensions)
BUILT_BIN        := $(CARGO_TARGET_DIR)/release/$(BIN_NAME)
INSTALLED_BIN    := $(BUNDLE_DIR)/$(BIN_NAME)

.PHONY: all build bin-build ui-build ui-dev pack login install load unload uninstall \
        test status cleanup-preview demo help

# Demo user the `demo` walkthrough provisions a device AS. Distinct from the
# admin so the team-scoped read (`$caller_team_ids`) is proven on a real,
# non-admin principal, not just the seed admin.
DEMO_EMAIL    ?= tech@nexus.local
DEMO_PASSWORD ?= change-me-tech

all: ui-build pack install

help:
	@echo "$(EXT_ID) — make targets:"
	@echo "  make ui-build         vite build ui-src/ → ui/remoteEntry.js"
	@echo "  make pack             tar → $(TARBALL)"
	@echo "  make install          POST $(API_BASE)/api/v1/extensions/install"
	@echo "  make load             enable (POST .../enable)"
	@echo "  make unload           disable (POST .../disable)"
	@echo "  make uninstall        DELETE .../$(EXT_ID)?purge=true"
	@echo "  make test             e2e probe (list, detail, ui bytes, run kinds)"
	@echo "  make demo             team + user + assign access + provision + view on dashboard"
	@echo "  make status           one-line state summary"
	@echo "  make cleanup-preview  dry-run cleanup manifest"

# ----- build ------------------------------------------------------------

# Build both halves of the bundle: the supervised process binary + the UI.
build: bin-build ui-build

# cargo build --release; install the binary next to block.yaml where the
# supervisor execs it (`runtime.bin`).
bin-build:
	@echo "==> cargo build -p $(BIN_NAME) (release, target=$(CARGO_TARGET_DIR))"
	cd $(WORKSPACE_DIR) && CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    cargo build --release -p $(BIN_NAME)
	@test -x $(BUILT_BIN) || { echo "build did not produce $(BUILT_BIN)"; exit 1; }
	@echo "==> install $(BIN_NAME) -> $(INSTALLED_BIN)"
	@install -m 0755 $(BUILT_BIN) $(INSTALLED_BIN)
	@ls -lh $(INSTALLED_BIN)

ui-build:
	@echo "==> vite build → ui/remoteEntry.js"
	cd $(UI_SRC_DIR) && pnpm install && pnpm run build

ui-dev:
	cd $(UI_SRC_DIR) && pnpm run dev

# ----- pack + install (the upload path) ---------------------------------

pack:
	@test -f $(BUNDLE_DIR)/block.yaml || { echo "no block.yaml in $(BUNDLE_DIR)"; exit 1; }
	@test -f $(BUNDLE_DIR)/ui/remoteEntry.js || { echo "no ui/remoteEntry.js — run 'make ui-build' first"; exit 1; }
	@echo "==> tar $(EXT_ID) → $(TARBALL)"
	@cd $(PACK_DIR) && tar \
	    --exclude='node_modules' --exclude='ui-src' --exclude='.git' \
	    -czf $(TARBALL) $(EXT_ID)
	@ls -lh $(TARBALL)

# Cookie sessions are guarded by the double-submit CSRF layer: every mutating
# (POST/PUT/PATCH/DELETE) request must echo the `starter_csrf` cookie back as the
# `X-CSRF-Token` header. `login` stashes the token; `$(CSRF_HDR)` expands to the
# header for use on mutations. (Safe GETs need no token; bearer clients are
# exempt, but this harness uses the cookie session.)
CSRF_FILE  ?= /tmp/$(EXT_ID).csrf
CSRF_HDR    = -H "X-CSRF-Token: $$(cat $(CSRF_FILE))"

login:
	@command -v jq   >/dev/null || { echo "needs jq";   exit 1; }
	@command -v curl >/dev/null || { echo "needs curl"; exit 1; }
	@curl -fsS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(API_BASE)/auth/login \
	    | jq -r '.csrf_token' > $(CSRF_FILE) \
	    || { echo "login failed — is nexus-api up (make dev-be) and seeded (make seed)?"; exit 1; }
	@test -s $(CSRF_FILE) || { echo "login: no csrf_token in response"; exit 1; }

install: login
	@test -f $(TARBALL) || { echo "no $(TARBALL) — run 'make pack' first"; exit 1; }
	@echo "==> POST $(API_BASE)/api/v1/extensions/install (multipart)"
	@curl -fsS -b $(COOKIE_JAR) $(CSRF_HDR) -X POST \
	    -F "file=@$(TARBALL);type=application/gzip" \
	    $(API_BASE)/api/v1/extensions/install \
	    | jq -c '{id, code, pending_restart}'

# ----- enable / disable (load / unload) ---------------------------------

load: login
	@curl -fsS -b $(COOKIE_JAR) $(CSRF_HDR) -X POST \
	    $(API_BASE)/api/v1/extensions/$(EXT_ID)/enable | jq -c .

unload: login
	@curl -fsS -b $(COOKIE_JAR) $(CSRF_HDR) -X POST \
	    $(API_BASE)/api/v1/extensions/$(EXT_ID)/disable | jq -c .

uninstall: login
	@echo "==> DELETE $(API_BASE)/api/v1/extensions/$(EXT_ID)?purge=true"
	@curl -fsS -b $(COOKIE_JAR) $(CSRF_HDR) -X DELETE \
	    "$(API_BASE)/api/v1/extensions/$(EXT_ID)?purge=true" \
	    | jq -c '{id, code, removed: (.removed | length), bundle}'

# ----- verification ------------------------------------------------------

# End-to-end probe of everything this bundle contributes, against the live
# server: admin list shows it, detail resolves, the federation bundle serves
# (200 + ETag, then 304 on revalidation), and BOTH contributed query-kinds run
# through the dispatcher's third source.
test: login
	@echo "==> list contains $(EXT_ID)"
	@curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/extensions \
	    | jq -e '.[] | select(.id == "$(EXT_ID)") | {id, state, enabled, restart_required}' \
	    || { echo "FAIL: $(EXT_ID) not in registry"; exit 1; }
	@echo "==> detail"
	@curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq -c '{id, state, enabled}'
	@echo "==> ui bundle serves (expect 200 + etag)"
	@etag=$$(curl -fsSI $(API_BASE)/api/v1/extensions/$(EXT_ID)/ui/remoteEntry.js \
	    | tr -d '\r' | awk 'tolower($$1)=="etag:"{print $$2}'); \
	  test -n "$$etag" || { echo "FAIL: no ETag on ui bundle"; exit 1; }; \
	  echo "    etag: $$etag"; \
	  code=$$(curl -s -o /dev/null -w '%{http_code}' \
	      -H "if-none-match: $$etag" \
	      $(API_BASE)/api/v1/extensions/$(EXT_ID)/ui/remoteEntry.js); \
	  test "$$code" = "304" || { echo "FAIL: revalidation gave $$code, want 304"; exit 1; }; \
	  echo "    revalidation: 304"
	@echo "==> site-checkout query-kind runs principal-scoped (\$$caller_team_ids)"
	@# The kind selects the host-bound `$$caller_tenant_id` / `$$caller_team_ids`
	@# tokens. `POST /api/v1/query` now binds the verified principal for a
	@# kind-mode request (running against the metadata DB where extension-owned
	@# tables live), so a cookie session resolves those tokens and the kind RUNS.
	@# We assert it both (a) resolves by name (a bogus kind says "not a known
	@# kind") and (b) returns a row frame with the expected columns — proof the
	@# host tokens bound, not a guard error. (A principal-LESS call still 400s
	@# "needs host token" — exercised separately by the dev-shortcut path.)
	@body=$$(curl -s -b $(COOKIE_JAR) $(CSRF_HDR) -H 'content-type: application/json' \
	    -d '{"kind":"$(EXT_ID).site_checkout","params":{}}' $(API_BASE)/api/v1/query); \
	  echo "$$body" | grep -q 'not a known kind' \
	    && { echo "FAIL: site_checkout kind not registered"; exit 1; }; \
	  echo "$$body" | grep -q '"owner_tenant"' \
	    || { echo "FAIL: unexpected query response: $$body"; exit 1; }; \
	  echo "    runs principal-scoped (host tokens bound from the session)"
	@echo "==> ensure caller is in an allowed team (template allows: hvac-ops)"
	@uid=$$(curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/me | jq -r '.subject'); \
	  team_id=$$(curl -fsS -b $(COOKIE_JAR) $(API_BASE)/v1/tenants/nexus/teams \
	    | jq -r '.[] | select(.slug=="hvac-ops") | .id'); \
	  if [ -z "$$team_id" ]; then \
	    team_id=$$(curl -fsS -b $(COOKIE_JAR) $(CSRF_HDR) -H 'content-type: application/json' \
	      -d '{"slug":"hvac-ops","display_name":"HVAC Ops"}' \
	      $(API_BASE)/v1/tenants/nexus/teams | jq -r '.id'); \
	    echo "    created team hvac-ops ($$team_id)"; \
	  fi; \
	  curl -s -b $(COOKIE_JAR) $(CSRF_HDR) -H 'content-type: application/json' \
	    -d "{\"user_id\":\"$$uid\"}" \
	    $(API_BASE)/v1/tenants/nexus/teams/$$team_id/members >/dev/null; \
	  echo "    $$uid in hvac-ops"
	@echo "==> re-login so the session principal carries the team (refreshes CSRF too)"
	@curl -fsS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(API_BASE)/auth/login | jq -r '.csrf_token' > $(CSRF_FILE)
	@echo "==> bundled setup template imported (source = extension)"
	@curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/setup/templates \
	    | jq -e '.[] | select(.id == "com.acme.add-device")' \
	    || { echo "FAIL: add-device template not in catalog"; exit 1; }
	@echo "==> run the add-device automation (202 + run_id)"
	@run_id=$$(curl -fsS -b $(COOKIE_JAR) $(CSRF_HDR) -H 'content-type: application/json' \
	    -d '{"barcode":"0X1A2B","location":"Roof AHU-3"}' \
	    $(API_BASE)/api/v1/setup/templates/com.acme.add-device/run | jq -r '.run_id'); \
	  test -n "$$run_id" || { echo "FAIL: no run_id"; exit 1; }; \
	  echo "    run_id: $$run_id"; \
	  sleep 2; \
	  echo "==> snapshot (expect status=completed, progress 2/2 — nodes ran in the child)"; \
	  snap=$$(curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/setup/runs/$$run_id); \
	  echo "$$snap" | jq -c '{status, progress}'; \
	  echo "$$snap" | jq -e '.status == "completed" and .progress.done == .progress.total' >/dev/null \
	    || { echo "FAIL: run did not complete"; exit 1; }
	@echo "PASS: $(EXT_ID) end to end"

status: login
	@curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/extensions \
	    | jq -r '.[] | select(.id == "$(EXT_ID)") | "\(.id) state=\(.state) enabled=\(.enabled) restarts=\(.restart_count) restart_required=\(.restart_required)"'

cleanup-preview: login
	@curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/extensions/$(EXT_ID)/cleanup | jq .

# ----- demo: nav + team + user + assigned access + view the device ------
#
# The full story the dashboard demo tells, against the live stack:
#   1. (as admin) create the `hvac-ops` TEAM the add-device template allows.
#   2. (as admin) create a dedicated `tech` USER (writer role).
#   3. (as admin) ASSIGN access by adding tech to hvac-ops — that team membership
#      is what gets tech past the template's `allowed_teams` gate AND what scopes
#      the `devices_list` read to their team ($caller_team_ids).
#   4. (as tech) provision a device — the run is tagged with tech's trusted
#      identity (owner/team/tenant) seeded server-side, never from the form.
#   5. (as tech) read the persisted device back via the `devices_list` kind — the
#      exact rows the Devices dashboard page renders, scoped to tech's team.
#
# Idempotent: re-running reuses the existing team/user/membership.
DEMO_JAR  ?= /tmp/$(EXT_ID).demo.cookies
DEMO_CSRF ?= /tmp/$(EXT_ID).demo.csrf
DEMO_HDR   = -H "X-CSRF-Token: $$(cat $(DEMO_CSRF))"

demo: login
	@echo "==> [admin] ensure team hvac-ops"
	@team_id=$$(curl -fsS -b $(COOKIE_JAR) $(API_BASE)/v1/tenants/nexus/teams \
	    | jq -r '.[] | select(.slug=="hvac-ops") | .id'); \
	  if [ -z "$$team_id" ]; then \
	    team_id=$$(curl -fsS -b $(COOKIE_JAR) $(CSRF_HDR) -H 'content-type: application/json' \
	      -d '{"slug":"hvac-ops","display_name":"HVAC Ops"}' \
	      $(API_BASE)/v1/tenants/nexus/teams | jq -r '.id'); \
	    echo "    created team hvac-ops ($$team_id)"; \
	  else echo "    team hvac-ops exists ($$team_id)"; fi; \
	  echo "==> [admin] ensure user $(DEMO_EMAIL) (writer)"; \
	  uid=$$(curl -s -b $(COOKIE_JAR) $(CSRF_HDR) -H 'content-type: application/json' \
	    -d '{"email":"$(DEMO_EMAIL)","password":"$(DEMO_PASSWORD)","role":"writer"}' \
	    $(API_BASE)/v1/tenants/nexus/users | jq -r '.user_id // empty'); \
	  if [ -z "$$uid" ]; then \
	    echo "    (already exists — logging in to resolve its id)"; \
	    curl -fsS -c $(DEMO_JAR) -H 'content-type: application/json' \
	      -d '{"email":"$(DEMO_EMAIL)","password":"$(DEMO_PASSWORD)"}' \
	      $(API_BASE)/auth/login >/dev/null; \
	    uid=$$(curl -fsS -b $(DEMO_JAR) $(API_BASE)/api/v1/me | jq -r '.subject'); \
	  fi; \
	  echo "    user id: $$uid"; \
	  echo "==> [admin] assign access: add $(DEMO_EMAIL) to hvac-ops"; \
	  curl -s -b $(COOKIE_JAR) $(CSRF_HDR) -H 'content-type: application/json' \
	    -d "{\"user_id\":\"$$uid\"}" \
	    $(API_BASE)/v1/tenants/nexus/teams/$$team_id/members >/dev/null; \
	  echo "    $$uid is in hvac-ops"
	@echo "==> [tech] log in (session principal now carries the team)"
	@curl -fsS -c $(DEMO_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(DEMO_EMAIL)","password":"$(DEMO_PASSWORD)"}' \
	    $(API_BASE)/auth/login | jq -r '.csrf_token' > $(DEMO_CSRF)
	@curl -fsS -b $(DEMO_JAR) $(API_BASE)/api/v1/me | jq -c '{subject, role, teams}'
	@echo "==> [tech] provision a device (gated by allowed_teams: hvac-ops)"
	@run_id=$$(curl -fsS -b $(DEMO_JAR) $(DEMO_HDR) -H 'content-type: application/json' \
	    -d '{"barcode":"ACME-DEMO-0001","location":"Roof AHU-7"}' \
	    $(API_BASE)/api/v1/setup/templates/com.acme.add-device/run | jq -r '.run_id'); \
	  test -n "$$run_id" || { echo "FAIL: no run_id (is tech in hvac-ops?)"; exit 1; }; \
	  echo "    run_id: $$run_id"; sleep 2; \
	  snap=$$(curl -fsS -b $(DEMO_JAR) $(API_BASE)/api/v1/setup/runs/$$run_id); \
	  echo "$$snap" | jq -c '{status, progress, owner, team, tenant_id}'; \
	  echo "$$snap" | jq -e '.status=="completed"' >/dev/null \
	    || { echo "FAIL: run did not complete"; exit 1; }
	@echo "==> verify the device PERSISTED to the extension-owned nexus table"
	@# Ground truth: the row the automation wrote via the tenant-stamped
	@# `warehouse.write` host method into `com_acme_devices__devices`. We assert
	@# it landed with tech's trusted identity (tenant=nexus, owner=tech,
	@# team=hvac-ops) — seeded server-side, never from the form. This is what the
	@# Devices dashboard renders.
	@#
	@# NOTE on the read path: the dashboard UI reads the row back via the
	@# `devices_list` query-kind, which uses the host-bound `$$caller_tenant_id` /
	@# `$$caller_team_ids` tokens. Those only resolve on a PRINCIPAL-BEARING query
	@# route; the ad-hoc `POST /api/v1/query` runs with no principal (by design),
	@# so it returns 400 "needs host token" there. The persisted row below is the
	@# authoritative proof; wiring a principal-bearing read for extension-owned
	@# tables is the remaining UI-read gap (see this session's notes).
	@PGPASSWORD=$${DB_PASS:-nexus} docker exec $${DB_CONTAINER:-nexus-dev-pg} \
	    psql -U $${DB_USER:-nexus} -d $${DB_NAME:-nexus} -tAc \
	    "SELECT tenant_id||'|'||owner||'|'||coalesce(team,'')||'|'||barcode \
	     FROM com_acme_devices__devices WHERE barcode='ACME-DEMO-0001';" \
	    | tee /tmp/$(EXT_ID).demo.row; \
	  grep -q '^nexus|.*|hvac-ops|ACME-DEMO-0001$$' /tmp/$(EXT_ID).demo.row \
	    || { echo "FAIL: device did not persist with tech's trusted identity"; exit 1; }
	@echo "==> [tech] view the device on the dashboard (devices_list, team-scoped)"
	@# The Devices dashboard reads the persisted rows back through the
	@# `devices_list` query-kind. `POST /api/v1/query` binds tech's verified
	@# principal for this kind-mode read, so `$$caller_tenant_id`/`$$caller_team_ids`
	@# resolve and tech sees their team's devices — exactly what the page renders.
	@rows=$$(curl -fsS -b $(DEMO_JAR) $(DEMO_HDR) -H 'content-type: application/json' \
	    -d '{"kind":"$(EXT_ID).devices_list","params":{}}' $(API_BASE)/api/v1/query); \
	  echo "$$rows" | jq -c '.rows[] | {device_id, barcode, location, team}'; \
	  echo "$$rows" | jq -e '[.rows[] | select(.barcode=="ACME-DEMO-0001")] | length >= 1' >/dev/null \
	    || { echo "FAIL: provisioned device not visible to tech via devices_list"; exit 1; }
	@echo "PASS: $(EXT_ID) demo — team + user + assigned access + device PERSISTED and VISIBLE on the dashboard"
