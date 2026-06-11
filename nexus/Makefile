# Nexus — backend (Rust/cargo) + UI (pnpm/vite) developer tasks.
#
#   make build       build backend + UI
#   make dev         run backend + UI together (Ctrl-C stops both)
#   make dev-all     fresh start: bootstrap + seed test data + run (one command)
#   make dev-be      run just the backend (needs a database — see `make db`)
#   make dev-ui      run just the UI
#   make test        run backend + UI tests
#   make db          start a local Postgres for the backend in Docker
#   make db-stop     stop and remove that Postgres container
#   make nats-install   build/start a no-auth NATS broker for testing
#   make zenoh-install  build/start a no-auth Zenoh router for testing
#   make brokers        start both testing brokers
#   make brokers-stop   stop and remove both testing brokers
#   make bootstrap   from zero: start db, build, seed the admin user
#   make seed        seed/refresh the admin tenant + user + grant (idempotent)
#   make seed-sim    seed synthetic test data (hvac/energy/door) into the datasource
#   make docker-build  build the all-in-one image (Caddy + UI + nexus-api)
#   make docker-run    run that image locally against `make db`
#   make fly-secrets   push DB url + keys to the Fly app
#   make fly-deploy    deploy the all-in-one image to Fly.io
#   make clean       remove build artifacts (cargo target + UI dist)
#   make kill        stop anything left running on the dev ports
#   make build-be    build just the backend
#   make build-ui    build just the UI

# The UI is a pnpm workspace rooted at the repo root, so its commands run from
# there with a package filter; the backend is the cargo workspace in backend/.
ROOT     := $(abspath $(CURDIR)/..)
BE_DIR   := backend
UI_PKG   := @nube/nexus-ui

# Dev ports — kept in sync with the code: nexus-api binds 127.0.0.1:4780
# (NEXUS_BIND in backend/crates/nexus-api/src/main.rs), vite serves :4790
# (nexus/ui/vite.config.ts). Deliberately off the common defaults (8080/5273)
# and grouped in the project's 47xx block (db 4770, api 4780, ui 4790) so nexus
# never collides with another local service. Override on the CLI to remap them.
BE_PORT  ?= 4780
UI_PORT  ?= 4790

# Optional cargo features for the backend, applied by every target that runs or
# builds nexus-api (dev, dev-be, dev-all, build, build-be). Empty by default;
# set FEATURES=zenoh to compile in the Zenoh ingest source (needed for the
# datapump --transport zenoh testing path). Examples:
#   make dev FEATURES=zenoh
#   make dev-be FEATURES=zenoh
FEATURES ?=
CARGO_FEATURES := $(if $(FEATURES),--features $(FEATURES),)

# Local dev Postgres (see `make db`). The backend reads its connection + key
# material from these env vars (backend/crates/nexus-api/src/main.rs):
# NEXUS_MASTER_KEY is validated as exactly 32 bytes and NEXUS_STREAM_TOKEN_KEY
# as >=32, so the dev defaults below are sized to match. Override for anything
# beyond local dev — these are not secrets.
DB_CONTAINER ?= nexus-dev-pg
DB_PORT      ?= 4770
DB_USER      ?= nexus
DB_PASS      ?= nexus
DB_NAME      ?= nexus
DB_URL       ?= postgres://$(DB_USER):$(DB_PASS)@127.0.0.1:$(DB_PORT)/$(DB_NAME)

# No-auth local brokers for tests and manual integration work. These use
# Dockerfiles under docker/ so the test images and exposed ports are explicit.
NATS_CONTAINER ?= nexus-test-nats
NATS_IMAGE     ?= nexus/test-nats:local
NATS_PORT      ?= 4222
NATS_HTTP_PORT ?= 8222

ZENOH_CONTAINER ?= nexus-test-zenoh
ZENOH_IMAGE     ?= nexus/test-zenoh:local
ZENOH_PORT      ?= 7447
ZENOH_REST_PORT ?= 8000

