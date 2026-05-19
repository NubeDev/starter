//! `starter-smoke-tests` — Stage 9 design checks.
//!
//! This crate has no public API. The five smoke tests defined in
//! `DOCS/tools/scope/SCOPE.md` (section "Smoke test for the design")
//! live one-per-file under `tests/`:
//!
//! 1. `smoke_1_no_dep_leakage.rs` — `cargo tree -p starter-spi --edges
//!    normal` must match `DOCS/tools/scope/starter-spi-deps.baseline.txt`;
//!    no provider crate's deps may appear in the snapshot.
//! 2. `smoke_2_no_special_case_wiring.rs` — every provider crate's
//!    `Tool` / `Service` registers via the same `.register(...)` call
//!    the notes demo uses.
//! 3. `smoke_3_config_guarded_construction.rs` — an `if`-around-
//!    `.register(...)` flips the integration off via a single env var
//!    with no recompile of the consumer.
//! 4. `smoke_4_secrets_backend_swappable.rs` — switching from
//!    `starter-secrets-file` to `starter-secrets-keyring` requires
//!    zero changes to provider `Config` structs.
//! 5. `smoke_5_shutdown_actually_shuts_down.rs` — calling
//!    `ServiceRegistry::shutdown` causes every Service's `JoinHandle`
//!    to resolve within `SHUTDOWN_DEADLINE_DEFAULT` (5 s).
//!
//! The dep-baseline diff (smoke test 1) is **also** a CI gate, not
//! merely an after-the-fact check: see the `spi-dep-baseline` job in
//! `.github/workflows/ci.yml` which calls
//! `scripts/check-spi-dep-baseline.sh` directly. The cargo-level test
//! is a developer convenience; CI is the authority.
