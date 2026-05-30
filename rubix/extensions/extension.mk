# Canonical extension Makefile contract (installed-only model).
#
# See `docs/scope/extensions/installed-only-model.md` — bundles only
# reach the runtime via POST /extensions/install. Dev and production
# use the same path. `dev_dirs` is on its way out.
#
# To adopt this in an extension's Makefile, write:
#
#     EXT_ID    := com.foo.bar
#     BIN_NAME  := rubix-foo-bar-extension
#     HAS_UI    := 1            # optional; omit if no ui-src/
#     include ../extension.mk
#
# Then add any bundle-specific targets (demo seeders, dump loaders,
# etc.) below the `include` line. Everything in this file is shared.
#
# Targets (the contract — identical for every extension):
#
#   make build      cargo build --release; copy binary next to block.yaml
#   make ui-build   vite build (only if HAS_UI is set)
#   make pack       tar -czf /tmp/<id>.tar.gz <bundle>
#   make install    POST /extensions/install with the tarball
#   make reload     make -C <repo>/rubix restart; wait for HTTP
#   make uninstall  DELETE /extensions/<id>?purge=true; assert dir gone
#   make all        build + ui-build (if HAS_UI) + pack + install + reload
#   make test       authenticate + GET /api/v1/extensions/<id>
#   make status     compact one-line state summary
#   make logs       tail /tmp/rubix-agent.log
#   make help       this menu
#   make clean      uninstall + cargo clean -p <bin>
#
# Verification targets (kept from the data-root scope doc):
#   make data-root        print resolved data root + ls installs dir
#   make cleanup-preview  GET /extensions/<id>/cleanup

SHELL          := /bin/bash

# Required (extension Makefile must set these before `include`):
ifndef EXT_ID
$(error EXT_ID must be set before `include ../extension.mk`)
endif
ifndef BIN_NAME
$(error BIN_NAME must be set before `include ../extension.mk`)
endif

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
TARBALL        ?= /tmp/$(EXT_ID).tar.gz
AGENT_LOG      ?= /tmp/rubix-agent.log

UI_SRC_DIR     := $(BUNDLE_DIR)/ui-src

# installs_dir as resolved by the running agent — grepped from the log
# rather than guessed. When the agent runs under snap its view of $HOME
# differs from this shell's (e.g. /home/user/snap/code/<rev>/.local/...).
INSTALLS_DIR   ?= $(shell sed 's/\x1B\[[0-9;]*[mGKHF]//g' $(AGENT_LOG) 2>/dev/null | grep -oE 'installs_dir=\S+' | tail -1 | cut -d= -f2)
DATA_ROOT      ?= $(if $(INSTALLS_DIR),$(patsubst %/extensions/installed,%,$(INSTALLS_DIR)),$(HOME)/.local/share/rubix)

# `make all` skips ui-build unless HAS_UI is set. Bundles without a
# ui-src/ tree (com.rubix.geo today) just don't define HAS_UI.
ifdef HAS_UI
ALL_DEPS := build ui-build pack install reload
else
ALL_DEPS := build pack install reload
endif

.PHONY: all build ui-build ui-dev pack install reload uninstall \
        test status logs clean help login \
        data-root cleanup-preview

help:
	@echo "$(EXT_ID) — make targets (installed-only model):"
	@echo "  make build         cargo build --release; copy binary into bundle dir"
ifdef HAS_UI
	@echo "  make ui-build      vite build → ui/remoteEntry.js"
	@echo "  make ui-dev        vite dev (watch mode)"
endif
	@echo "  make pack          tar -czf $(TARBALL) (excludes target/, node_modules/)"
	@echo "  make install       POST $(AGENT_BASE)/api/v1/extensions/install (multipart)"
	@echo "  make reload        restart rubix-agent so it re-scans installs_dir"
	@echo "  make uninstall     DELETE /extensions/$(EXT_ID)?purge=true"
	@echo "  make all           $(ALL_DEPS)"
	@echo "  make test          GET /extensions/$(EXT_ID) (after login)"
	@echo "  make status        compact state summary"
	@echo "  make logs          tail $(AGENT_LOG)"
	@echo "  make clean         uninstall + cargo clean -p $(BIN_NAME)"
	@echo ""
	@echo "  make data-root        print resolved data root + ls installs dir"
	@echo "  make cleanup-preview  GET /extensions/$(EXT_ID)/cleanup"

# ----- build -----------------------------------------------------------

build:
	@echo "==> cargo build -p $(BIN_NAME) (release, target=$(CARGO_TARGET_DIR))"
	cd $(WORKSPACE_DIR) && CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    cargo build --release -p $(BIN_NAME)
	@test -x $(BUILT_BIN) || { echo "build did not produce $(BUILT_BIN)"; exit 1; }
	@echo "==> install $(BIN_NAME) -> $(INSTALLED_BIN)"
	@install -m 0755 $(BUILT_BIN) $(INSTALLED_BIN)
	@ls -lh $(INSTALLED_BIN)

ifdef HAS_UI
ui-build:
	@echo "==> vite build → ui/remoteEntry.js"
	cd $(UI_SRC_DIR) && pnpm run build

ui-dev:
	cd $(UI_SRC_DIR) && pnpm run dev
else
ui-build:
	@echo "    (no ui-src/ — HAS_UI not set; skipping)"
endif

# ----- pack + install (the canonical dev + prod path) -----------------

pack:
	@echo "==> tar $(EXT_ID) → $(TARBALL)"
	@test -f $(BUNDLE_DIR)/block.yaml || { echo "no block.yaml in $(BUNDLE_DIR)"; exit 1; }
	@test -x $(INSTALLED_BIN) || { echo "no $(INSTALLED_BIN) — run 'make build' first"; exit 1; }
	@cd $(WORKSPACE_DIR) && tar \
	    --exclude='target' --exclude='node_modules' --exclude='.git' \
	    -czf $(TARBALL) $(EXT_ID)
	@ls -lh $(TARBALL)