export NEXUS_METADATA_URL     ?= $(DB_URL)
export NEXUS_DATASOURCE_URL   ?= $(DB_URL)
export NEXUS_MASTER_KEY       ?= 0123456789abcdef0123456789abcdef
export NEXUS_STREAM_TOKEN_KEY ?= 0123456789abcdef0123456789abcdef

# The built-in query-kinds and datasource-kinds packs ship inside the nexus-api
# crate. The dev targets run the server with CWD=$(BE_DIR) (`cd $(BE_DIR) &&
# cargo run`), so the in-code defaults (`./kinds`, `./datasource-kinds`) resolve
# to backend/kinds — which doesn't exist — and the Explore "Kind" picker shows
# "No kinds". Point them at the real pack dirs (relative to $(BE_DIR)).
export NEXUS_KINDS_DIR            ?= crates/nexus-api/kinds
export NEXUS_DATASOURCE_KINDS_DIR ?= crates/nexus-api/datasource-kinds
# WS-14 extensions runtime. The read-only in-repo pack lives at `nexus/extensions/`
# (a top-level sibling of `backend/`, matching rubix's `rubix/extensions/` layout)
# — so from $(BE_DIR) it resolves as `../extensions`. The installs + pidfile dirs
# go to a gitignored scratch dir under $(BE_DIR) so `make dev-be` can accept uploads
# and the supervisor can write pidfiles without polluting the source tree.
export NEXUS_EXTENSIONS_DIR          ?= ../extensions
export NEXUS_EXTENSIONS_INSTALLS_DIR ?= .nexus-ext/installs
export NEXUS_EXTENSIONS_PIDFILE_DIR  ?= .nexus-ext/pids

# Admin seed credentials (consumed by `make seed` / the seed-admin binary).
# Dev defaults — override for any real deploy.
export ADMIN_EMAIL    ?= admin@nexus.local
export ADMIN_PASSWORD ?= change-me-admin

# Fly app + the all-in-one image tag. The Docker build context is the repo root
# (ROOT) because both workspaces resolve deps above nexus/.
FLY_APP    ?= nexus
IMAGE      ?= nexus:local

.PHONY: build dev dev-all dev-be dev-ui test soak db db-stop \
        nats-build nats-install nats-stop zenoh-build zenoh-install zenoh-stop brokers brokers-stop \
        bootstrap seed seed-sim docker-build docker-run fly-secrets fly-deploy clean kill \
        build-be build-ui

# Rows per profile for `make seed-sim` (hvac/energy/door); override on the CLI.
SIM_ROWS ?= 200

build: build-be build-ui

build-be:
	cd $(BE_DIR) && cargo build $(CARGO_FEATURES)

build-ui:
	cd $(ROOT) && pnpm --filter $(UI_PKG) build

# Run both in one foreground process group; Ctrl-C (or `make kill`) stops both.
# The backend logs prefix-free; the trap reaps the child on exit so no orphan
# server keeps the port held. Depends on `db` so a stopped/missing dev Postgres
# is (re)started first — otherwise the backend exits on a connection refusal at
# startup and the UI just shows proxy ECONNREFUSED with no hint why.
dev: db
	@echo "backend → http://127.0.0.1:$(BE_PORT)   ui → http://127.0.0.1:$(UI_PORT)"
	@trap 'kill 0' EXIT INT TERM; \
	( cd $(BE_DIR) && cargo run --bin nexus-api $(CARGO_FEATURES) ) & \
	( cd $(ROOT) && pnpm --filter $(UI_PKG) dev ) & \
	wait

# Run one side alone. dev-be applies migrations at startup against the database
# pointed to by NEXUS_METADATA_URL; depends on `db` so that database is up first.
dev-be: db
	cd $(BE_DIR) && cargo run --bin nexus-api $(CARGO_FEATURES)

dev-ui:
	cd $(ROOT) && pnpm --filter $(UI_PKG) dev

