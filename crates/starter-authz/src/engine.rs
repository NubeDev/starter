//! The default policy engine: RBAC + ownership + attribute
//! conditions. Loaded from [`crate::AuthzConfig`] at boot.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};

use starter_spi::auth::{Principal, Role};
use starter_spi::authz::{Decision, PolicyEngine, ResourceRef, ResourceRegistry};

use crate::audit::{DecisionEntry, DecisionSink, NoopDecisionSink};
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
    sink: Arc<dyn DecisionSink>,
}

#[derive(Debug, Clone)]
struct CompiledRule {
    id: String,
    role: String,
    resource: String,
    actions: Vec<String>,
    effect: Effect,
    /// Tenant scope of this rule (Phase 7a). `None` is a global
    /// rule (matches any tenant). `Some(_)` only matches when the
    /// principal is bound to that tenant.
    tenant_id: Option<String>,
    /// Instance scope. `None` or `Some("*")` is kind-wide; a
    /// concrete id only matches a request whose `object.id` equals
    /// it. This is what makes a per-resource grant authorize only
    /// its own immutable id rather than the whole kind.
    resource_id: Option<String>,
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
                tenant_id: r.tenant_id,
                resource_id: r.resource_id,
                cond,
            });
        }

        Ok(Self {
            rules: compiled,
            assignments: cfg.assignments,
            registry,
            sink: Arc::new(NoopDecisionSink),
        })
    }

    /// Replace the audit sink. Default is [`NoopDecisionSink`]
    /// (silent drop, zero overhead). Wire a [`crate::DbDecisionSink`]
    /// here to persist decisions. SCOPE-EXT.md R14.
    pub fn with_sink(mut self, sink: Arc<dyn DecisionSink>) -> Self {
        self.sink = sink;
        self
    }

    /// Borrow the configured sink — used by `DbPolicyEngine` to
    /// propagate the sink across cache reloads.
    pub fn sink(&self) -> &Arc<dyn DecisionSink> {
        &self.sink
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
        // G3 — synthesise a `team:<slug>` role for each team the
        // principal carries. The engine already matches by role
        // string; the grants API persists rules whose `role` is
        // `team:<slug>`. Synthesising here keeps the SPI stable
        // (no new field on `Principal`) and works for both the
        // static and DB-backed engines (db_engine delegates to
        // `check`, which calls this).
        for slug in &p.teams {
            let role = format!("team:{slug}");
            if !out.contains(&role) {
                out.push(role);
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

/// A rule with no `resource_id` (or `"*"`) applies to every instance of its
/// kind — the original kind-wide behaviour. A rule pinned to a concrete id only
/// matches a request that targets that exact instance; a collection-level check
/// (no `object.id`) never matches an instance-pinned rule.
fn instance_matches(rule_id: &Option<String>, object_id: &Option<String>) -> bool {
    match rule_id.as_deref() {
        None | Some("*") => true,
        Some(id) => object_id.as_deref() == Some(id),
    }
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
        // Phase 7a — expose `tenant` so conditions can reference it
        // (e.g. `tenant == "acme"`). The cross-tenant predicate
        // already fires before condition evaluation; this is here
        // for diagnostic / dashboard rules.
        if let Some(t) = &p.tenant_id {
            map.insert("tenant".into(), Value::String(t.clone()));
        }
        // Phase 7b (R13) — expose principal-level fields under a
        // dedicated `principal.*` namespace so rules can say
        // `principal.teams contains "hvac-ops"`. The `teams`
        // array is always present (empty for pre-Phase-7b
        // principals) — see SCOPE-EXT.md "strictly additive".
        map.insert(
            "principal".into(),
            json!({
                "subject": p.subject,
                "role":    role_name(p.role),
                "teams":   p.teams,
                "tenant":  p.tenant_id,
            }),
        );
        map.insert(
            "object".into(),
            json!({
                "kind": object.kind,
                "id": object.id,
                "owner": object.owner,
                "tenant": object.tenant,
            }),
        );
    }
    Context { vars: root }
}

#[async_trait]
impl PolicyEngine for StaticRbacEngine {
    async fn check(&self, principal: &Principal, action: &str, object: &ResourceRef) -> Decision {
        // SCOPE.md R3 — default-deny on unknown resources.
        let spec = match self.registry.lookup(&object.kind) {
            Some(s) => s,
            None => {
                tracing::info!(
                    subject = %principal.subject,
                    action = %action,
                    kind = %object.kind,
                    reason = "unknown_resource",
                    "authz deny"
                );
                let decision = Decision::deny("unknown_resource");
                self.audit(principal, action, object, &decision).await;
                return decision;
            }
        };

        // SCOPE-EXT.md R11 + ADR-tenant-hierarchy — cross-tenant
        // predicate runs BEFORE role / condition evaluation. A
        // tenant-scoped kind requires the principal to ADMINISTER the
        // object's tenant: either it is the principal's own tenant
        // (the flat R11 case) or a descendant in the principal's
        // resolved subtree (`principal.tenant_scope`), which lets a
        // parent — e.g. a reseller — act on a child's resource.
        // Missing binding or a tenant outside the subtree short-
        // circuits with a typed deny reason. The super-admin sentinel
        // `"*"` bypasses the check entirely (whole-forest admin).
        //
        // A principal with an empty `tenant_scope` (pre-hierarchy
        // wiring) falls back to strict `tenant_id == object.tenant`
        // equality via `administers_tenant`, preserving R11 byte-for-
        // byte.
        if spec.tenant_scoped && !principal.is_super_admin() {
            match (&principal.tenant_id, &object.tenant) {
                (None, _) => {
                    tracing::info!(
                        subject = %principal.subject,
                        action = %action,
                        kind = %object.kind,
                        reason = "no_tenant_binding",
                        "authz deny"
                    );
                    let decision = Decision::deny("no_tenant_binding");
                    self.audit(principal, action, object, &decision).await;
                    return decision;
                }
                (Some(_), Some(ot)) if principal.administers_tenant(ot) => { /* fall through */ }
                _ => {
                    tracing::info!(
                        subject = %principal.subject,
                        action = %action,
                        kind = %object.kind,
                        principal_tenant = ?principal.tenant_id,
                        object_tenant = ?object.tenant,
                        reason = "cross_tenant",
                        "authz deny"
                    );
                    let decision = Decision::deny("cross_tenant");
                    self.audit(principal, action, object, &decision).await;
                    return decision;
                }
            }
        }

        let roles = self.roles_for(principal);
        let ctx = build_context(principal, object);

        let mut allow_match: Option<&CompiledRule> = None;
        let mut deny_match: Option<&CompiledRule> = None;

        for rule in &self.rules {
            // Phase 7a + ADR-tenant-hierarchy — tenant-scoped rules
            // match when the principal administers the rule's tenant:
            // its own tenant (flat R11 case) or any tenant in its
            // subtree. A rule written for a parent tenant (e.g.
            // Daikin) thus applies when that parent's admin acts on a
            // descendant's resource — the cross-tenant predicate above
            // already proved the object is in-subtree. Global (None)
            // rules always evaluate. Super-admin matches every rule.
            if let Some(rule_tenant) = &rule.tenant_id {
                if !principal.is_super_admin() && !principal.administers_tenant(rule_tenant) {
                    continue;
                }
            }
            if !role_matches(&rule.role, &roles) {
                continue;
            }
            if !resource_matches(&rule.resource, &object.kind) {
                continue;
            }
            if !instance_matches(&rule.resource_id, &object.id) {
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
                Some(CompiledCondition::Expr(e)) => match e.try_eval(&ctx) {
                    Ok(v) => v,
                    Err(err) => {
                        // SCOPE-EXT.md R13 — `contains` LHS that
                        // isn't an array is a loud-failure deny,
                        // not a silent false. We short-circuit
                        // the whole `check()` so the operator
                        // sees the malformed rule on first hit.
                        tracing::error!(
                            subject = %principal.subject,
                            action = %action,
                            kind = %object.kind,
                            rule = %rule.id,
                            error = %err,
                            "authz rule evaluation error"
                        );
                        let decision = Decision::deny_by("condition_invalid", rule.id.clone());
                        self.audit(principal, action, object, &decision).await;
                        return decision;
                    }
                },
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

        self.audit(principal, action, object, &decision).await;
        decision
    }
}

impl StaticRbacEngine {
    /// Build a [`DecisionEntry`] from the decision and push it
    /// through the configured sink. Default sink is the
    /// [`NoopDecisionSink`] silent-drop; a `DbDecisionSink` does a
    /// non-blocking `try_send` so this never blocks `check()`.
    async fn audit(
        &self,
        principal: &Principal,
        action: &str,
        object: &ResourceRef,
        decision: &Decision,
    ) {
        let (effect, rule_id, reason) = match decision {
            Decision::Allow { matched_rule } => (Effect::Allow, matched_rule.clone(), None),
            Decision::Deny {
                reason,
                matched_rule,
            } => (Effect::Deny, matched_rule.clone(), Some(reason.clone())),
        };
        let entry = DecisionEntry {
            at: chrono::Utc::now(),
            tenant: principal.tenant_id.clone(),
            subject: principal.subject.clone(),
            principal_role: role_name(principal.role).to_string(),
            action: action.to_string(),
            kind: object.kind.clone(),
            id: object.id.clone(),
            effect,
            rule_id,
            reason,
            surface: crate::surface::current_surface(),
        };
        self.sink.record(entry).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::StaticRegistry;
    use starter_spi::auth::{Role, Scope};

    fn principal_with_teams(teams: Vec<String>) -> Principal {
        Principal {
            subject: "u".into(),
            role: Role::Reader,
            scopes: vec![Scope("reader".into())],
            tenant_id: None,
            teams,
            tenant_scope: Vec::new(),
            extra: Value::Null,
        }
    }

    #[test]
    fn roles_for_synthesises_team_slug_from_principal_teams() {
        let registry = Arc::new(StaticRegistry::new());
        let engine =
            StaticRbacEngine::from_config(AuthzConfig::default(), registry).expect("engine");
        let p = principal_with_teams(vec!["hvac-ops".into(), "alerts".into()]);
        let roles = engine.roles_for(&p);
        assert!(roles.contains(&"reader".to_string()));
        assert!(roles.contains(&"team:hvac-ops".to_string()));
        assert!(roles.contains(&"team:alerts".to_string()));
        // Deduplication when an assignment already named the team.
        let p2 = principal_with_teams(vec!["hvac-ops".into(), "hvac-ops".into()]);
        let roles2 = engine.roles_for(&p2);
        assert_eq!(
            roles2.iter().filter(|r| *r == "team:hvac-ops").count(),
            1
        );
    }

    fn page_registry() -> Arc<StaticRegistry> {
        let reg = Arc::new(StaticRegistry::new());
        reg.register(starter_spi::authz::ResourceSpec::from_static(
            "page",
            &["view", "edit"],
            starter_spi::authz::Ownership::Subject,
            "Page",
            "test page",
        ));
        reg
    }

    fn allow_rule(resource_id: Option<&str>) -> Rule {
        Rule {
            id: Some("grant-1".into()),
            role: "team:ops".into(),
            resource: "page".into(),
            actions: vec!["view".into()],
            condition: None,
            effect: Effect::Allow,
            priority: 100,
            tenant_id: None,
            resource_id: resource_id.map(|s| s.to_string()),
        }
    }

    fn ops_member() -> Principal {
        Principal {
            subject: "u".into(),
            role: Role::Reader,
            scopes: vec![],
            tenant_id: None,
            teams: vec!["ops".into()],
            tenant_scope: Vec::new(),
            extra: Value::Null,
        }
    }

    async fn check_id(rule: Rule, object_id: &str) -> Decision {
        let cfg = AuthzConfig {
            default_policy: false,
            assignments: vec![],
            rules: vec![rule],
        };
        let engine = StaticRbacEngine::from_config(cfg, page_registry()).expect("engine");
        engine
            .check(&ops_member(), "view", &ResourceRef::row("page", object_id))
            .await
    }

    #[tokio::test]
    async fn instance_grant_authorizes_only_its_own_instance() {
        // A grant carrying a concrete resource_id allows that page…
        assert!(check_id(allow_rule(Some("page-a")), "page-a").await.is_allow());
        // …and does not leak to a sibling page in the same kind/tenant.
        assert!(!check_id(allow_rule(Some("page-a")), "page-b").await.is_allow());
    }

    #[tokio::test]
    async fn kind_wide_rule_still_matches_every_instance() {
        // The pre-instance behaviour: a rule with no resource_id (or "*")
        // applies to every instance of the kind.
        assert!(check_id(allow_rule(None), "page-a").await.is_allow());
        assert!(check_id(allow_rule(None), "page-z").await.is_allow());
        assert!(check_id(allow_rule(Some("*")), "anything").await.is_allow());
    }
}
