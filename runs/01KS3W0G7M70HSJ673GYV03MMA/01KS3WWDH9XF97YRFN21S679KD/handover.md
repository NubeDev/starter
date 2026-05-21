## Done

- Added `crates/starter-skills` as a workspace member with the Phase 1-locked dep tree (`starter-flow-spi`, `serde`, `serde_yaml`, `thiserror`, `async-trait`, `blake3`, `tracing` — no tokio runtime, no provider SDKs, no I/O crates beyond `std::fs`).
- Implemented `parser::parse_skill_md` with `serde(deny_unknown_fields)` over `Frontmatter { id, description, allowed_tools, model_hint, trust, resources }`. `Trust` defaults to `Approved` per R-skills-3 row 1. `id` validates via `SkillId`; `allowed_tools` entries validate via `KindId`; resources are scheme-checked against `SUPPORTED_RESOURCE_SCHEMES = ["file"]` per S-D2.
- Implemented `bundle::load_bundle` walker: reads `SKILL.md`, parses it, then reads each `file://` resource once into `Arc<[u8]>`. Rejects absolute paths and `..` traversal with `ResourcePathEscapesBundle`.
- Structured `SkillParseError` enum names the offending path on every variant (MissingSkillMd, MalformedFrontmatter, InvalidFrontmatter, InvalidSkillId, InvalidAllowedTool, UnsupportedResourceScheme, ResourcePathEscapesBundle, Io).
- 16 unit tests pass (`cargo test -p starter-skills`); full `cargo build --workspace` is green.
- Committed as `Phase 1 — crate skeleton + SKILL.md parser` (89bd622).

## Next

- Phase 2: implement `approval::hash_bundle(path)` per R-skills-2 (length-prefixed framing, CRLF/CR→LF on text files, `/` path separators, sort, blake3 hex) plus the `EXCLUDED` pub const slice and property tests (collision, line-ending stability, pinned digest).

## What you need to know

- `SkillId` lives at `starter_flow_spi::skill::SkillId` (not the crate root); the parser imports it via `use starter_flow_spi::skill::SkillId;`.
- `Bundle::resources` preserves authoring order — Phase 2 will sort lexicographically for the hash, so don't rely on this order anywhere downstream.
- `Resource::bytes` is already `Arc<[u8]>` so Phase 2's hasher can borrow without re-reading, and Phase 3's registry can hand out cheap clones.
- The walker also strips a leading UTF-8 BOM from the SKILL.md text before fence detection (editor-friendly), but per S-D5 the hash algorithm in Phase 2 must NOT do BOM handling — keep that separate.
- `serde_yaml` is pinned to `"0.9"` directly (not via workspace deps) because the workspace `Cargo.toml` doesn't expose it; it matches the version already used by `starter-flow-watch`.
- Workspace registration added `starter-skills` to both `members` and `[workspace.dependencies]`; future crates can pull it via `workspace = true`.

## Open questions

- (none)
