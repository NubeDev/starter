## Done

- Wrote `rubix/extensions/README.md` (present-tense): layout, add-new-extension steps, build commands, link to `rubix/docs/design/extensions/README.md` (Phase E target).
- Added `rubix-extensions` job to `.github/workflows/ci.yml` running `cargo build --manifest-path rubix/extensions/Cargo.toml --workspace` with its own cache key, separate from the main `rust` job.
- Verified `cargo build` on the sibling workspace is green.
- Committed as stage 5 (single commit covers both docs + ci concerns).

## Next

- Stage 6: next phase B item per WORKFLOW.md (likely B.3 — first wire-in of starter-ext-host into rubix-agent boot).

## What you need to know

- The sibling workspace is intentionally excluded from rubix root; the new CI job is the only thing exercising it on PRs.
- Commit message lists both `docs(rubix-extensions)` and `ci(rubix-extensions-build)` per the stage instruction (combined into one commit since stage is a single unit).
- `rubix/docs/design/extensions/README.md` already exists as block-author guide; the link is forward-compatible with the Phase E rewrite.

## Open questions

- (none)
