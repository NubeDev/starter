# Long-term layout preview — `dto/clickhouse.rs`

When the clickhouse goal collapses to a single flat file (instead
of the current `clickhouse/<verb>.rs` tree), this is the file
header it will carry:

```rust
//! clickhouse goal — REST DTOs.
//!
//! This file holds DTO structs only; tool dispatch logic lives in
//! `rubix-tools::clickhouse`.
```

This `.preview.md` file is intentionally not a `.rs` file: Rust
forbids `clickhouse.rs` and `clickhouse/` coexisting in the same
parent module. The preview stays as documentation so the long-term
shape is visible without breaking compilation.
