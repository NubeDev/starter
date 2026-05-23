# Long-term layout preview — `dto/flow_ops.rs`

When the flow_ops goal collapses to a single flat file (instead
of the current `flow_ops/<verb>.rs` tree), this is the file
header it will carry:

```rust
//! flow_ops goal — REST DTOs.
//!
//! This file holds DTO structs only; tool dispatch logic lives in
//! `rubix-tools::flow_ops`.
```

This `.preview.md` file is intentionally not a `.rs` file: Rust
forbids `flow_ops.rs` and `flow_ops/` coexisting in the same
parent module. The preview stays as documentation so the long-term
shape is visible without breaking compilation.
