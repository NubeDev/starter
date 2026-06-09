# All-in-one Nexus image: Caddy fronts the static UI and reverse-proxies the API
# routes to the nexus-api binary running in the same container. One process
# manager (Caddy serving + a backgrounded nexus-api) keeps the Fly deploy to a
# single app.
#
# BUILD CONTEXT IS THE REPO ROOT (/home/user/code/rust/starter), not nexus/,
# because both the cargo workspace and the pnpm workspace resolve path/workspace
# deps that live above nexus/. Build with:
#   docker build -f nexus/Dockerfile -t nexus .

# ---- Stage 1: build the Rust backend ----------------------------------------
FROM rust:1.88-slim-bookworm AS backend
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /src
# Copy the whole repo: nexus/backend pulls starter-* crates from ../../crates via
# path deps, so a narrower copy would break resolution.
COPY . .
RUN cargo build --release --manifest-path nexus/backend/Cargo.toml \
    --bin nexus-api --bin seed-admin

# ---- Stage 2: build the UI --------------------------------------------------
FROM node:22-slim AS ui
RUN corepack enable
WORKDIR /src
COPY . .
RUN pnpm install --frozen-lockfile
RUN pnpm --filter @nube/nexus-ui build

# ---- Stage 3: runtime -------------------------------------------------------
FROM caddy:2-alpine
# nexus-api needs glibc; the slim build target above is gnu, so pull the libs.
RUN apk add --no-cache gcompat ca-certificates
COPY --from=backend /src/nexus/backend/target/release/nexus-api  /usr/local/bin/nexus-api
COPY --from=backend /src/nexus/backend/target/release/seed-admin /usr/local/bin/seed-admin
COPY --from=ui       /src/nexus/ui/dist                          /srv/ui
COPY nexus/deploy/Caddyfile  /etc/caddy/Caddyfile
COPY nexus/deploy/start.sh   /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh

# Caddy listens on 8080 (Fly's internal port); nexus-api stays on loopback:9090
# and is only reachable through Caddy.
ENV NEXUS_BIND=127.0.0.1:9090
EXPOSE 8080
CMD ["/usr/local/bin/start.sh"]