# Backend integration tests stand up their own Postgres via testcontainers and
# are #[ignore]d by default, so `--include-ignored` opts them in; they need a
# running Docker daemon, not `make db`.
test:
	cd $(BE_DIR) && cargo test -- --include-ignored
	cd $(ROOT) && pnpm --filter $(UI_PKG) test

# Backpressure soak (RW-08): a high-rate flow into a Docker Postgres datasource,
# asserting bounded RSS, zero lost rows, a sliced fat batch, and a DB-fault chaos
# case. #[ignore]d so it never runs in the normal suite; this target opts it in.
# Needs a running Docker daemon. Tune length/rate with NEXUS_SOAK_SECS (default
# 20; set 600 for the documented ≥10-minute run) and NEXUS_SOAK_BATCH. See
# docs/scope/nextgen/rewrite/BACKPRESSURE.md for runtime + interpretation.
soak:
	cd $(BE_DIR) && cargo test -p nexus-api --features testing \
		--test routes_soak_backpressure -- --ignored --nocapture

# A throwaway Postgres for running the backend locally. Idempotent: re-running
# reuses the existing container. The backend applies its own migrations on
# startup, so this only provides the empty database.
db:
	@existing="$$(docker ps -aq -f name=^/$(DB_CONTAINER)$$)"; \
	if [ -n "$$existing" ]; then \
		mapped="$$(docker port $(DB_CONTAINER) 5432/tcp 2>/dev/null | head -1 | sed 's/.*://')"; \
		if [ "$$mapped" != "$(DB_PORT)" ]; then \
			echo "recreating $(DB_CONTAINER): mapped :$$mapped != wanted :$(DB_PORT)"; \
			docker rm -f $(DB_CONTAINER) >/dev/null; existing=""; \
		fi; \
	fi; \
	if [ -z "$$existing" ]; then \
		docker run -d --name $(DB_CONTAINER) \
			-e POSTGRES_USER=$(DB_USER) -e POSTGRES_PASSWORD=$(DB_PASS) -e POSTGRES_DB=$(DB_NAME) \
			-p $(DB_PORT):5432 postgres:16-alpine >/dev/null; \
		echo "started $(DB_CONTAINER) on :$(DB_PORT)"; \
	else \
		docker start $(DB_CONTAINER) >/dev/null; \
		echo "reused $(DB_CONTAINER) on :$(DB_PORT)"; \
	fi
	@printf "waiting for postgres "; \
	for i in $$(seq 1 60); do \
		if docker exec $(DB_CONTAINER) pg_isready -U $(DB_USER) -q 2>/dev/null; then echo "ready"; exit 0; fi; \
		printf "."; sleep 0.5; \
	done; \
	echo "timed out waiting for $(DB_CONTAINER)"; exit 1

db-stop:
	-@docker rm -f $(DB_CONTAINER) >/dev/null 2>&1 || true
	@echo "removed $(DB_CONTAINER)"

nats-build:
	docker build -f docker/testing-nats.Dockerfile -t $(NATS_IMAGE) docker

nats-install: nats-build
	@existing="$$(docker ps -aq -f name=^/$(NATS_CONTAINER)$$)"; \
	if [ -n "$$existing" ]; then \
		client_port="$$(docker port $(NATS_CONTAINER) 4222/tcp 2>/dev/null | head -1 | sed 's/.*://')"; \
		http_port="$$(docker port $(NATS_CONTAINER) 8222/tcp 2>/dev/null | head -1 | sed 's/.*://')"; \
		if [ "$$client_port" != "$(NATS_PORT)" ] || [ "$$http_port" != "$(NATS_HTTP_PORT)" ]; then \
			echo "recreating $(NATS_CONTAINER): mapped :$$client_port/:$$http_port != wanted :$(NATS_PORT)/:$(NATS_HTTP_PORT)"; \
			docker rm -f $(NATS_CONTAINER) >/dev/null; existing=""; \
		fi; \
	fi; \
	if [ -z "$$existing" ]; then \
		docker run -d --name $(NATS_CONTAINER) \
			-p $(NATS_PORT):4222 -p $(NATS_HTTP_PORT):8222 \
			$(NATS_IMAGE) >/dev/null; \
		echo "started $(NATS_CONTAINER): nats://127.0.0.1:$(NATS_PORT), http://127.0.0.1:$(NATS_HTTP_PORT)"; \
	else \
		docker start $(NATS_CONTAINER) >/dev/null; \
		echo "reused $(NATS_CONTAINER): nats://127.0.0.1:$(NATS_PORT), http://127.0.0.1:$(NATS_HTTP_PORT)"; \
	fi

