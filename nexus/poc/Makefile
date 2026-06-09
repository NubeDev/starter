# Nexus · ArkFlow POC — build / dev helpers.
#
#   make install     install UI deps
#   make build       build backend (release) + UI
#   make dev         run backend + UI together (Ctrl-C stops both)
#   make backend     run backend only (debug)
#   make frontend    run UI dev server only
#   make clean       remove build artifacts

# protoc is needed to compile the engine. Use one on PATH, else the POC-local copy.
PROTOC ?= $(shell command -v protoc 2>/dev/null || echo /tmp/protoc-local/bin/protoc)
export PROTOC

BACKEND_DIR := backend
UI_DIR      := ui

.PHONY: install build build-backend build-ui dev backend frontend clean

install:
	cd $(UI_DIR) && npm install

build: build-backend build-ui

build-backend:
	cd $(BACKEND_DIR) && cargo build --release

build-ui:
	cd $(UI_DIR) && npm run build

# Run both at once. Backend runs in the background; the UI dev server holds the
# foreground. A trap kills the backend when you Ctrl-C or the UI exits.
dev:
	@command -v $(PROTOC) >/dev/null 2>&1 || { echo "protoc not found (set PROTOC=...)"; exit 1; }
	@echo "▶ backend → http://127.0.0.1:8787   ▶ ui → http://localhost:5274"
	@trap 'kill 0' INT TERM EXIT; \
	( cd $(BACKEND_DIR) && cargo run ) & \
	( cd $(UI_DIR) && npm run dev ) ; \
	wait

backend:
	cd $(BACKEND_DIR) && cargo run

frontend:
	cd $(UI_DIR) && npm run dev

clean:
	cd $(BACKEND_DIR) && cargo clean
	rm -rf $(UI_DIR)/dist $(UI_DIR)/node_modules
