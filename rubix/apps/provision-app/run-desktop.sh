#!/usr/bin/env bash
# Launch the Tauri desktop app from a CLEAN environment.
#
# Why: this repo is opened in the Snap-packaged VS Code, whose integrated
# terminal exports GTK_PATH / LOCPATH / GIO_MODULE_DIR / etc. pointing into
# /snap/code/. When the native binary loads GTK+WebKit it then pulls Snap's
# bundled libs, and /snap/core20 libpthread collides with the system glibc:
#   symbol lookup error: __libc_pthread_init, version GLIBC_PRIVATE
# Stripping the Snap vars and running under the system loader fixes it.
#
# Usage: ./run-desktop.sh            (full cargo tauri dev: vite + native shell)
#        ./run-desktop.sh binary     (run the already-built debug binary only)
set -euo pipefail
cd "$(dirname "$0")"

# A minimal, Snap-free environment. Keep display/session bits so the window
# can actually open; drop everything that points into /snap.
clean_env=(
  env -i
  HOME="$HOME"
  USER="${USER:-user}"
  PATH="$HOME/.cargo/bin:$HOME/.nvm/versions/node/v22.22.0/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
  DISPLAY="${DISPLAY:-:0}"
  WAYLAND_DISPLAY="${WAYLAND_DISPLAY:-}"
  XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}"
  XDG_SESSION_TYPE="${XDG_SESSION_TYPE:-x11}"
  DBUS_SESSION_BUS_ADDRESS="${DBUS_SESSION_BUS_ADDRESS:-}"
  # Force the system WebKit DMABUF off — avoids GPU/driver fallbacks that
  # also misbehave under stripped environments on some boxes.
  WEBKIT_DISABLE_DMABUF_RENDERER=1
)

if [[ "${1:-}" == "binary" ]]; then
  exec "${clean_env[@]}" ./src-tauri/target/debug/provision-app
else
  exec "${clean_env[@]}" cargo tauri dev
fi
