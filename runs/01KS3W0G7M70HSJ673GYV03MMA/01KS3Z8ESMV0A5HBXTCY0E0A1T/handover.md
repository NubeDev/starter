## Done

- Reviewed stages 2–7 for R1/R2/R4/R5 + wire-format invariants
- Verified `cargo tree -p starter-skills --edges normal` (no provider SDK, no upward edges)
- Ran `cargo test -p starter-skills` (41 tests green) and `cargo test -p starter-flow-nodes --features ai-agent --test stage7_ai_agent_mount` (1 test green)
- Confirmed the SPI delta is a single non-breaking constructor on `ResourceRef`
- PASS: Layer-1 invariants hold — crate dep direction acyclic with skills strictly above spi and below nodes/ext-flow, single transport (AiRunner) preserved, content-hash byte-identical between freeze (`hash_bundle`) and on-mount verify (`read_and_verify`), trust boundary enforced by load-path with append-mostly store and re-quarantine on drift, no wire-format change

## Next

- Begin Stage 9: implement `starter-store-sqlite` and `starter-store-postgres` `ApprovalStore` impls behind the `skill-approvals` feature, with one new `skill_approvals` table per backend. Mirror `InMemoryApprovalStore`'s append-mostly semantics (only `record` and `revoke` mutate; `lookup` and `list` never write). `record` must be idempotent on `(skill_id, hash)`. Add a migration file per crate, gate the dep tree so the feature is default-on but opt-out-able, and ship smoke tests that round-trip an approval row, prove revoke removes only the targeted row, and prove drift (re-record at a new hash) leaves the prior row inert in `list()`. Do not touch `starter-skills` core types — Stage 8 froze them

## What you need to know

- Core surface is frozen at stage 8: `SkillRegistry`, `ApprovalStore`, `LlmSkillSelector` / `KeywordSkillSelector` / `FirstSkillSelector`, and the ai-agent on-mount check. Stages 9–12 build on top — they must not feed back into core shape changes
- The only SPI delta in this branch is `ResourceRef::new(uri, content_hash)`; the struct is `#[non_exhaustive]` so external constructors must keep using `new`
- `starter-flow-nodes`' `ai-agent` feature now pulls in `starter-skills` (downward edge); `dev-dependencies` also pull it for the mount smoke. Keep that direction
- The line-ending normalisation + text-path classification used by `hash_bundle` is exposed through `crate::approval::{is_text_path_pub, normalise_line_endings_pub}` so per-resource hashes and on-mount verification stay byte-identical with the bundle hash. Do not fork that algorithm in the store impls — stores only persist `(skill_id, hash, principal, ts)` rows
- `tokio` is in the normal dep tree of `starter-skills` (features `sync,time,rt`) to power the `LlmSkillSelector` 2 s hard timeout; that is intentional and matches SCOPE §"R-skills-5". Do not regress it

## Open questions

- (none)