login:
	@command -v jq   >/dev/null || { echo "needs jq";   exit 1; }
	@command -v curl >/dev/null || { echo "needs curl"; exit 1; }
	@curl -fsS -c $(COOKIE_JAR) -H 'content-type: application/json' \
	    -d '{"email":"$(ADMIN_EMAIL)","password":"$(ADMIN_PASSWORD)"}' \
	    $(AGENT_BASE)/api/v1/auth/login > $(LOGIN_JSON) \
	    || { echo "login failed — is the agent up?"; exit 1; }

install: login
	@test -f $(TARBALL) || { echo "no $(TARBALL) — run 'make pack' first"; exit 1; }
	@echo "==> POST $(AGENT_BASE)/api/v1/extensions/install (multipart)"
	@curl -fsS -b $(COOKIE_JAR) -X POST \
	    -F "file=@$(TARBALL);type=application/gzip" \
	    $(AGENT_BASE)/api/v1/extensions/install \
	    | jq -c '{id, code, pending_restart}'
	@if [[ -n "$(INSTALLS_DIR)" ]]; then \
	    test -d "$(INSTALLS_DIR)/$(EXT_ID)" \
	        && echo "    unpacked → $(INSTALLS_DIR)/$(EXT_ID)" \
	        || echo "    WARN: did not find $(INSTALLS_DIR)/$(EXT_ID) after install"; \
	fi

# Reload = restart the agent. Today the registry is sealed at boot,
# so a newly installed bundle only becomes Running after a restart.
# This will go away when/if the loader gains a live re-scan path.
reload:
	@echo "==> make -C $(REPO_ROOT)/rubix restart"
	@$(MAKE) --no-print-directory -C $(REPO_ROOT)/rubix restart >/tmp/rubix-restart.log 2>&1 &
	@echo "==> waiting for agent on 127.0.0.1:$(AGENT_PORT)"
	@for i in $$(seq 1 240); do \
	    code=$$(curl -sS -o /dev/null -w '%{http_code}' -m 1 \
	        $(AGENT_BASE)/api/v1/auth/me 2>/dev/null || echo 000); \
	    if [[ "$$code" == "401" || "$$code" == "200" ]]; then \
	        echo "    agent up after $${i}s (http=$$code)"; exit 0; \
	    fi; \
	    sleep 1; \
	done; \
	echo "agent did not come up within 240s"; tail -20 /tmp/rubix-restart.log; exit 1

uninstall: login
	@echo "==> DELETE $(AGENT_BASE)/api/v1/extensions/$(EXT_ID)?purge=true"
	@curl -fsS -b $(COOKIE_JAR) -X DELETE \
	    "$(AGENT_BASE)/api/v1/extensions/$(EXT_ID)?purge=true" \
	    | jq '{id, code, bundle}'
	@if [[ -n "$(INSTALLS_DIR)" && -e "$(INSTALLS_DIR)/$(EXT_ID)" ]]; then \
	    echo "WARN: $(INSTALLS_DIR)/$(EXT_ID) still exists"; \
	elif [[ -n "$(INSTALLS_DIR)" ]]; then \
	    echo "    confirmed: $(INSTALLS_DIR)/$(EXT_ID) removed"; \
	fi

all: $(ALL_DEPS)

# ----- introspection --------------------------------------------------

test: login
	@echo "==> GET $(AGENT_BASE)/api/v1/extensions/$(EXT_ID)"
	@curl -fsS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq '{id, state, enabled, version: .manifest.version, contributes: {tools: (.manifest.contributes.tools // [] | map(.id)), tables: (.manifest.contributes.warehouse_tables // [] | map(.name)), templates: (.manifest.contributes.warehouse_templates // [] | map(.name)), ui_slots: (.manifest.contributes.ui.exposes // [] | map(.slot))}}'

status: login
	@curl -fsS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID) \
	    | jq -c '{id, state, enabled}'

logs:
	@tail -F $(AGENT_LOG)

clean:
	@$(MAKE) --no-print-directory uninstall || true
	@if [[ -e $(INSTALLED_BIN) ]]; then rm -f $(INSTALLED_BIN); fi
	@if [[ -f $(TARBALL) ]]; then rm -f $(TARBALL); fi
	cd $(WORKSPACE_DIR) && CARGO_TARGET_DIR='$(CARGO_TARGET_DIR)' \
	    cargo clean -p $(BIN_NAME) || true

# ----- data-root verification (from the original scope doc) -----------

data-root:
	@echo "DATA_ROOT     = $(DATA_ROOT)"
	@echo "INSTALLS_DIR  = $(INSTALLS_DIR)"
	@if [[ -n "$(INSTALLS_DIR)" && -d "$(INSTALLS_DIR)" ]]; then \
	    echo "==> ls $(INSTALLS_DIR)"; ls -la "$(INSTALLS_DIR)"; \
	elif [[ -z "$(INSTALLS_DIR)" ]]; then \
	    echo "(could not parse installs_dir from $(AGENT_LOG) — is the agent up?)"; \
	else \
	    echo "(installs_dir does not yet exist — agent creates it on first install)"; \
	fi

cleanup-preview: login
	@echo "==> GET $(AGENT_BASE)/api/v1/extensions/$(EXT_ID)/cleanup"
	@curl -fsS -b $(COOKIE_JAR) -H 'accept: application/json' \
	    $(AGENT_BASE)/api/v1/extensions/$(EXT_ID)/cleanup \
	    | jq '{id, total_bytes, bundle, items: (.items | map({kind, name, bytes}))}'
