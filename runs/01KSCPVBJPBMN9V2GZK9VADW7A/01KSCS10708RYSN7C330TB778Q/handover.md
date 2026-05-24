## Done

- Added POST /extensions/install (multipart tarball; JSON registry-URL → HTTP 501 stub) and DELETE /extensions/{id} in upstream starter-ext-server (src/lifecycle.rs); registered routes in the gated router.
- Plumbed extensions_dir through ExtensionAdmin builder; rubix-agent boot/extensions.rs passes cfg.extensions.dir so handlers can stage/extract/remove bundles.
- Added 4 message keys (rubix.extension.install.{succeeded,invalid_manifest}, uninstall.{succeeded,not_found}) to en.json + es.json same commit.
- Added starter-ext-server/tests/lifecycle_routes.rs: full tarball roundtrip, install_invalid_manifest, uninstall_missing — all 3 pass.
- cargo build -p rubix-agent green; cargo test -p starter-ext-server (lib + lifecycle_routes) green.
- Committed as 11bf66b "phase D.1 — install/uninstall extension endpoints".

## Next

- (none) — fresh session picks up the next stage.

## What you need to know

- Hot-mount-after-seal is intentionally out of scope: install extracts + validates + persists Enabled, but the new extension only surfaces on the next agent boot (ExtensionRegistry is sealed after Loader::commit). Uninstall stops live supervisor (if any), removes bundle dir, persists Disabled — works without a registry entry.
- Handlers return a stable upstream `code` string (e.g. `install.succeeded`); the rubix-spi catalog keys are `rubix.extension.<code>` — rubix layer maps as it formats. No upstream coupling to the rubix.* prefix.
- Multipart parsing requires axum's `multipart` feature; added at the workspace level in starter-extensions/Cargo.toml together with tar + flate2 deps. The test fabricates multipart bodies manually (no test-only dep needed).
- Tarball extractor rejects absolute paths and `..` components; skips symlinks/hardlinks/devices. A single-top-dir-wrapped tarball is auto-promoted to the bundle root.

## Open questions

- The integration test lives in upstream starter-ext-server (where the logic is) rather than rubix-agent/tests/; if the stage gate expects rubix-agent/tests/ specifically, lift the test into a thin rubix-agent wrapper that exercises the same /api/v1/extensions/* surface end-to-end via TestApp + PgEnablementStore.
