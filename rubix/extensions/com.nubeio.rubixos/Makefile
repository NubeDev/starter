# com.nubeio.rubixos — extension build + lifecycle + pg-dump loader.
#
# The canonical lifecycle targets (build / pack / install / reload /
# uninstall / test / status / data-root / cleanup-preview) come from
# ../extension.mk. See `docs/scope/extensions/installed-only-model.md`.

EXT_ID    := com.nubeio.rubixos
BIN_NAME  := rubix-nubeio-rubixos-extension
HAS_UI    := 1

include ../extension.mk

# ---------------------------------------------------------------------------
# pg_restore loader. Default container is the rubix-postgres service
# from `rubix/docker/`. Override with `make load-dump CONTAINER=… DUMP=…`.
# ---------------------------------------------------------------------------
DUMP       ?= /home/user/Documents/db.dump
CONTAINER  ?=
TENANT_ID  ?= *

.PHONY: load-dump

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