nats-stop:
	-@docker rm -f $(NATS_CONTAINER) >/dev/null 2>&1 || true
	@echo "removed $(NATS_CONTAINER)"

zenoh-build:
	docker build -f docker/testing-zenoh.Dockerfile -t $(ZENOH_IMAGE) docker

zenoh-install: zenoh-build
	@existing="$$(docker ps -aq -f name=^/$(ZENOH_CONTAINER)$$)"; \
	if [ -n "$$existing" ]; then \
		router_port="$$(docker port $(ZENOH_CONTAINER) 7447/tcp 2>/dev/null | head -1 | sed 's/.*://')"; \
		rest_port="$$(docker port $(ZENOH_CONTAINER) 8000/tcp 2>/dev/null | head -1 | sed 's/.*://')"; \
		if [ "$$router_port" != "$(ZENOH_PORT)" ] || [ "$$rest_port" != "$(ZENOH_REST_PORT)" ]; then \
			echo "recreating $(ZENOH_CONTAINER): mapped :$$router_port/:$$rest_port != wanted :$(ZENOH_PORT)/:$(ZENOH_REST_PORT)"; \
			docker rm -f $(ZENOH_CONTAINER) >/dev/null; existing=""; \
		fi; \
	fi; \
	if [ -z "$$existing" ]; then \
		docker run -d --init --name $(ZENOH_CONTAINER) \
			-p $(ZENOH_PORT):7447/tcp -p $(ZENOH_REST_PORT):8000/tcp \
			$(ZENOH_IMAGE) >/dev/null; \
		echo "started $(ZENOH_CONTAINER): tcp/127.0.0.1:$(ZENOH_PORT), http://127.0.0.1:$(ZENOH_REST_PORT)"; \
	else \
		docker start $(ZENOH_CONTAINER) >/dev/null; \
		echo "reused $(ZENOH_CONTAINER): tcp/127.0.0.1:$(ZENOH_PORT), http://127.0.0.1:$(ZENOH_REST_PORT)"; \
	fi

zenoh-stop:
	-@docker rm -f $(ZENOH_CONTAINER) >/dev/null 2>&1 || true
	@echo "removed $(ZENOH_CONTAINER)"

brokers: nats-install zenoh-install

brokers-stop: nats-stop zenoh-stop

# Zero-to-running: database up, everything built, admin seeded. After this,
# `make dev` (or `make dev-be`) serves against a seeded database.
bootstrap: db build seed
	@echo "bootstrap done — run 'make dev', then log in as $(ADMIN_EMAIL)"

# One command for a fresh start: bootstrap (db + build + admin seed), then load
# synthetic test data, then run backend + UI. The steps are ordered explicitly
# so seed-sim runs after migrations are applied (seed-admin applies them) and
# dev runs last. Ctrl-C stops the servers; re-running is safe (all steps are
# idempotent except seed-sim, which appends another SIM_ROWS batch).
dev-all:
	$(MAKE) bootstrap
	$(MAKE) seed-sim
	$(MAKE) dev

# Seed (or refresh) the admin tenant + user + admin grant. Idempotent — the
# binary applies migrations first, so it works against an empty database too.
# Override ADMIN_EMAIL / ADMIN_PASSWORD on the command line for real creds.
seed:
	cd $(BE_DIR) && cargo run --bin seed-admin

