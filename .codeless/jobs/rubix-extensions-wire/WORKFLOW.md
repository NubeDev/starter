# Workflow — rubix-extensions-wire

## Sequencing

13 stages across five phases. Strict order: A (upstream PG store) → B (example builds) → C (rubix-agent boot wiring) → D (install + frontend) → E (test + docs + PR). Four REVIEW gates total — one between each pair of phases.

This job is **pure wiring**. The `starter-extensions` framework is built; rubix is the first real consumer. Every primitive — host, supervisor, admin router, UI runtime — already exists upstream. The job's value is in proving the composition works and shipping the PG persistence impl every future consumer also needs.

## Per-stage discipline

### Phase A — upstream PG store

R2 strictly. The PG `EnablementStore` impl lives in `starter-extensions/` because `starter-ext-server` already defined the trait there, and every future consumer of `starter-ext-server` will also want a real persistence backend.

1. **Read `starter-ext-server::store` end-to-end** before writing the impl. The trait's contract is short but precise; `set` is UPSERT, not "insert or error". `get` returns `Option`, not "Result with NotFound". The integration test asserts these.
2. **`PgEnablementStore` lives in its own crate, not in `starter-ext-server`.** The store trait belongs with the consumer interface; the impl belongs in a crate that depends on `sqlx`. Keeps `starter-ext-server` free of DB deps for in-memory and SQLite consumers.
3. **The migration is owned by `starter-ext-store-pg`, not by any rubix crate.** Migrations live under `starter-ext-store-pg/migrations/`. The consumer wires the migration runner; in rubix's case that's `boot::extensions.rs`.
4. **Test against testcontainers, not against a shared dev DB.** Required for CI parallelism + reproducibility. Match the test pattern used by `starter-store-postgres`'s existing tests.
5. `cargo test -p starter-ext-store-pg --features testcontainers` and `./rubix/scripts/lint-doc-refs.sh` (if it lints starter-extensions/ — confirm; if not, don't add a rubix lint to that workspace) green.

### Phase B — example extension builds

The example exists but isn't part of any workspace. Per the rubix workspace rule (extensions never depend on rubix-*), the right move is to give `rubix/extensions/` its own workspace.

1. **Don't add `rubix/extensions/com.rubix.example/process` to the parent workspace.** That would let the extension accidentally `path = "../../../crates/rubix-tools"` and break R8. The split workspace prevents the temptation.
2. **The path-dep on `starter-ext-sdk` is workspace-relative across two workspaces.** Both workspaces live under the same repo root, so `path = "../../../../starter-extensions/crates/starter-ext-sdk"` works. Verify the build produces a binary at `rubix/extensions/com.rubix.example/process/target/debug/rubix-example-extension` (or whatever the resolved target dir is given the split workspace) — adjust the `block.yaml` `runtime.bin` reference if the path drifted.
3. **CI builds both workspaces.** A separate cargo line in the existing workflow is sufficient — don't try to merge them.
4. **The example's `block.yaml` is the contract.** Any change to it (version bump, contributes update) requires a content-hash re-quarantine per SCOPE R6. For v1 leave it at 0.1.0 if the schema hasn't changed.

### Phase C — rubix-agent boot wiring

The integration core. Three commits, dependency-ordered.

1. **`boot/extensions.rs` is a verb file.** ≤ 400 lines hard. If composing the 7 primitives crosses 300, split per SCOPE Phase C's recommended `build.rs`/`router.rs`/`autostart.rs` shape.
2. **Migration runs at boot, not at startup script time.** `PgEnablementStore` carries its own migrations; `build_extension_admin` calls `pg_store.run_migrations(pool).await` before constructing the host. The agent.toml documents that PG schema mutates on first boot — same pattern as the goals-2-4-3 `flows_definitions` migration.
3. **Autostart respects the PG state, not the host's default.** A previously-disabled extension stays disabled across restarts; a never-touched extension uses `cfg.extensions.autostart_enabled_records` to decide.
4. **The admin router merges under a versioned prefix.** `/api/v1/extensions/*` matches the rest of the rubix surface. The existing AuthZ middleware sits in front of the gated lifecycle routes; the public manifest/ui-bundle routes pass through.
5. **Boot log explicitly counts extensions.** `INFO rubix.boot.extensions loaded=N autostarted=M failed=K` — operators read this; absence is a regression signal.

### Phase D — install/uninstall + frontend

The visible piece. Two commits.

1. **Tarball-only install in v1.** Registry-URL install returns `400 Bad Request` with `rubix.extension.registry_url_not_supported` Diagnostic. Document the deferral in the design doc.
2. **`DELETE /extensions/<id>` does NOT drop the PG row.** It marks `state = disabled` so future re-installs have audit history. The directory is removed; the row is not. Document this in the design doc.
3. **The test-ui-5 page imports from `@nube/starter-ext-ui`, not from a path.** The package is published in the workspace; the import should be `@nube/starter-ext-ui` per the existing convention. If the workspace isn't yet linked, add it to `packages/test-ui-5/package.json` `dependencies`.
4. **If `com.rubix.example` doesn't ship UI today, Phase D adds a minimal panel.** The block.yaml's `contributes.ui.exposes.main` keys to `./ui/main.tsx` per the existing UI runtime convention. The panel renders the example's `version` + a host-context theme value to prove the round-trip.

### Phase E — closing

1. **The integration test asserts 8 things from SCOPE Phase E.** If any one is harder than expected (e.g. event stream messages depend on async ordering), use `tokio::time::timeout` + `assert_eq` on the message set (not the order). Don't loosen the assertion below structural correctness.
2. **The design doc rewrite removes ALL "planned upstream" and "STARTER-CHANGES" references.** They're stale. Replace with present-tense pointers to the actual crate / file.
3. **The session note follows the goals-2-4-3 shape.** Per-phase summary, operator-runnable manual flow, test counts.

## Anti-patterns specific to this job

- **Don't author a new extension to demo this job.** The example is the worked example. Adding a second extension fragments review.
- **Don't put the PG store impl in `starter-ext-server`.** That crate stays sqlx-free for non-PG consumers (sqlite, fs-backed, etc.). The trait is there; impls live in their own crates.
- **Don't add `rubix/extensions/` to the parent workspace.** SCOPE R8 separation is load-bearing.
- **Don't add WASM in this job.** Phase 4 is a separate future job.
- **Don't introduce `Reversible` on install/uninstall.** Installs happen outside the agent loop typically; the changelog is the audit trail. Document the choice.
- **Don't list paths with brace expansion in handovers.** Trips diff-verify.
- **Don't list a path under Done that the stage didn't modify.** Same trap.
- **Don't `--no-verify`, don't `--force`.**

## REVIEW gate behaviour

Four gates: between A↔B, B↔C, C↔D, D↔E. Each gate commits and pushes the stage(s) that led to it; the gate itself commits nothing.

At each gate, the handover must include:

- One-line title per commit made in the phase, with file count.
- `cargo test` summary per crate.
- For A↔B: confirmation that `cargo test` across the full `starter-extensions` workspace passes, not just the new crate.
- For B↔C: confirmation that `target/debug/rubix-example-extension` exists and is executable.
- For C↔D: `tools/list` includes the example's echo tool; `GET /api/v1/extensions` returns the example record.
- For D↔E: one operator-runnable manual flow demonstrating the browser-side panel render.
- Any deviation from SCOPE.
- Open Questions evidence where the stage answered one.

## Closing trio — the last three todos of every stage

Every stage's todo checklist ends with the same three items, in order. Do **not** rename or reorder them.

1. `checks` — run the stage's verify list. Every step must pass.
2. `docs` — update `handover.md` for the next stage and the active session doc.
3. `git` — stage the changes, commit with the message `stage N: <one-line title from template.yaml>`, push to `codeless/rubix-extensions-wire`.

REVIEW gate stages mark `git` as `skipped — gate-only`. Never `--force`, never `--no-verify`.

## Hard rules (repeated)

- One verb per file. ≤ 400 lines hard, ~100 typical.
- Code comments link `docs/design/<area>/README.md` only.
- No phasing markers in code.
- Upstream-first (R2). `starter-ext-store-pg` lands before rubix consumes it.
- Tool outputs are `Diagnostic` + structured data, never pre-formatted strings (extension lifecycle endpoints return the existing starter-ext-server JSON shape, not Diagnostics — admin-router contract).
- Catalogue files are the source of truth for MessageKeys.
- Tests live with the code in the same commit.
- Extensions never depend on rubix-* (SCOPE R8).
- Comments explain *why*, not *what*. No emojis.

## References

- `DOCS/extensions/scope/SCOPE.md` — authoritative starter-extensions scope.
- `starter-extensions/crates/starter-ext-spi/` — the contract.
- `starter-extensions/crates/starter-ext-host/` — the loader.
- `starter-extensions/crates/starter-ext-supervisor/` — the lifecycle manager.
- `starter-extensions/crates/starter-ext-server/` — the admin router + bundle serving + EnablementStore trait.
- `starter-extensions/packages/starter-ext-ui/` — the frontend host-manager.
- `starter-extensions/packages/starter-ext-sdk-ts/` — the author-side TS SDK.
- `rubix/extensions/com.rubix.example/` — the worked example.
- `rubix/docs/design/extensions/README.md` — gets rewritten in Phase E.
- `rubix/SCOPE.md` R7, R8 — the rules.
- `rubix/docs/sessions/2026-05-24-goals-2-4-3-landed.md` — the verification-evidence shape Phase E mirrors.
