## Done

- Added `rubix/scripts/snapshot-openapi.sh`: builds rubix-agent (release), boots it backgrounded with `RUBIX_BIND=127.0.0.1:0` and no DB/CH DSNs, parses `local_addr=` from the boot log to learn the ephemeral port, curls `/openapi.json`, pretty-prints via `jq -S --indent 2` into `rubix/openapi.json`, tears the child down via `trap` (SIGTERM, ~2s grace, then SIGKILL).
- Extended `rubix-agent::health::serve` to log the bound `local_addr` so the script can discover the ephemeral port (back-compat: the original `bind=…` field is still emitted).
- Generated `rubix/openapi.json` (133 lines, 9 tags, paths `/healthz` + `/api/v1/tools/{tool_id}`, info.title=`rubix-agent`).
- Verified the snapshot is well-formed JSON (`jq -e .`) and byte-deterministic across two runs (`diff` clean).
- Documented the regen flow in `rubix/HOW-TO-CODE.md` §5 ("OpenAPI snapshot regen"): when to regen, the command, and the `chore(rubix): refresh openapi.json snapshot` commit-message convention.
- Committed: `091ab5b stage 7: phase B.3 — snapshot script + committed rubix/openapi.json`.

## Next

- Phase B.4: add CI drift job that re-runs `rubix/scripts/snapshot-openapi.sh` and `git diff --exit-code rubix/openapi.json`.

## What you need to know

- utoipa does not emit timestamps for this document, so no scrubbing pass in `routes/openapi_doc.rs` was needed. The committed snapshot is bit-for-bit reproducible.
- The script sets `NO_COLOR=1` AND strips ANSI defensively before grepping the log — needed because the tracing-subscriber fmt layer was still emitting escape codes when stdout was a regular file in the first run.
- The script uses the no-DB / no-CH laptop boot path (auth/authz/audit middleware not mounted). The OpenAPI doc is independent of that branch; both modes serve the same `RubixApi::openapi()`.
- New log field on `rubix-agent listening`: `local_addr=<addr>` (in addition to existing `bind=<addr>`). Anything grepping the boot log keeps working.

## Open questions

- (none)
