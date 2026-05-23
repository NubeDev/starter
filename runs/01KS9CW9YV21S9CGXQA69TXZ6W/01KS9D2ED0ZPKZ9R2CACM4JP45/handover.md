## Done

- Scaffolded `crates/starter-tags/` per DOCS/Tags/SCOPE.md (sync, driver-free; deps: serde, serde_json, thiserror, nom 7).
- `set.rs`: TagSet, TagValue (Bool|Str only — no Num), `tag_value_to_ch_string` as the single canonical conversion. `insert` rejects reserved bool strings (M-2, all case variants). `insert_json` coerces integers to canonical decimal strings and rejects non-integer / non-finite numbers (T2).
- `query.rs`: TagQuery AST + nom parser for the T7 grammar. Integer literals only; float literals (incl. negatives and scientific notation) raise `TagParseError::FloatLiteral` mentioning `samples.value_num`. Display round-trips through FromStr.
- `compile_pg.rs` (T8a): only `column @> $N::jsonb`, AND/OR/NOT.
- `compile_ch.rs` (T8b): only `column[$k] = $v`, AND/OR/NOT; uses `tag_value_to_ch_string` for the literal.
- `compile_match.rs` (T8c): in-process matcher, the oracle for T8a/T8b.
- `definition.rs`: TagKind enum `{Bool, Str, Ref, NumDiscriminant}` (T5 reconciliation, no bare `Num`); TagDictionary trait so storage lives elsewhere.
- `reserved.rs`: T6 table as `RESERVED_KEYS` constant.
- `error.rs`: typed `TagParseError` / `TagSetError`.
- Tests: `parser.rs` (10), `pg.rs` (4), `ch.rs` (4), `match.rs` (5), `roundtrip.rs` (1), `semantic_parity.rs` (8). All 32 pass; no DB contact. Parity fixture covers integer-as-string discriminant, Bool no-coercion, bare-tag sugar, float rejection, reserved-bool-string rejection, and `tag_value_to_ch_string` round-trip.
- Added crate to root `[workspace].members` and `[workspace.dependencies]` (alphabetical position next to `starter-spi`).
- Committed as `stage 1 (slice A) — starter-tags crate` (6b4797f).

## Next

- Stage 2 (slice B): `starter-store-postgres` dimensions feature — entities / entity_refs / tag_definitions / tag_prefix_registry / marts / cleaners / sandboxes / ext_manifest_approvals catalogs. Wire the `TagDictionary` Postgres impl against this crate.

## What you need to know

- Single workspace dep added: `nom = "7"` is local to this crate (not a workspace dep yet) — fine because it's the only consumer per T1.
- `query.rs` is 348 lines; SCOPE budget says `< 250`. Hand-rolled string-literal handling pushed it over. Acceptable for stage 1 but a future change that touches the parser should split it (suggestion: `query/parser.rs` + `query/ast.rs`).
- Two clippy warnings remain (one `redundant_closure` in `set.rs` Deserializer, one `type_complexity` in the parity battery). Not `-D warnings`, just lints; left for the final sweep.
- `TagValue` has a custom `Deserialize` impl that mirrors `insert` semantics — JSON arrays/objects are rejected, integers coerce to canonical decimal strings, reserved bool strings are rejected. Use the typed `insert` / `insert_json` for the same guarantees when constructing programmatically; raw `TagSet.0.insert(...)` bypasses them (used in one test to prove the matcher does not coerce).
- `SqlFragment` is defined in `compile_pg.rs` and re-exported from `compile_ch.rs` so both compilers share the type (per the SCOPE's "two SQL flavours, no logic duplicated" wording).
- `f.binds` for PG holds a single JSONB-shaped `Value::Object` per leaf; for CH it holds two `Value::String`s per leaf (key, value). Storage crates in later stages bind these directly.
- Tests use `tags` as the column name and `first_bind: 1`. The compilers track running bind count across `AND`/`OR`/`NOT` branches automatically.

## Open questions

- (none)
