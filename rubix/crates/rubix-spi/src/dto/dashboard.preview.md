# Long-term layout preview — `dto/dashboard.rs`

When the dashboard goal collapses to a single flat file (instead
of the current `dashboard/<verb>.rs` tree), this is the file
header it will carry:

```rust
//! dashboard goal — REST DTOs.
//!
//! This file holds DTO structs only; tool dispatch logic lives in
//! `rubix-tools::dashboard`.
```

This `.preview.md` file is intentionally not a `.rs` file: Rust
forbids `dashboard.rs` and `dashboard/` coexisting in the same
parent module. The preview stays as documentation so the long-term
shape is visible without breaking compilation.
