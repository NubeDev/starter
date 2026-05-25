# rubix Makefile — thin wrapper over `mani` + pnpm + docker compose.
#
# Targets:
#   make build    — backend (cargo) + frontend (vite) + pull docker images
#   make start    — Postgres + ClickHouse up, bootstrap operator, then
#                   run rubix-agent (background) and the Vite dev server
#   make restart  — stop running services, then `make start`
#   make clean    — stop services, remove docker volumes, prune build artifacts
#
# Conventions:
#   - Run from rubix/ (this directory). `mani` already cd's into rubix/.
#   - The agent listens on 127.0.0.1:8088; the frontend on 127.0.0.1:5173.
#   - Logs land in /tmp/rubix-agent.log and /tmp/rubix-frontend.log.

SHELL          := /bin/bash
REPO_ROOT      := $(abspath $(CURDIR)/..)
FRONTEND_DIR   := frontend
COMPOSE_FILE   := docker/docker-compose.dev.yaml
AGENT_LOG      := /tmp/rubix-agent.log
FRONTEND_LOG   := /tmp/rubix-frontend.log
AGENT_PORT     := 8088
FRONTEND_PORT  := 5173

RUBIX_DSN      ?= postgres://rubix:rubix-dev@127.0.0.1:5433/rubix
RUBIX_CONFIG   ?= rubix/dev/agent.toml
RUBIX_CH_URL   ?= http://127.0.0.1:8124
CH_EXPLORER_URL ?= http://127.0.0.1:8124

.PHONY: build start restart clean stop deps-up deps-down bootstrap agent frontend help warehouse-seed warehouse-explorer

help:
	@echo "rubix make targets:"
	@echo "  make build    — build backend + frontend, pull docker images"
	@echo "  make start    — bring up Postgres + ClickHouse, bootstrap, run agent + frontend"
	@echo "  make restart  — stop + start"
	@echo "  make clean    — stop, remove docker volumes, cargo + pnpm clean"

build:
	@echo "==> docker compose pull"
	docker compose -f $(COMPOSE_FILE) pull
	@echo "==> mani run build (cargo)"
	mani run --all build
	@echo "==> pnpm install + build (frontend)"
	cd $(FRONTEND_DIR) && pnpm install --frozen-lockfile && pnpm build

deps-up:
	@echo "==> docker compose up -d (Postgres + ClickHouse)"
	docker compose -f $(COMPOSE_FILE) up -d
	@./scripts/wait-for-deps.sh

deps-down:
	@echo "==> docker compose down"
	docker compose -f $(COMPOSE_FILE) down

bootstrap: deps-up
	@echo "==> bootstrap operator (op@example.com)"
	RUBIX_DSN=$(RUBIX_DSN) cargo run -p rubix-agent --bin rubix-admin -- \
	    bootstrap-user --email op@example.com --password rubix-dev-passwd

agent:
	@if ss -tlnp 2>/dev/null | grep -q ':$(AGENT_PORT) '; then \
	    echo "rubix-agent: :$(AGENT_PORT) already in use — skipping launch." >&2; \
	else \
	    echo "==> rubix-agent → $(AGENT_LOG) (cwd=$(REPO_ROOT))"; \
	    cd $(REPO_ROOT) && \
	    RUBIX_DSN=$(RUBIX_DSN) RUBIX_CONFIG=$(RUBIX_CONFIG) RUBIX_CH_URL=$(RUBIX_CH_URL) \
	        nohup cargo run -p rubix-agent > $(AGENT_LOG) 2>&1 & \
	    echo "    pid=$$!"; \
	fi

frontend:
	@if ss -tlnp 2>/dev/null | grep -q '127.0.0.1:$(FRONTEND_PORT) '; then \
	    echo "frontend: 127.0.0.1:$(FRONTEND_PORT) already in use — skipping launch." >&2; \
	else \
	    echo "==> clearing stale vite optimizer cache ($(FRONTEND_DIR)/node_modules/.vite)"; \
	    rm -rf $(FRONTEND_DIR)/node_modules/.vite; \
	    echo "==> vite dev → $(FRONTEND_LOG)"; \
	    cd $(FRONTEND_DIR) && nohup pnpm dev --port $(FRONTEND_PORT) --host 127.0.0.1 --force > $(FRONTEND_LOG) 2>&1 & \
	    echo "    pid=$$!"; \
	fi

start: bootstrap agent frontend
	@WIFI_IP=$$(ip -4 addr show scope global | awk '/inet /{print $$2}' | grep -v '172\.' | head -1 | cut -d/ -f1); \
	echo ""; \
	echo "rubix is starting:"; \
	echo "  backend : http://127.0.0.1:$(AGENT_PORT)   (log: $(AGENT_LOG))"; \
	echo "  frontend: http://127.0.0.1:$(FRONTEND_PORT)   (log: $(FRONTEND_LOG))"; \
	if [ -n "$$WIFI_IP" ]; then \
	    echo ""; \
	    echo "  network (wifi/lan):"; \
	    echo "  backend : http://$$WIFI_IP:$(AGENT_PORT)"; \
	fi

stop:
	@echo "==> stop rubix-agent + vite"
	-@pkill -f 'target/debug/rubix-agent' 2>/dev/null || true
	-@pkill -f 'cargo run -p rubix-agent' 2>/dev/null || true
	-@fuser -k $(FRONTEND_PORT)/tcp 2>/dev/null || true

restart: stop start

clean: stop
	@echo "==> docker compose down -v (drops volumes)"
	docker compose -f $(COMPOSE_FILE) down -v
	@echo "==> cargo clean (rubix crates)"
	-cargo clean -p rubix-spi -p rubix-tools -p rubix-skills -p rubix-flows -p rubix-client -p rubix-agent
	@echo "==> frontend dist + vite cache"
	-rm -rf $(FRONTEND_DIR)/dist $(FRONTEND_DIR)/node_modules/.vite $(FRONTEND_DIR)/.tanstack
	@rm -f $(AGENT_LOG) $(FRONTEND_LOG)

warehouse-seed:
	@echo "==> ch-explorer seed (CH_EXPLORER_URL=$(CH_EXPLORER_URL))"
	CH_EXPLORER_URL=$(CH_EXPLORER_URL) cargo run -p ch-explorer-example -- seed

warehouse-explorer:
	@echo "==> ch-explorer serve → http://127.0.0.1:3030/warehouse/explorer"
	CH_EXPLORER_URL=$(CH_EXPLORER_URL) cargo run -p ch-explorer-example -- serve
