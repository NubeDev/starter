# `com.nubeio.rubixos` — extension build & load Makefile. Mirrors
# `com.rubix.example/Makefile`; see that file's comments for the
# full agent boot/restart/test flow.

SHELL          := /bin/bash
EXT_ID         := com.nubeio.rubixos
BIN_NAME       := rubix-nubeio-rubixos-extension
BUNDLE_DIR     := $(CURDIR)
WORKSPACE_DIR  := $(abspath $(BUNDLE_DIR)/..)
REPO_ROOT      := $(abspath $(WORKSPACE_DIR)/../..)
CARGO_TARGET_DIR := $(REPO_ROOT)/target/rubix-extensions
TARGET_DIR     := $(CARGO_TARGET_DIR)/release
BUILT_BIN      := $(TARGET_DIR)/$(BIN_NAME)
INSTALLED_BIN  := $(BUNDLE_DIR)/$(BIN_NAME)

AGENT_HOST     ?= 127.0.0.1
AGENT_PORT     ?= 8088
AGENT_BASE     ?= http://$(AGENT_HOST):$(AGENT_PORT)
ADMIN_EMAIL    ?= op@example.com
ADMIN_PASSWORD ?= rubix-dev-passwd

COOKIE_JAR     ?= /tmp/$(EXT_ID).cookies
LOGIN_JSON     ?= /tmp/$(EXT_ID).login.json

UI_SRC_DIR     := $(BUNDLE_DIR)/ui-src

# pg dump loader knobs. Default container is the rubix-postgres
# service from `rubix/docker/`. Override with `make load-dump
# CONTAINER=… DUMP=…`.
DUMP           ?= /home/user/Documents/db.dump
CONTAINER      ?=
TENANT_ID      ?= system

.PHONY: all build install uninstall load test status logs clean help \
        ui-build ui-dev load-dump

help:
	@echo "$(EXT_ID) — make targets:"
	@echo "  make build      compile the extension binary (release)"
	@echo "  make install    copy built binary into bundle dir"
	@echo "  make ui-build   vite build → ui/remoteEntry.js"
	@echo "  make load       restart rubix-agent to pick up changes"
	@echo "  make all        ui-build + build + install + load + test"
	@echo "  make test       authenticate + GET /api/v1/extensions/$(EXT_ID)"
	@echo "  make load-dump  pg_restore + ingest dump into com_nubeio_rubixos__* tables"
	@echo "                    DUMP=$(DUMP)  CONTAINER=…  TENANT_ID=$(TENANT_ID)"

ui-build:
	@echo "==> vite build → ui/remoteEntry.js"
	cd $(UI_SRC_DIR) && pnpm run build

ui-dev:
	cd $(UI_SRC_DIR) && pnpm run dev

build:
	@echo "==> cargo build -p $(BIN_NAME) (release, target=$(CARGO_TARGET_DIR))"
	cd $(WORKSPACE_DIR) && CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    cargo build --release -p $(BIN_NAME)
	@test -x $(BUILT_BIN) || { echo "build did not produce $(BUILT_BIN)"; exit 1; }
	@echo "    built: $(BUILT_BIN)"

install: build
	@echo "==> install $(BIN_NAME) -> $(INSTALLED_BIN)"
	@install -m 0755 $(BUILT_BIN) $(INSTALLED_BIN)
	@ls -lh $(INSTALLED_BIN)

uninstall:
	@if [[ -e $(INSTALLED_BIN) ]]; then rm -f $(INSTALLED_BIN); fi

load:
	@echo "==> make -C $(REPO_ROOT)/rubix restart"
	@$(MAKE) --no-print-directory -C $(REPO_ROOT)/rubix restart
	@echo "==> waiting for agent to bind 127.0.0.1:$(AGENT_PORT)"
	@for i in $$(seq 1 120); do \
	    code=$$(curl -sS -o /dev/null -w '%{http_code}' \
	        $(AGENT_BASE)/api/v1/auth/me 2>/dev/null || echo 000); \
	    if [[ "$$code" == "401" || "$$code" == "200" ]]; then \
	        echo "    agent up (http=$$code)"; exit 0; \
	    fi; \
	    sleep 1; \
	done; \
	echo "agent did not come up within 120s; last code=$$code"; exit 1

test:
	@command -v jq  >/dev/null || { echo "test target needs jq";  exit 1; }
	@command -v curl >/dev/null || { echo "test target needs curl"; exit 1; }
	@echo "==> login"
	@curl -sS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(AGENT_BASE)/api/v1/auth/login > $(LOGIN_JSON)
	@echo "==> GET $(AGENT_BASE)/api/v1/extensions/$(EXT_ID)"
	@curl -sS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq '{id, state, enabled, version: .manifest.version, contributes: {tools: (.manifest.contributes.tools // [] | map(.id)), tables: (.manifest.contributes.warehouse_tables // [] | map(.name)), templates: (.manifest.contributes.warehouse_templates // [] | map(.name)), ui_slots: (.manifest.contributes.ui.exposes // [] | map(.slot))}}'

status:
	@curl -sS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(AGENT_BASE)/api/v1/auth/login >/dev/null
	@curl -sS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq -c '{id, state, enabled}'

logs:
	@tail -F /tmp/rubix-agent.log

clean: uninstall
	cd $(WORKSPACE_DIR) && CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    cargo clean -p $(BIN_NAME) || true

all: ui-build build install load test

# pg_restore + INSERT...SELECT into com_nubeio_rubixos__*. Auto-
# detects the rubix-postgres container if CONTAINER is unset.
load-dump:
	@if [[ -z "$(CONTAINER)" ]]; then \
	    CONTAINER=$$(docker ps --format '{{.ID}} {{.Image}}' | awk '/postgres|timescale/ {print $$1; exit}'); \
	    if [[ -z "$$CONTAINER" ]]; then \
	        echo "no postgres/timescale container detected — pass CONTAINER=<id>"; exit 1; \
	    fi; \
	else \
	    CONTAINER='$(CONTAINER)'; \
	fi; \
	echo "==> CONTAINER=$$CONTAINER  DUMP=$(DUMP)  TENANT_ID=$(TENANT_ID)"; \
	$(BUNDLE_DIR)/scripts/load-dump.sh \
	    --dump '$(DUMP)' \
	    --container "$$CONTAINER" \
	    --tenant-id '$(TENANT_ID)'
