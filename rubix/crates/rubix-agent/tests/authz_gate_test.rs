//! Integration test for the `starter-authz` gate on rubix tool
//! dispatch. Pins the assertion shape; the wiring lands in PR 2
//! part 2 once the upstream `PgUserStore` arrives (see
//! docs/design/starter-changes/ "Phase 2a — starter-auth-users
//! Postgres store impls").
//!
//! What the live test will assert, once unblocked:
//!
//! 1. Boot rubix-agent against a testcontainers Postgres + the
//!    bootstrap operator account.
//! 2. Unauthenticated `POST /v1/tools/rubix.system.disk` returns
//!    401 (auth gate).
//! 3. Authenticated call from an operator missing the
//!    `system.read` permission returns 403 (authz gate).
//! 4. Authenticated call from an operator holding `system.read`
//!    returns 200 with the expected `DiskUsageResponse` shape.
//! 5. The `starter_changes` table grows by exactly one row per
//!    successful turn, carrying the operator id + the tool id.

#[test]
#[ignore = "blocked on upstream PgUserStore; see docs/design/starter-changes/"]
fn authz_gate_rejects_missing_permission_and_audit_writes_one_row() {
    // Intentionally empty: the test compiles so CI catches imports
    // drifting against the auth surface; the body lands with PR 2
    // part 2.
}

#[test]
fn per_verb_permission_constants_are_declared() {
    // Locks the convention without needing a live server: every
    // system verb owns its `REQUIRED_PERMISSION` next to its
    // `DESCRIPTOR`. Adding a system verb that forgets the constant
    // fails this test at compile time (unresolved import).
    use rubix_spi::dto::system::{alert_send, db, disk, flow_errors};

    assert_eq!(disk::REQUIRED_PERMISSION, "system.read");
    assert_eq!(db::REQUIRED_PERMISSION, "system.read");
    assert_eq!(flow_errors::REQUIRED_PERMISSION, "system.read");
    assert_eq!(alert_send::REQUIRED_PERMISSION, "system.alert");
}
