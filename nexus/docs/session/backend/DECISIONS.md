# Nexus Backend — Resolved Decisions

Decisions made during the autonomous backend build. Each is a one-liner with the
rationale that justified it. Newest first.

## D2 — R8: SSE auth = short-lived signed stream token in the URL (not cookie)

`POST /streams` (Bearer-authed) mints an HMAC-signed, ~60s-TTL token bound to the
stream registry key (spec + datasource + tenant + required permission); `GET
/streams/:id` reads it from the query string and verifies it. **Chosen over an
`HttpOnly` cookie** because the frontend is a separate-origin SPA already on Bearer
for REST — a cookie path drags in CSRF + CORS-credentials handling for one route,
while a per-subscription signed token is least-privilege (it authorizes exactly one
stream, not the whole session) and needs no extra browser state. Native `EventSource`
can't set headers, and the token-in-URL is acceptable because the token is
short-lived, single-audience, and carries no standing credential.

## D3 — ArkFlow native build deps (Risk #4): curl headers via a user prefix

`arkflow-plugin` unconditionally links `rdkafka`, whose cmake-vendored
`librdkafka` build needs `curl/curl.h` + `libcurl` — and the plugin crate has **no
feature gates** to drop the Kafka connector. On a host without
`libcurl4-openssl-dev`, the whole backend fails to compile (the POC fails here
too, identically). Resolved without root by extracting the distro dev headers
into `~/.local/{include,lib}` and pointing the C toolchain at them via
`nexus/backend/.cargo/config.toml` (`CPATH`/`LIBRARY_PATH`). The proper long-term
fix is upstream feature-gating of arkflow-plugin's connectors; until then this is
the documented operational cost of the ArkFlow dependency. The `[env]` entries are
`force = false`, so a host with the package installed (headers in `/usr/include`)
is unaffected.

## D1 — Risk #17: ArkFlow is on the M0 critical path (option a), not deferred to M3

The standalone POC (`nexus/poc/backend`, arkflow rev `b8f82b3`) already proves the
Collector-sink → `Stream::run(token)` → Arrow→JSON seam end-to-end — the single
biggest ArkFlow risk (Risk #1, request/response-over-streaming) is **already
retired**. Re-targeting M0–M2 onto raw DataFusion+sqlx and swapping ArkFlow back in at
M3 would mean writing the query path twice and discarding a working seam. The git-rev
cancellation API (Risk #5) is likewise confirmed present at that rev. So M0 builds the
real ArkFlow seam now; the DataFusion+sqlx fallback stays unused unless a later
ArkFlow bump breaks the pinned signatures.
