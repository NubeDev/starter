# Long-term layout preview — `dto/user.rs`

When the user goal collapses to a single flat file (instead of
the current `user/<verb>.rs` tree), this is the file header it
will carry:

```rust
//! user goal — REST DTOs.
//!
//! This file holds DTO structs only; tool dispatch logic lives in
//! `rubix-tools::user`.
```

This `.preview.md` file is intentionally not a `.rs` file: Rust
forbids `user.rs` and `user/` coexisting in the same parent
module. The preview stays as documentation so the long-term shape
is visible without breaking compilation.
