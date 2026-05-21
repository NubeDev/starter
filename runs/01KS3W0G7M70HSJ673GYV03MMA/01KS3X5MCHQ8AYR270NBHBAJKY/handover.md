## Done

- Added `crates/starter-skills/src/approval.rs` implementing `hash_bundle(path) -> io::Result<String>` per R-skills-2 / agent R4: recursive walk, `EXCLUDED` (`pub const`) skip list, `/` separator normalisation, lexicographic sort on relative-path bytes, CRLF→LF then lone-CR→LF on text-extension files (`md/txt/json/yaml/yml/toml`), single `blake3` hasher with `u64_le` length-prefixed framing for each entry, lower-case hex digest.
- Wired `pub mod approval;` into `crates/starter-skills/src/lib.rs` and updated the module-level doc to mention Phase 2.
- 23 unit tests pass, including the three algorithm-pinning tests: `path_framing_prevents_collision`, `line_ending_normalisation_is_stable`, `fixture_digest_is_pinned` (plus `binary_files_are_not_normalised`, `excluded_paths_do_not_affect_hash`, `normalise_handles_mixed_endings`, `is_excluded_matches_suffix_globs`).
- Committed as `6fb8aca` with message starting `Phase 2 — content-hash algorithm + EXCLUDED list`.

## Next

- (none) — next session picks up Phase 3 per the job plan (likely `ApprovalStore` trait + `InMemoryApprovalStore`, then SQLite/Postgres impls).

## What you need to know

- `EXCLUDED` is matched on each path component during the walk: directory entries with excluded names are never descended into, file entries with excluded names are skipped. `*.swp` / `*.swo` / `*~` are handled by a hand-rolled suffix matcher (no `glob` dep, keeps the SCOPE dep tree boring).
- `is_text` uses a **closed** extension set (no content sniffing, no libmagic). Anything not in that set hashes verbatim. Changing the set will shift every approved hash.
- The pinned fixture digest is not a bare hex literal — `expected_pinned_digest()` recomputes it inline using the documented framing rules, so the test fails loudly if either the spec OR the implementation drifts.
- Non-UTF-8 file names and non-`Normal` path components return `io::Error(InvalidData)` rather than silently re-encoding. Bundles must be addressable by `/`-separated UTF-8 paths.
- Symlinks and other non-file/non-dir entries are ignored; bundles that rely on them are misusing the loader.

## Open questions

- (none)
