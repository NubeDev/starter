# Long-term layout preview — `dto/analytics.rs`

When the analytics goal collapses to a single flat file (instead of
the current `analytics/<verb>.rs` tree), this is the file header it
will carry:

```rust
//! analytics goal — REST DTOs.
//!
//! This file holds DTO structs only; tool dispatch logic lives in
//! `rubix-tools::analytics`.
```

This `.preview.md` file is intentionally not a `.rs` file: Rust
forbids `analytics.rs` and `analytics/` coexisting in the same
parent module. The preview stays as documentation so the long-term
shape is visible without breaking compilation.
