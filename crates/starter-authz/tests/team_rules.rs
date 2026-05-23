//! Phase 7b — team-grant semantics. SCOPE-EXT.md R13.
//!
//! These tests cover the engine-level half of slice 7b: the
//! `principal.teams contains "X"` operator, the
//! "compile/load-time" loud-failure behaviour when the LHS resolves
//! to a non-array, and the cross-tenant rule scoping that pairs
//! with R11. The auth-store half (team CRUD + authenticator
//! population) is covered in `starter-auth-users/tests/teams_*.rs`.

use std::sync::Arc;

use starter_authz::condition::EvalError;
use starter_authz::{AuthzConfig, Expr, StaticRbacEngine, StaticRegistry};
use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{
    Decision, Ownership, PolicyEngine, ResourceRef, ResourceRegistry, ResourceSpec,
};

fn registry(kind: &'static str, actions: &'static [&'static str]) -> Arc<StaticRegistry> {
    let r = Arc::new(StaticRegistry::new());
    r.register(ResourceSpec::from_static_tenant_scoped(
        kind,
        actions,
        Ownership::None,
        kind,
        "",
    ));
    r
}

fn principal_with_teams(subject: &str, tenant_id: &str, teams: &[&str]) -> Principal {
    Principal {
        subject: subject.into(),
        role: Role::Reader,
        scopes: vec![],
        tenant_id: Some(tenant_id.into()),
        teams: teams.iter().map(|s| (*s).to_string()).collect(),
        extra: serde_json::Value::Null,
    }
}

fn team_rule_cfg(tenant_id: &str, team_slug: &str) -> AuthzConfig {
    // Exactly one rule, scoped to the tenant, gated on team
    // membership. Phase 7b R13 — "one row covers every team
    // member".
    let toml = format!(
        r#"
default_policy = false

[[rules]]
role      = "*"
resource  = "weather"
actions   = ["refresh"]
condition = 'principal.teams contains "{team_slug}"'
effect    = "allow"
tenant_id = "{tenant_id}"
"#
    );
    AuthzConfig::from_toml_str(&toml).unwrap()
}

#[tokio::test]
async fn team_grant_covers_every_team_member() {
    // One rule, two principals: alice in hvac-ops gets refresh,
    // bob outside the team gets denied. No per-user rule rows.
    let reg = registry("weather", &["refresh"]);
    let eng = StaticRbacEngine::from_config(team_rule_cfg("tenant-a", "hvac-ops"), reg).unwrap();
    let row = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");

    let alice = principal_with_teams("alice", "tenant-a", &["hvac-ops"]);
    let bob = principal_with_teams("bob", "tenant-a", &[]);

    match eng.check(&alice, "refresh", &row).await {
        Decision::Allow { .. } => {}
        d => panic!("alice in hvac-ops should be allowed, got {d:?}"),
    }
    match eng.check(&bob, "refresh", &row).await {
        Decision::Deny { reason, .. } => {
            assert_eq!(reason, "no_matching_rule");
        }
        d => panic!("bob not in team should be denied, got {d:?}"),
    }
}

#[tokio::test]
async fn team_membership_remove_takes_effect_immediately() {
    // Removing alice from the team is modeled here by minting a
    // new Principal with an empty `teams` list (which is exactly
    // what the authenticator does on the next request after the
    // membership row is deleted). Same rule, same DB, no engine
    // reload — purely principal-driven.
    let reg = registry("weather", &["refresh"]);
    let eng = StaticRbacEngine::from_config(team_rule_cfg("tenant-a", "hvac-ops"), reg).unwrap();
    let row = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");

    let alice_member = principal_with_teams("alice", "tenant-a", &["hvac-ops"]);
    let alice_removed = principal_with_teams("alice", "tenant-a", &[]);

    matches!(
        eng.check(&alice_member, "refresh", &row).await,
        Decision::Allow { .. }
    )
    .then_some(())
    .expect("alice as member should be allowed");
    match eng.check(&alice_removed, "refresh", &row).await {
        Decision::Deny { reason, .. } => assert_eq!(reason, "no_matching_rule"),
        d => panic!("alice after removal should be denied, got {d:?}"),
    }
}

#[tokio::test]
async fn team_rules_are_tenant_scoped() {
    // R13 + R11 — a team rule in tenant A must NOT match a
    // request from tenant B even when the principal carries the
    // same team slug. The cross-tenant predicate fires first;
    // even if it didn't, the rule's tenant_id filter would.
    let reg = registry("weather", &["refresh"]);
    let eng = StaticRbacEngine::from_config(team_rule_cfg("tenant-a", "hvac-ops"), reg).unwrap();

    // Bob is in tenant-b, has the same team slug there, asks for
    // a row in tenant-b. There is no rule for tenant-b → deny.
    let bob = principal_with_teams("bob", "tenant-b", &["hvac-ops"]);
    let row_b = ResourceRef::row("weather", "row-2").with_tenant("tenant-b");
    match eng.check(&bob, "refresh", &row_b).await {
        Decision::Deny { reason, .. } => assert_eq!(reason, "no_matching_rule"),
        d => panic!("cross-tenant team match leaked: {d:?}"),
    }

    // And the cross-tenant case (bob in tenant-b reaching for a
    // tenant-a row) is short-circuited by the predicate.
    let row_a = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");
    match eng.check(&bob, "refresh", &row_a).await {
        Decision::Deny { reason, .. } => assert_eq!(reason, "cross_tenant"),
        d => panic!("cross-tenant should deny by predicate: {d:?}"),
    }
}

#[tokio::test]
async fn contains_lhs_not_array_is_loud_failure_not_silent_false() {
    // SCOPE-EXT.md R13 — "engine compile-time error otherwise,
    // parallel to R8's loud-failure shape — NOT a silent false at
    // evaluation". When the LHS of `contains` resolves to a value
    // that is not an array, the engine surfaces a typed
    // `condition_invalid` deny (so an operator sees the
    // malformed-rule message on the very first hit) instead of
    // silently returning `false` (which would mask a misconfigured
    // rule indefinitely).

    // Direct expression test — proves the typed error surfaces
    // from the parser/evaluator boundary.
    let expr = Expr::parse(r#"principal.role contains "admin""#).unwrap();
    let ctx = starter_authz::condition::Context {
        vars: serde_json::json!({"principal": {"role": "reader"}}),
    };
    let err = expr.try_eval(&ctx).expect_err("expected typed error");
    match err {
        EvalError::ContainsLhsNotArray { path, actual_type } => {
            assert_eq!(path, "principal.role");
            assert_eq!(actual_type, "string");
        }
        #[allow(unreachable_patterns)]
        other => panic!("unexpected EvalError variant: {other:?}"),
    }

    // Engine integration — surface as condition_invalid deny.
    let toml = r#"
default_policy = false

[[rules]]
id        = "bad-rule"
role      = "*"
resource  = "weather"
actions   = ["refresh"]
# `principal.subject` is always a string, never an array.
condition = 'principal.subject contains "alice"'
effect    = "allow"
tenant_id = "tenant-a"
"#;
    let cfg = AuthzConfig::from_toml_str(toml).unwrap();
    let reg = registry("weather", &["refresh"]);
    let eng = StaticRbacEngine::from_config(cfg, reg).unwrap();

    let alice = principal_with_teams("alice", "tenant-a", &["hvac-ops"]);
    let row = ResourceRef::row("weather", "row-1").with_tenant("tenant-a");
    match eng.check(&alice, "refresh", &row).await {
        Decision::Deny {
            reason,
            matched_rule,
        } => {
            assert_eq!(reason, "condition_invalid");
            assert_eq!(matched_rule.as_deref(), Some("bad-rule"));
        }
        d => panic!("expected condition_invalid deny, got {d:?}"),
    }
}

#[tokio::test]
async fn contains_with_missing_lhs_is_silent_false_not_error() {
    // Symmetric to R8's "missing attribute != true" — a principal
    // that pre-dates Phase 7b (no `principal.teams` populated)
    // should just NOT match team rules; it should not deny with
    // condition_invalid. We model this by directly evaluating the
    // expression against a context that has no `principal.teams`
    // entry; missing-vs-non-array is the discriminator.
    let expr = Expr::parse(r#"principal.teams contains "hvac-ops""#).unwrap();
    let ctx = starter_authz::condition::Context {
        vars: serde_json::json!({"principal": {"role": "reader"}}),
    };
    assert!(!expr.try_eval(&ctx).unwrap());
}
