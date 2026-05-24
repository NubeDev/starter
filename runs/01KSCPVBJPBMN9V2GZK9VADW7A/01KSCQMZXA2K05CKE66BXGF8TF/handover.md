## Done

- Created rubix/extensions/Cargo.toml as a sibling workspace with com.rubix.example/process as sole member.
- Excluded rubix/extensions from the root workspace in repo-root Cargo.toml.
- Wired rubix/extensions/com.rubix.example/process/Cargo.toml to depend on starter-ext-sdk via path `../../../../starter-extensions/crates/starter-ext-sdk` with `default-features = false, features = ["process"]`.
- `cargo build --manifest-path rubix/extensions/Cargo.toml -p rubix-example-extension` green; binary at `rubix/extensions/target/debug/rubix-example-extension`; running with `--help` prints placeholder banner and exits 0.
- Committed as 629d6f4.

## Next

- (none) — next session picks up the following stage.

## What you need to know

- src/main.rs is still the Phase-0 placeholder (eprintln + exit 0). The starter-ext-sdk dep is declared but unused; cargo doesn't fail on that. Future phases will replace main.rs with a real `register_process_main!` entry-point.
- A new rubix/extensions/Cargo.lock was generated and committed alongside the workspace bootstrap.
- Root workspace metadata still resolves (`cargo metadata` green).

## Open questions

- (none)
