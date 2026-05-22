# flow-agent example — build, run, kill
# Uses non-standard ports: backend=9741, frontend=9742

BACKEND_PORT  := 9741
FRONTEND_PORT := 9742
PID_DIR       := .pids
DB_URL        := sqlite:./flow-agent.db?mode=rwc

.PHONY: build build-backend build-frontend start stop clean demo

build: build-backend build-frontend

build-backend:
	cargo build -p flow-agent --release

build-frontend:
	cd frontend && pnpm install && pnpm build

start: build-backend stop $(PID_DIR)
	@echo "Starting backend on :$(BACKEND_PORT)..."
	HTTP_BIND="127.0.0.1:$(BACKEND_PORT)" \
	DATABASE_URL="$(DB_URL)" \
	RUST_LOG=info \
		../../target/release/flow-agent \
		> $(PID_DIR)/backend.log 2>&1 & echo $$! > $(PID_DIR)/backend.pid
	@echo "Starting frontend on :$(FRONTEND_PORT)..."
	cd frontend && npx vite --port $(FRONTEND_PORT) --strictPort \
		> ../$(PID_DIR)/frontend.log 2>&1 & echo $$! > $(PID_DIR)/frontend.pid
	@echo "backend  → http://127.0.0.1:$(BACKEND_PORT)"
	@echo "frontend → http://127.0.0.1:$(FRONTEND_PORT)"

stop:
	@if [ -f $(PID_DIR)/backend.pid ]; then \
		kill $$(cat $(PID_DIR)/backend.pid) 2>/dev/null && echo "backend stopped (pidfile)" || true; \
		rm -f $(PID_DIR)/backend.pid; \
	fi
	@if [ -f $(PID_DIR)/frontend.pid ]; then \
		kill $$(cat $(PID_DIR)/frontend.pid) 2>/dev/null && echo "frontend stopped (pidfile)" || true; \
		rm -f $(PID_DIR)/frontend.pid; \
	fi
	@# Fallback: kill orphaned processes (e.g. started from a prior shell
	@# session where the pidfile was lost) so `make start` can rebind.
	@pkill -f 'target/release/flow-agent' 2>/dev/null && echo "backend stopped (orphan)" || true
	@pkill -f 'vite\.js --port $(FRONTEND_PORT)' 2>/dev/null && echo "frontend stopped (orphan)" || true

clean: stop
	rm -rf $(PID_DIR) frontend/dist flow-agent.db

# Seed a demo flow (Trigger → AI Agent → Transform) via the REST API.
# Assumes `make start` is already running.
demo:
	BACKEND_URL=http://127.0.0.1:$(BACKEND_PORT) \
	FRONTEND_URL=http://127.0.0.1:$(FRONTEND_PORT) \
	./scripts/seed-demo.sh

$(PID_DIR):
	mkdir -p $(PID_DIR)
