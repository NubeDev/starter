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
WATCHDOG_LOG   := /tmp/rubix-watchdog.log
AGENT_PORT     := 8088
FRONTEND_PORT  := 5173

RUBIX_DSN      ?= postgres://rubix:rubix-dev@127.0.0.1:5433/rubix
RUBIX_CONFIG   ?= rubix/dev/agent.toml
RUBIX_CH_URL   ?= http://127.0.0.1:8124
CH_EXPLORER_URL ?= http://127.0.0.1:8124

# Set REMOTE=1 to run the LOCAL agent against the REMOTE database
# (rubix/dev/agent.remote.toml) instead of the local docker Postgres.
# The agent process still runs on this machine — only its database is
# remote. The DB password is read from the age secrets store via
# `database_password_secret` in that config, so nothing secret is
# passed on the command line. Store it once:
#   cargo run -p rubix-agent --bin rubix-admin -- \
#       secrets set db:password '<pwd>' --path ~/.config/rubix/secrets
#
# With REMOTE=1, `make start` / `make restart` skip `deps-up` +
# `bootstrap` (no local Postgres) and free port 8088 first, so the
# remote-DB agent replaces any local one. Works the same on a plain
# `make agent`. Combine with CONSOLE=1 as usual.
REMOTE ?= 0
ifeq ($(REMOTE),1)
RUBIX_CONFIG  := rubix/dev/agent.remote.toml
RUBIX_DSN     :=
START_PREREQS := stop agent wait-agent frontend
else
START_PREREQS := bootstrap agent wait-agent frontend
endif

.PHONY: build start restart clean stop deps-up deps-down bootstrap agent wait-agent frontend help warehouse-seed warehouse-explorer

help:
	@echo "rubix make targets:"
	@echo "  make build    — build backend + frontend, pull docker images"
	@echo "  make start    — bring up Postgres + ClickHouse, bootstrap, run agent + frontend"
	@echo "  make start REMOTE=1 — run the local agent against the REMOTE DB (no local Postgres)"
	@echo "  make restart  — stop + start  (add REMOTE=1 for the remote DB)"
	@echo "  make restart-console — restart with tokio-console layer (port 6669)"
	@echo "  make watchdog — run external liveness supervisor (foreground)"
	@echo "  make watchdog-bg / watchdog-stop — daemonised variants"
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

# Set CONSOLE=1 to build + launch with the tokio-console layer
# wired in. Adds the `--cfg tokio_unstable` rustflag and
# `--features tokio-console` so the layer is compiled. The console
# server binds 127.0.0.1:6669; connect with `tokio-console`.
#
# Trade-off: the cfg flag invalidates the cargo cache the first
# time you toggle CONSOLE, so the next build with or without it
# rebuilds the whole workspace. Keep CONSOLE=1 set across a debug
# session to avoid bouncing the cache.
CONSOLE ?= 0
ifeq ($(CONSOLE),1)
AGENT_CARGO_EXTRA    := --features tokio-console
AGENT_RUSTFLAGS      := --cfg tokio_unstable
AGENT_CONSOLE_BANNER := echo "    tokio-console : 127.0.0.1:6669  (connect: tokio-console)"
else
AGENT_CARGO_EXTRA    :=
AGENT_RUSTFLAGS      :=
AGENT_CONSOLE_BANNER := true
endif

agent:
	@if ss -tlnp 2>/dev/null | grep -q ':$(AGENT_PORT) '; then \
	    echo "rubix-agent: :$(AGENT_PORT) already in use — skipping launch." >&2; \
	else \
	    echo "==> rubix-agent → $(AGENT_LOG) (cwd=$(REPO_ROOT))"; \
	    $(AGENT_CONSOLE_BANNER); \
	    cd $(REPO_ROOT) && \
	    RUBIX_DSN=$(RUBIX_DSN) RUBIX_CONFIG=$(RUBIX_CONFIG) RUBIX_CH_URL=$(RUBIX_CH_URL) \
	    RUSTFLAGS="$(AGENT_RUSTFLAGS)" \
	        nohup cargo run -p rubix-agent $(AGENT_CARGO_EXTRA) > $(AGENT_LOG) 2>&1 & \
	    echo "    pid=$$!"; \
	fi

