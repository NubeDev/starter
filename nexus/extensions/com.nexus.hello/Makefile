# com.nexus.hello — bundle lifecycle against a running nexus-api.
#
# Mirrors the rubix `extensions/extension.mk` contract, trimmed for a
# builtin-flavour bundle (no Rust binary to build — the contributions are the
# product). The server is the one `make dev-be` / `make dev` runs from
# `nexus/` (API on 127.0.0.1:4780, admin seeded by `make seed`).
#
#   make ui-build         vite build ui-src/ → ui/remoteEntry.js
#   make pack             tar -czf /tmp/com.nexus.hello.tar.gz (the bundle)
#   make install          POST /api/v1/extensions/install (multipart upload)
#   make load             POST /api/v1/extensions/com.nexus.hello/enable
#   make unload           POST /api/v1/extensions/com.nexus.hello/disable
#   make uninstall        DELETE /api/v1/extensions/com.nexus.hello?purge=true
#   make test             end-to-end probe: list → detail → ui bytes → run the
#                         contributed kinds (ping + echo)
#   make status           one-line state summary
#   make cleanup-preview  GET /extensions/com.nexus.hello/cleanup (dry run)
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

EXT_ID     := com.nexus.hello
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
BIN_NAME         := nexus-hello-extension
WORKSPACE_DIR    := $(abspath $(BUNDLE_DIR)/..)
CARGO_TARGET_DIR := $(abspath $(WORKSPACE_DIR)/../../target/nexus-extensions)
BUILT_BIN        := $(CARGO_TARGET_DIR)/release/$(BIN_NAME)
INSTALLED_BIN    := $(BUNDLE_DIR)/$(BIN_NAME)

.PHONY: all build bin-build ui-build ui-dev pack login install load unload uninstall \
        test status cleanup-preview help

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
	cd $(UI_SRC_DIR) && pnpm run build

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

login:
	@command -v jq   >/dev/null || { echo "needs jq";   exit 1; }
	@command -v curl >/dev/null || { echo "needs curl"; exit 1; }
	@curl -fsS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(API_BASE)/auth/login > /dev/null \
	    || { echo "login failed — is nexus-api up (make dev-be) and seeded (make seed)?"; exit 1; }

install: login
	@test -f $(TARBALL) || { echo "no $(TARBALL) — run 'make pack' first"; exit 1; }
	@echo "==> POST $(API_BASE)/api/v1/extensions/install (multipart)"
	@curl -fsS -b $(COOKIE_JAR) -X POST \
	    -F "file=@$(TARBALL);type=application/gzip" \
	    $(API_BASE)/api/v1/extensions/install \
	    | jq -c '{id, code, pending_restart}'

# ----- enable / disable (load / unload) ---------------------------------

load: login
	@curl -fsS -b $(COOKIE_JAR) -X POST \
	    $(API_BASE)/api/v1/extensions/$(EXT_ID)/enable | jq -c .

unload: login
	@curl -fsS -b $(COOKIE_JAR) -X POST \
	    $(API_BASE)/api/v1/extensions/$(EXT_ID)/disable | jq -c .

uninstall: login
	@echo "==> DELETE $(API_BASE)/api/v1/extensions/$(EXT_ID)?purge=true"
	@curl -fsS -b $(COOKIE_JAR) -X DELETE \
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
	@echo "==> run contributed kind $(EXT_ID).ping"
	@curl -fsS -b $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"sql":"","kind":"$(EXT_ID).ping"}' \
	    $(API_BASE)/api/v1/query | jq -c '.rows[0]'
	@echo "==> run contributed kind $(EXT_ID).echo"
	@curl -fsS -b $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"sql":"","kind":"$(EXT_ID).echo","params":{"message":"e2e works"}}' \
	    $(API_BASE)/api/v1/query | jq -c '.rows[0]'
	@echo "PASS: $(EXT_ID) end to end"

status: login
	@curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/extensions \
	    | jq -r '.[] | select(.id == "$(EXT_ID)") | "\(.id) state=\(.state) enabled=\(.enabled) restarts=\(.restart_count) restart_required=\(.restart_required)"'

cleanup-preview: login
	@curl -fsS -b $(COOKIE_JAR) $(API_BASE)/api/v1/extensions/$(EXT_ID)/cleanup | jq .
