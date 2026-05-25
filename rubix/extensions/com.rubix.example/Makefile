# `com.rubix.example` — extension build & load Makefile.
#
# Builds the `rubix-example-extension` process binary out of the
# sibling `rubix/extensions/` cargo workspace and lands it next to
# `block.yaml`, where the rubix-agent supervisor expects to exec it
# (it resolves `runtime.bin` against the bundle dir — see
# `starter-ext-supervisor::supervisor`).
#
# Typical flow:
#
#   make build      compile the extension binary (release profile)
#   make install    copy the built binary into this bundle dir
#   make load       restart rubix-agent so the loader rescans
#                   (cheap; the agent's boot path re-validates the
#                    sealed registry and the supervisor respawns
#                    every enabled record)
#   make test       hit GET /api/v1/extensions/<id> via the agent's
#                   admin route and dump the manifest + lifecycle
#                   state (operator authn, Role::Admin)
#   make all        build + install + load + test in one shot
#
# Conventions:
#   - The bundle dir is THIS directory (where `block.yaml` lives).
#     The supervisor's `runtime.bin` is resolved relative to it.
#   - The agent is expected to be listening on $(AGENT_PORT) (the
#     same port rubix/Makefile's `agent` target uses).
#   - `make load` is a process restart, not a hot-reload — the
#     extension subsystem has no live `/reload` route today; the
#     loader runs once at boot.
#
# References:
#   - rubix/Makefile         agent + frontend launchers
#   - rubix/dev/agent.toml   `[extensions].dir = "rubix/extensions"`

SHELL          := /bin/bash
EXT_ID         := com.rubix.example
BIN_NAME       := rubix-example-extension
BUNDLE_DIR     := $(CURDIR)
WORKSPACE_DIR  := $(abspath $(BUNDLE_DIR)/..)
REPO_ROOT      := $(abspath $(WORKSPACE_DIR)/../..)
# IMPORTANT: keep cargo's target dir OUT of $(WORKSPACE_DIR) — the
# extensions root is scanned by the agent's loader, which treats every
# immediate child directory as an extension candidate, so a stray
# `rubix/extensions/target/` would surface as a phantom (failed) id.
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

.PHONY: all build install uninstall load test status logs clean help

help:
	@echo "$(EXT_ID) — make targets:"
	@echo "  make build      compile the extension binary (release)"
	@echo "  make install    copy built binary into bundle dir"
	@echo "  make load       restart rubix-agent to pick up changes"
	@echo "  make test       authenticate + GET /api/v1/extensions/$(EXT_ID)"
	@echo "  make status     same as test but compact summary only"
	@echo "  make logs       tail the rubix-agent log"
	@echo "  make uninstall  remove the installed binary from bundle dir"
	@echo "  make clean      uninstall + cargo clean the workspace"
	@echo "  make all        build + install + load + test"

# Build the binary in the sibling rubix/extensions/ cargo workspace.
# `-p rubix-example-extension` pins the member; --release keeps the
# disk footprint of the bundle dir reasonable.
build:
	@echo "==> cargo build -p $(BIN_NAME) (release, target=$(CARGO_TARGET_DIR))"
	cd $(WORKSPACE_DIR) && CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    cargo build --release -p $(BIN_NAME)
	@test -x $(BUILT_BIN) || { echo "build did not produce $(BUILT_BIN)"; exit 1; }
	@echo "    built: $(BUILT_BIN)"

# Copy (not symlink) the binary into the bundle dir. A real file
# matches what a `tar`-distributed extension would look like in
# production and side-steps any symlink-escape checks the supervisor
# might add later. Replaces an existing binary in place.
install: build
	@echo "==> install $(BIN_NAME) -> $(INSTALLED_BIN)"
	@install -m 0755 $(BUILT_BIN) $(INSTALLED_BIN)
	@ls -lh $(INSTALLED_BIN)

uninstall:
	@if [[ -e $(INSTALLED_BIN) ]]; then \
	    echo "==> rm $(INSTALLED_BIN)"; rm -f $(INSTALLED_BIN); \
	else \
	    echo "    no installed binary at $(INSTALLED_BIN)"; \
	fi

# Reload = restart rubix-agent. The boot path:
#   1. scans `[extensions].dir` (rubix/extensions),
#   2. runs `Loader::validate_all` on every block.yaml,
#   3. seals the registry,
#   4. autostarts every enabled record's supervisor.
# So a restart is the canonical way to pick up a freshly installed
# binary today.
#
# We delegate to `make -C rubix restart` (the canonical rubix
# launcher). That target stops any running agent + Vite, then
# re-spawns them with cwd=$(REPO_ROOT) so the repo-relative
# `[extensions].dir = rubix/extensions` resolves correctly.
AGENT_LOG     ?= /tmp/rubix-agent.log

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

# Smoke-test the agent's admin route. Two HTTP calls:
#   1. POST /api/v1/auth/login (operator) → cookie jar + csrf token
#   2. GET  /api/v1/extensions/<id>       (Role::Admin via cookie)
# We don't need the CSRF token for the GET — CSRF is only enforced
# on mutating verbs — but it's free to capture for chained tests.
test:
	@command -v jq  >/dev/null || { echo "test target needs jq";  exit 1; }
	@command -v curl >/dev/null || { echo "test target needs curl"; exit 1; }
	@echo "==> POST $(AGENT_BASE)/api/v1/auth/login"
	@curl -sS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(AGENT_BASE)/api/v1/auth/login > $(LOGIN_JSON)
	@if ! jq -e '.csrf_token // .session_id // .principal' $(LOGIN_JSON) >/dev/null 2>&1; then \
	    echo "login failed; response:"; cat $(LOGIN_JSON); exit 1; \
	fi
	@echo "    login ok"
	@echo
	@echo "==> GET  $(AGENT_BASE)/api/v1/extensions/$(EXT_ID)"
	@curl -sS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq '{id, state, enabled, manifest_id: .manifest.id, version: .manifest.version, contributes: {tools: (.manifest.contributes.tools // [] | map(.id)), skills: (.manifest.contributes.skills // [] | map(.dir)), flows: (.manifest.contributes.flows // [] | map(.id)), ui_slots: (.manifest.contributes.ui.exposes // [] | map(.slot))}}'
	@echo
	@echo "==> GET  $(AGENT_BASE)/api/v1/extensions  (this id only)"
	@curl -sS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions \
	    | jq --arg id '$(EXT_ID)' '.. | objects | select(.id == $$id)'

status:
	@command -v jq >/dev/null || { echo "status target needs jq"; exit 1; }
	@curl -sS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(AGENT_BASE)/api/v1/auth/login >/dev/null
	@curl -sS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq -c '{id, state, enabled}'

logs:
	@tail -F /tmp/rubix-agent.log

clean: uninstall
	@echo "==> cargo clean -p $(BIN_NAME)"
	cd $(WORKSPACE_DIR) && CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    cargo clean -p $(BIN_NAME) || true

all: build install load test