# WS-14 example extension passthroughs: `make ext-hello-<target>` /
# `make ext-demo-<target>` run <target> in that bundle's own Makefile against
# the dev server — e.g. `make ext-demo-ui-build`, `make ext-hello-test`,
# `make ext-demo-install`, `make ext-hello-status`. The bundles live at the
# top-level `nexus/extensions/` (rubix-style layout). Admin creds + API port
# flow through the exported env above.
ext-hello-%:
	$(MAKE) -C extensions/com.nexus.hello $*

ext-demo-%:
	$(MAKE) -C extensions/com.nexus.demo $*

# Seed synthetic test data into the datasource DB: SIM_ROWS rows each of three
# device profiles (hvac numeric floats, energy monotonic counter, door bool+str)
# into sim_hvac / sim_energy / sim_door. Re-running appends another batch.
seed-sim:
	cd $(BE_DIR) && DATABASE_URL="$(NEXUS_DATASOURCE_URL)" cargo run --bin nexus-seed -- $(SIM_ROWS)

# Build the all-in-one image. Context = repo root so path/workspace deps
# resolve; Dockerfile lives under nexus/.
docker-build:
	cd $(ROOT) && docker build -f nexus/Dockerfile -t $(IMAGE) .

# Run the image locally, talking to the `make db` Postgres. host.docker.internal
# lets the container reach the db published on the host; SEED_ADMIN seeds on boot.
docker-run:
	docker run --rm -p $(BE_PORT):8080 \
		--add-host host.docker.internal:host-gateway \
		-e NEXUS_METADATA_URL="postgres://$(DB_USER):$(DB_PASS)@host.docker.internal:$(DB_PORT)/$(DB_NAME)" \
		-e NEXUS_DATASOURCE_URL="postgres://$(DB_USER):$(DB_PASS)@host.docker.internal:$(DB_PORT)/$(DB_NAME)" \
		-e NEXUS_MASTER_KEY="$(NEXUS_MASTER_KEY)" \
		-e NEXUS_STREAM_TOKEN_KEY="$(NEXUS_STREAM_TOKEN_KEY)" \
		-e SEED_ADMIN=1 \
		$(IMAGE)

# Push runtime secrets to Fly. Set NEXUS_METADATA_URL / NEXUS_DATASOURCE_URL to
# your Fly Postgres DSN and the keys to real values before running. The dev
# defaults exported above are a fallback so this never pushes an empty value —
# override them for a real deploy.
fly-secrets:
	fly secrets set --app $(FLY_APP) \
		NEXUS_METADATA_URL="$(NEXUS_METADATA_URL)" \
		NEXUS_DATASOURCE_URL="$(NEXUS_DATASOURCE_URL)" \
		NEXUS_MASTER_KEY="$(NEXUS_MASTER_KEY)" \
		NEXUS_STREAM_TOKEN_KEY="$(NEXUS_STREAM_TOKEN_KEY)" \
		ADMIN_EMAIL="$(ADMIN_EMAIL)" \
		ADMIN_PASSWORD="$(ADMIN_PASSWORD)"

# Deploy from the repo root so Fly uses the root build context with nexus/fly.toml.
fly-deploy:
	cd $(ROOT) && fly deploy --app $(FLY_APP) --config nexus/fly.toml

clean:
	cd $(BE_DIR) && cargo clean
	rm -rf ui/dist

# Free the dev ports if a previous run was killed ungracefully and left a
# listener behind. Hard kill (SIGKILL) via fuser so a wedged process can't
# ignore it. Best-effort: fuser exits non-zero when nothing is bound, so the
# leading '-' keeps make from failing.
kill:
	-@fuser -k -KILL $(BE_PORT)/tcp 2>/dev/null || true
	-@fuser -k -KILL $(UI_PORT)/tcp 2>/dev/null || true
	@echo "freed ports $(BE_PORT) and $(UI_PORT)"
