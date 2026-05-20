//! The default policy engine: RBAC + ownership + attribute
//! conditions. Loaded from [`crate::AuthzConfig`] at boot.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{Decision, PolicyEngine, ResourceRef, ResourceRegistry};

use crate::condition::{Context, Expr};
use crate::config::{Assignment, AuthzConfig, Effect, Rule};
use crate::defaults;
use crate::error::Result;

/// Built-in RBAC engine. Holds rules, assignments, and a handle to
/// the resource registry. Cheap to clone via `Arc`.
pub struct StaticRbacEngine {
    rules: Vec<CompiledRule>,
    assignments: Vec<Assignment>,
    registry: Arc<dyn ResourceRegistry>,
}

#[derive(Debug, Clone)]
struct CompiledRule {
    id: String,
    role: String,
    resource: String,
    actions: Vec<String>,
    effect: Effect,
    /// `Some(Owner)` when the source rule used the magic
    /// `condition = "owner"` shortcut; `Some(Expr(_))` for a
    /// parsed mini-language expression; `None` for an
    /// unconditional rule.
    cond: Option<CompiledCondition>,
}

#[derive(Debug, Clone)]
enum CompiledCondition {
    /// `principal.subject == object.owner`.
    Owner,
    /// A parsed expression from the mini-language.
    Expr(Expr),
}

impl StaticRbacEngine {
    /// Build an engine from config + registry. Compiles all rule
    /// conditions up front so per-`check` evaluation is allocation-
    /// free for the common case.
    pub fn from_config(cfg: AuthzConfig, registry: Arc<dyn ResourceRegistry>) -> Result<Self> {
        let mut rules: Vec<Rule> = Vec::new();
        if cfg.default_policy {
            rules.extend(defaults::built_in_rules());
        }
        rules.extend(cfg.rules);

        let mut compiled = Vec::with_capacity(rules.len());
        for (idx, r) in rules.into_iter().enumerate() {
            let id = r.id.clone().unwrap_or_else(|| format!("rule-{idx}"));
            let cond = match r.condition.as_deref() {
                None | Some("") => None,
                Some("owner") => Some(CompiledCondition::Owner),
                Some(expr) => Some(CompiledCondition::Expr(Expr::parse(expr)?)),
            };
            compiled.push(CompiledRule {
                id,
                role: r.role,
                resource: r.resource,
                actions: r.actions,
                effect: r.effect,
                cond,
            });
        }

        Ok(Self {
            rules: compiled,
            assignments: cfg.assignments,
            registry,
        })
    }

    /// Resolve every role the principal carries: the built-in
    /// `Principal.role` plus any roles bound to the subject in
    /// `assignments` (exact match or single-trailing-`*` glob).
    fn roles_for(&self, p: &Principal) -> Vec<String> {
        let mut out = vec![role_name(p.role).to_string()];
        for a in &self.assignments {
            if subject_matches(&a.subject, &p.subject) {
                for r in &a.roles {
                    if !out.contains(r) {
                        out.push(r.clone());
                    }
                }
            }
        }
        out
    }
}

fn role_name(r: Role) -> &'static str {
    match r {
        Role::Reader => "reader",
        Role::Writer => "writer",
        Role::Admin => "admin",
    }
}

fn subject_matches(pattern: &str, subject: &str) -> bool {
    if let Some(suffix) = pattern.strip_prefix('*') {
        subject.ends_with(suffix)
    } else {
        pattern == subject
    }
}

fn role_matches(rule_role: &str, principal_roles: &[String]) -> bool {
    rule_role == "*" || principal_roles.iter().any(|r| r == rule_role)
}

fn resource_matches(rule_kind: &str, kind: &str) -> bool {
    rule_kind == "*" || rule_kind == kind
}

fn action_matches(rule_actions: &[String], action: &str) -> bool {
    rule_actions.iter().any(|a| a == "*" || a == action)
}

fn build_context(p: &Principal, object: &ResourceRef) -> Context {
    let mut root = match p.extra.clone() {
        Value::Object(map) => Value::Object(map),
        _ => Value::Object(serde_json::Map::new()),
    };
    if let Value::Object(map) = &mut root {
        map.insert("subject".into(), Value::String(p.subject.clone()));
        map.insert("role".into(), Value::String(role_name(p.role).into()));
        map.insert(
            "object".into(),
            json!({
                "kind": object.kind,
                "id": object.id,
                "owner": object.owner,
            }),
        );
    }
    Context { vars: root }
}

#[async_trait]
impl PolicyEngine for StaticRbacEngine {
    async fn check(&self, principal: &Principal, action: &str, object: &ResourceRef) -> Decision {
        // SCOPE.md R3 — default-deny on unknown resources.
        if self.registry.lookup(&object.kind).is_none() {
            tracing::info!(
                subject = %principal.subject,
                action = %action,
                kind = %object.kind,
                reason = "unknown_resource",
                "authz deny"
            );
            return Decision::deny("unknown_resource");
        }

        let roles = self.roles_for(principal);
        let ctx = build_context(principal, object);

        let mut allow_match: Option<&CompiledRule> = None;
        let mut deny_match: Option<&CompiledRule> = None;

        for rule in &self.rules {
            if !role_matches(&rule.role, &roles) {
                continue;
            }
            if !resource_matches(&rule.resource, &object.kind) {
                continue;
            }
            if !action_matches(&rule.actions, action) {
                continue;
            }
            let cond_ok = match &rule.cond {
                None => true,
                Some(CompiledCondition::Owner) => match &object.owner {
                    Some(owner) => owner == &principal.subject,
                    None => false,
                },
                Some(CompiledCondition::Expr(e)) => e.eval(&ctx),
            };
            if !cond_ok {
                continue;
            }
            match rule.effect {
                Effect::Deny => {
                    deny_match = Some(rule);
                    // SCOPE.md R3: deny wins; we can stop scanning.
                    break;
                }
                Effect::Allow => {
                    if allow_match.is_none() {
                        allow_match = Some(rule);
                    }
                }
            }
        }

        let decision = match (deny_match, allow_match) {
            (Some(r), _) => {
                // Distinguish "this rule was an explicit deny" from
                // ownership-failure denies further down the stack.
                let reason = match &r.cond {
                    Some(CompiledCondition::Owner) => "not_owner",
                    _ => "explicit_deny",
                };
                Decision::deny_by(reason, r.id.clone())
            }
            (None, Some(r)) => Decision::allow_by(r.id.clone()),
            (None, None) => Decision::deny("no_matching_rule"),
        };

        match &decision {
            Decision::Allow { matched_rule } => {
                tracing::debug!(
                    subject = %principal.subject,
                    action = %action,
                    kind = %object.kind,
                    id = ?object.id,
                    matched_rule = ?matched_rule,
                    "authz allow"
                );
            }
            Decision::Deny {
                reason,
                matched_rule,
            } => {
                tracing::info!(
                    subject = %principal.subject,
                    action = %action,
                    kind = %object.kind,
                    id = ?object.id,
                    reason = %reason,
                    matched_rule = ?matched_rule,
                    "authz deny"
                );
            }
        }

        decision
    }
}
