#!/bin/sh
# Container entrypoint: run the API in the background, then Caddy in the
# foreground. If the API dies, take the container down with it so Fly restarts
# the machine rather than serving a UI with a dead backend.
set -e

# Optional one-shot admin seed on boot. Enable by setting SEED_ADMIN=1 (the
# seed binary is idempotent, so it is safe to leave on). Reads ADMIN_* and
# NEXUS_METADATA_URL from the environment, same as the binary documents.
if [ "${SEED_ADMIN:-0}" = "1" ]; then
	echo "seeding admin..."
	/usr/local/bin/seed-admin || echo "seed-admin failed (continuing)"
fi

/usr/local/bin/nexus-api &
api_pid=$!

# If nexus-api exits, stop Caddy so the whole container exits non-zero.
trap 'kill "$api_pid" 2>/dev/null || true' EXIT
( wait "$api_pid"; echo "nexus-api exited"; kill 1 2>/dev/null ) &

exec caddy run --config /etc/caddy/Caddyfile --adapter caddyfile