# Block until the agent's HTTP listener is accepting connections.
# Without this, Vite starts proxying before boot finishes and the
# browser sees `500 Internal Server Error` (proxy ECONNREFUSED) for
# every /api/* call until the agent logs `rubix-agent listening`.
# Cold builds can take well over a minute, so we wait up to 5 min.
wait-agent:
	@echo "==> waiting for rubix-agent on 127.0.0.1:$(AGENT_PORT) (up to 300s; tail $(AGENT_LOG) for detail)"
	@start=$$(date +%s); \
	last_phase=""; \
	for i in $$(seq 1 600); do \
	    if curl -fsS -o /dev/null -m 1 http://127.0.0.1:$(AGENT_PORT)/livez 2>/dev/null \
	       || curl -sS -o /dev/null -m 1 http://127.0.0.1:$(AGENT_PORT)/api/v1/auth/me 2>/dev/null; then \
	        echo "    agent ready (after $$(( $$(date +%s) - start ))s)"; \
	        exit 0; \
	    fi; \
	    if ! pgrep -f 'target/debug/rubix-agent' >/dev/null \
	       && ! pgrep -f 'cargo.*run.*-p rubix-agent' >/dev/null; then \
	        echo "wait-agent: rubix-agent process is not running — check $(AGENT_LOG)" >&2; \
	        tail -n 30 $(AGENT_LOG) >&2 2>/dev/null || true; \
	        exit 1; \
	    fi; \
	    if [ $$(( i % 20 )) -eq 0 ]; then \
	        elapsed=$$(( $$(date +%s) - start )); \
	        if pgrep -f 'target/debug/rubix-agent' >/dev/null; then \
	            phase="booting (process up, waiting for HTTP listener)"; \
	        else \
	            phase="compiling (cargo build in progress)"; \
	        fi; \
	        if [ "$$phase" != "$$last_phase" ] || [ $$(( i % 60 )) -eq 0 ]; then \
	            echo "    ... $${elapsed}s elapsed — $$phase"; \
	            last_phase="$$phase"; \
	        fi; \
	    fi; \
	    sleep 0.5; \
	done; \
	echo "wait-agent: timed out after 300s — check $(AGENT_LOG)" >&2; \
	tail -n 30 $(AGENT_LOG) >&2 2>/dev/null || true; \
	exit 1

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

start: $(START_PREREQS)
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
	@# Block until port $(AGENT_PORT) is actually free before
	@# returning. Without this, `make restart` races: `start` runs
	@# `agent`, which skips launch when `ss` still sees the dying
	@# listener in TIME_WAIT or LISTEN, and no new agent comes up.
	@# 20 × 250ms = 5s ceiling — well past any realistic TIME_WAIT
	@# on loopback. SO_REUSEADDR on the listener means TIME_WAIT
	@# alone wouldn't normally block bind(), but the agent's TCP
	@# listener doesn't set it, so we wait the full window.
	@for i in $$(seq 1 20); do \
	    if ! ss -tlnp 2>/dev/null | grep -q ':$(AGENT_PORT) '; then \
	        exit 0; \
	    fi; \
	    sleep 0.25; \
	done; \
	echo "stop: port $(AGENT_PORT) still bound after 5s — restart may fail" >&2

restart: stop start

# Convenience: restart with the tokio-console layer wired. First
# build re-compiles the workspace under `--cfg tokio_unstable`, so
# expect ~30s extra. Subsequent restart-console calls are fast.
# Connect from another terminal: `tokio-console`.
restart-console:
	$(MAKE) CONSOLE=1 restart

# External liveness supervisor. Probes /livez every 10s; on 3
# consecutive failures (~30s wedged), SIGUSR1 → SIGABRT → restart.
# See `rubix/crates/rubix-watchdog/src/main.rs` for the env var
# contract. Runs in the foreground so Ctrl+C stops it without
# touching the agent. NOTE: if you Ctrl+C *while a restart is in
# flight*, the terminal SIGINT propagates to the still-building
# `cargo run` child and the new agent never starts — use
# `watchdog-bg` if that's a concern.
watchdog:
	@echo "==> rubix-watchdog → stdout (Ctrl+C to stop; agent left running)"
	@RUBIX_WATCHDOG_RESTART_CMD='cd $(CURDIR) && $(MAKE) restart' \
	    cargo run -p rubix-watchdog

# Daemonised variant. Logs to $(WATCHDOG_LOG). Use this when you
# want the watchdog to survive your terminal closing.
watchdog-bg:
	@if pgrep -f 'target/debug/rubix-watchdog' >/dev/null; then \
	    echo "rubix-watchdog already running — skipping launch." >&2; \
	else \
	    echo "==> rubix-watchdog → $(WATCHDOG_LOG)"; \
	    RUBIX_WATCHDOG_RESTART_CMD='cd $(CURDIR) && $(MAKE) restart' \
	        nohup cargo run -p rubix-watchdog > $(WATCHDOG_LOG) 2>&1 & \
	    echo "    pid=$$!"; \
	fi

watchdog-stop:
	@echo "==> stop rubix-watchdog (agent left running)"
	-@pkill -f 'target/debug/rubix-watchdog' 2>/dev/null || true

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
