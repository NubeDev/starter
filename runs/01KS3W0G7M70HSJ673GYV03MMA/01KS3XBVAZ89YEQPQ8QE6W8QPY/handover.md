## Done

- Added `starter_skills::registry` module with `SkillRegistry`, `SkillRegistryBuilder`, `Skill`, `ContributedSkill`, `LoadError`. Builder methods: `with_approval_store`, `with_approval_store_arc`, `with_default_selector` (accepted but stored only — Phase 4 dispatch), `load_dir`, `load_dir_quarantined`, `extend`, `build`. Registry exposes `list`, `list_quarantined`, `get`, `approve`, `revoke`, `reload`, `approval_store`, `builder`.
- Added `starter_skills::store` module with async `ApprovalStore` trait, `ApprovalRow`, `ApprovalStoreError`, and `InMemoryApprovalStore`. Append-mostly per R-skills-7 (only `record` and `revoke` mutate).
- R-skills-3 trust matrix implemented: `load_dir` honours frontmatter; `load_dir_quarantined` and `extend` force quarantined regardless of frontmatter; `(skill_id, hash)` approval row promotes; hash mismatch re-quarantines.
- Per-resource `ResourceRef.content_hash` computed with the same text/binary classification + line-ending normalisation as `hash_bundle`, ready for the Phase 4b on-mount check.
- Added `ResourceRef::new(uri, content_hash)` constructor in `starter-flow-spi` (the struct is `#[non_exhaustive]`, blocking literal construction from outside).
- Both stage-mandated smoke tests pass: (a) extension-contributed skill stays quarantined despite `trust: approved` frontmatter and flips on `approve()`; (b) hash mismatch on `reload()` re-quarantines while the H1 row stays inert in `ApprovalStore::list()`. Plus three extra registry unit tests and five `InMemoryApprovalStore` tests. `cargo test -p starter-skills` → 33 passed.
- `cargo build --workspace` green.
- Committed: `0623364 stage 5: Phase 3 — SkillRegistry + ApprovalStore + InMemoryApprovalStore`.

## Next

- Stage 6 / Phase 4: three selectors (`LlmSkillSelector` default with 2s timeout / no retries / fail-to-None, `KeywordSkillSelector`, `FirstSkillSelector`); make `SkillRegistry: SkillSelector` (R-skills-4 — never return quarantined); two smoke tests (selection frozen per run; quarantined never reaches strategy). The builder's `with_default_selector(...)` hook already exists — wire dispatch through it.

## What you need to know

- Dev-deps added to `starter-skills/Cargo.toml`: `tokio`, `starter-spi`, `serde_json`. Dev-only so the SCOPE dep-tree CI gate (no provider SDKs in `cargo tree --edges normal`) is unaffected.
- `crate::approval::is_text_path` and `normalise_line_endings_pub` are `pub(crate)` accessors over the private text/normalise helpers, used by the registry to keep per-resource hashes byte-equal with `hash_bundle`'s normalisation rules.
- `load_dir` is one level deep — only direct subdirectories of the load root with a `SKILL.md` are loaded. Nested bundles (e.g. inside `node_modules/`) are ignored intentionally.
- `repartition_one` (used by `approve`/`revoke`) only moves a skill between maps when the in-memory `bundle_hash` matches the row's hash, so operator pre-approval of a future hash records the row but does not promote the current bundle.
- `LoadError::DuplicateSkillId` fails the build if two bundles share an id, deterministically — bundle dirs are sorted before iteration.
- `with_default_selector(...)` currently accepts and discards its argument so Phase 4 can wire dispatch without an API churn.

## Open questions

- (none)
