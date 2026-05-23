//! Helpers used by the CLI: create a user (returns the new user id),
//! issue an API token for that user (the demo prints the plaintext
//! once so curl can use it), and grant / revoke a single
//! `(subject, resource, action)` tuple by inserting an Allow / Deny
//! rule keyed on the user's subject id.
//!
//! Granular per-user authz is expressed by a `Rule` with
//! `role = "*"` and `condition = "subject == \"<user_id>\""`. The
//! `StaticRbacEngine` evaluates the condition against the resolved
//! `Principal`, so the rule only fires for that specific user.

use std::sync::Arc;

use anyhow::{Context, Result};
use starter_auth_users::admin::create_admin;
use starter_auth_users::role::Role;
use starter_auth_users::store::{TokenStore, UserStore};
use starter_auth_users::token::{issue, IssuedToken};
use starter_authz::config::Effect;
use starter_authz::db_engine::DbPolicyEngine;
use starter_authz::store::StoredRule;

/// Create a user with the supplied email + password + role.
pub async fn create_user(
    users: &Arc<dyn UserStore>,
    email: &str,
    password: &str,
    role: Role,
) -> Result<String> {
    let id = create_admin(users.as_ref(), email, password, role)
        .await
        .with_context(|| format!("create user {email}"))?;
    Ok(id)
}

/// Issue an API token for the user. Prints the plaintext exactly
/// once — the database stores only the hash.
pub async fn issue_token(tokens: &Arc<dyn TokenStore>, user_id: &str) -> Result<IssuedToken> {
    // No scopes attached — authz decisions are by policy engine + role,
    // not by token scopes, in this demo.
    let t = issue(tokens.as_ref(), user_id, &[], None)
        .await
        .context("issue token")?;
    Ok(t)
}

/// Insert an Allow rule scoped to a single user.
pub async fn grant(
    engine: &Arc<DbPolicyEngine>,
    admin_subject: &str,
    user_id: &str,
    resource: &str,
    action: &str,
) -> Result<String> {
    upsert_rule(engine, admin_subject, user_id, resource, action, Effect::Allow).await
}

/// Insert a Deny rule scoped to a single user. Wins over any
/// matching Allow (deny-overrides).
pub async fn revoke(
    engine: &Arc<DbPolicyEngine>,
    admin_subject: &str,
    user_id: &str,
    resource: &str,
    action: &str,
) -> Result<String> {
    upsert_rule(engine, admin_subject, user_id, resource, action, Effect::Deny).await
}

async fn upsert_rule(
    engine: &Arc<DbPolicyEngine>,
    admin_subject: &str,
    user_id: &str,
    resource: &str,
    action: &str,
    effect: Effect,
) -> Result<String> {
    let id = uuid::Uuid::new_v4().to_string();
    let row = StoredRule {
        id: id.clone(),
        role: "*".into(),
        resource: resource.into(),
        actions: vec![action.into()],
        condition: Some(format!("subject == \"{user_id}\"")),
        effect: match effect {
            Effect::Allow => "allow".into(),
            Effect::Deny => "deny".into(),
        },
        // Higher than the built-in default rules (priority 0) so the
        // intent is obvious — the engine's deny-overrides still wins
        // on conflict regardless of priority, but the field also
        // sorts ties.
        priority: 100,
        created_by: admin_subject.to_string(),
    };
    engine
        .store()
        .insert_rule(&row)
        .await
        .with_context(|| format!("insert {effect:?} rule"))?;
    engine.reload().await.context("reload engine cache")?;
    Ok(id)
}
