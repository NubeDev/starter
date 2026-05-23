## Done

- Created the rubix skeleton tree: `rubix/contracts/spi` (rubix-spi reserving KindManifest/Msg/slot_schema/dto/artifacts module slots, depends only on starter-spi); `rubix/agent/crates/{graph,engine,kinds-registry}` and `rubix/agent/crates/apps/agent` (bin prints `rubix-agent vX.Y.Z`); `rubix/agent-sdk`, `rubix/agent-client-rs`; TS packages `agent-client-ts`, `ui-kit`, `ui-core`, `extension-ui-sdk` (workspace:* re-exporting `@rubix/ui-core`), `studio` (with reserved `src-tauri/` dirs), `desktop`; Dart `agent-client-dart/pubspec.yaml`.
- Added all seven Rust crates to the starter root `Cargo.toml` workspace members under a `# rubix tree` comment block, plus `rubix-spi` in `[workspace.dependencies]`.
- Created `rubix/pnpm-workspace.yaml` listing the six TS packages.
- Wrote `rubix/mani.yaml` with `build`, `test`, `lint`, `status`, `commit` tasks. `lint` enforces R1 (≤400 lines per tracked rubix file) by iterating `git ls-files rubix/`.
- Verified: `cargo build` of all rubix crates green; rubix-agent binary prints version and exits 0; `pnpm install` at `rubix/` resolves all 6 packages; `mani run build/lint/test --all` green; demonstrated `mani run lint` fails on a synthetic 401-line file with the expected `R1 violation:` message before the fixture was removed.
- Committed as `7b66632` on `codeless/rubix-phase-0` with message starting `stage 1 — layout, workspace wiring, mani task surface`.

## Next

- Stage 2: testcontainer fixtures + CI smoke. Create `rubix/data-postgres` and `rubix/data-clickhouse` empty crates with smoke tests through the existing `starter-store-{postgres,clickhouse}::testing` seams, and write `rubix/docs/testing/SETUP.md` with docker prerequisites.
- Stage 3: write the eight load-bearing design docs under `rubix/docs/design/` (`OVERVIEW`, `EVERYTHING-AS-NODE`, `NODE-AUTHORING`, `KIND-MANIFEST`, `AUTH`, `MIGRATIONS`, `TESTS`, `VERSIONING`). `AUTH.md` and `MIGRATIONS.md` must be Phase-1-ready per Q3 of the job SCOPE.

## What you need to know

- The Cargo workspace lives at the starter repo root (single workspace per Q2). New rubix Rust crates must be added to the root `Cargo.toml` members array, and any shared internal alias goes in `[workspace.dependencies]` next to `rubix-spi`.
- `cargo build --workspace` (the whole starter tree) currently fails on a pre-existing toolchain mismatch: `aws-*` crates require rustc 1.91.1, the local toolchain is 1.90.0. This is unrelated to Phase 0. Stage-boundary verification was done with `-p` filters scoped to the rubix crates, and `mani run build --all` uses the same `-p` filter list.
- `mani run <task> --all` (not just `mani run <task>`) is required because the rubix mani config declares a single `rubix` project — without `--all` mani reports "no matching projects found".
- pnpm scripts use `pnpm -r --if-present run <script> || true` because no rubix package defines `build`/`test` scripts yet; once any does, the `--if-present` flag will pick it up cleanly. Revisit when Studio gets a real build.
- `studio/src-tauri/` contains only `.gitkeep` placeholders; Phase 1 will land the actual Tauri shell.
- `extension-ui-sdk` already declares `@rubix/ui-core` as a `workspace:*` dependency so the R7/R8 re-export pattern is exercised even though both packages are empty.

## Open questions

- (none)
