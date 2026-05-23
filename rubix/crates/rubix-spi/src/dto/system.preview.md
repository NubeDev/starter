# Long-term layout preview — `dto/system.rs`

When the system goal collapses to a single flat file (instead
of the current `system/<verb>.rs` tree), this is the file
header it will carry:

```rust
//! system goal — REST DTOs.
//!
//! This file holds DTO structs only; tool dispatch logic lives in
//! `rubix-tools::system`.
```

This `.preview.md` file is intentionally not a `.rs` file: Rust
forbids `system.rs` and `system/` coexisting in the same parent
module. The preview stays as documentation so the long-term shape
is visible without breaking compilation.
