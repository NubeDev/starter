//! Effective-ACL summariser.
//!
//! Given the tenant's rules for one resource kind, derive an
//! [`EffectiveAcl`] per resource instance. The summariser is
//! pure — it does not touch the DB. Callers (instance providers
//! and per-team Permissions views) batch-load rules once, then
//! call this per page to avoid N+1 lookups.
//!
//! **G2 caveat**: until G3 lands the additive migration that
//! adds a `resource_id` column to `starter_authz_rules`, rules
//! cannot target a specific instance — they apply kind-wide.
//! The summariser treats every rule for the kind as applicable
//! to every instance, which matches the engine's behavior
//! today. When G3 lands, switch the bucketing to read
//! `rule.resource_id` and treat `None` / `"*"` as the tenant-wide
//! fallback.

use std::collections::HashMap;

use crate::instances::{
    EffectiveAcl, GrantSummary, PermissionTier, ShareScope, SubjectRef,
};
use crate::store::StoredRule;

/// Kind id used by the v1 grant tier mapping.
pub const RUBIX_DASHBOARD_PAGE: &str = "rubix.dashboard.page";

/// Tier → actions table for a kind. Returns `None` when the kind
/// has no tier mapping declared (only `rubix.dashboard.page` in
/// v1; other kinds are expanded as they ship Simple-mode UIs).
pub fn actions_for_tier(kind: &str, tier: PermissionTier) -> Option<&'static [&'static str]> {
    match (kind, tier) {
        (RUBIX_DASHBOARD_PAGE, PermissionTier::View) => Some(&["view"]),
        (RUBIX_DASHBOARD_PAGE, PermissionTier::Edit) => Some(&["view", "edit"]),
        (RUBIX_DASHBOARD_PAGE, PermissionTier::Manage) => Some(&["view", "edit", "delete"]),
        _ => None,
    }
}

/// Infer a tier from the action list a rule carries. Returns the
/// highest tier whose actions are all satisfied. `["*"]` (any
/// action) maps to [`PermissionTier::Manage`].
pub fn tier_for_actions(kind: &str, actions: &[String]) -> Option<PermissionTier> {
    if actions.iter().any(|a| a == "*") {
        return Some(PermissionTier::Manage);
    }
    // Try Manage → Edit → View; first whose actions are all present wins.
    for tier in [
        PermissionTier::Manage,
        PermissionTier::Edit,
        PermissionTier::View,
    ] {
        let Some(needed) = actions_for_tier(kind, tier) else {
            continue;
        };
        if needed.iter().all(|n| actions.iter().any(|a| a == n)) {
            return Some(tier);
        }
    }
    None
}

/// Owner reference for a single instance — passed by the provider
/// so the summariser can decide Private-vs-Specific without
/// knowing the kind's ownership model.
#[derive(Debug, Clone)]
pub struct InstanceOwner {
    /// Subject id of the owner principal.
    pub subject: String,
}

/// Summarise the effective ACL for one instance, given the rules
/// pre-filtered to the matching kind + tenant.
///
/// `kind` selects the tier mapping; `rules` should be the rule
/// rows where `resource == kind` (and tenant matches). `owner` is
/// used to skip owner-self grants when classifying `share_scope`.
pub fn summarise(
    kind: &str,
    rules: &[&StoredRule],
    owner: Option<&InstanceOwner>,
) -> EffectiveAcl {
    let mut has_legacy_rules = false;
    let mut highest: HashMap<String, (SubjectRef, PermissionTier)> = HashMap::new();
    let mut tenant_view = false;

    for rule in rules {
        if rule.condition.as_deref().is_some_and(|c| !c.is_empty()) {
            has_legacy_rules = true;
            // Condition-based rules don't bucket cleanly into the
            // tier ladder; skip them in the grants list but flag
            // their presence so the UI can show the legacy badge.
            continue;
        }
        if rule.effect != "allow" {
            // Denies are out of scope for the Simple drawer — the
            // grants ladder is Allow-only. Engine still evaluates
            // denies normally.
            continue;
        }
        let Some(subject) = SubjectRef::parse(&rule.role) else {
            continue;
        };
        let Some(tier) = tier_for_actions(kind, &rule.actions) else {
            continue;
        };

        if matches!(subject, SubjectRef::Wildcard) && tier == PermissionTier::View {
            tenant_view = true;
        }

        let key = match &subject {
            SubjectRef::Team { slug } => format!("team:{slug}"),
            SubjectRef::User { sub } => format!("user:{sub}"),
            SubjectRef::Wildcard => "*".to_string(),
        };

        highest
            .entry(key)
            .and_modify(|(_, t)| {
                if tier > *t {
                    *t = tier;
                }
            })
            .or_insert((subject, tier));
    }

    let mut grants: Vec<GrantSummary> = highest
        .into_values()
        .map(|(subject, tier)| GrantSummary { subject, tier })
        .collect();
    grants.sort_by(|a, b| {
        b.tier
            .cmp(&a.tier)
            .then_with(|| subject_label(&a.subject).cmp(&subject_label(&b.subject)))
    });

    let non_owner_specific_grants = grants.iter().any(|g| match &g.subject {
        SubjectRef::Wildcard => false,
        SubjectRef::Team { .. } => true,
        SubjectRef::User { sub } => owner.map(|o| o.subject != *sub).unwrap_or(true),
    });

    let share_scope = if non_owner_specific_grants {
        ShareScope::Specific
    } else if tenant_view {
        ShareScope::Tenant
    } else {
        ShareScope::Private
    };

    EffectiveAcl {
        share_scope,
        grants,
        has_legacy_rules,
    }
}

fn subject_label(s: &SubjectRef) -> String {
    match s {
        SubjectRef::Team { slug } => format!("team:{slug}"),
        SubjectRef::User { sub } => format!("user:{sub}"),
        SubjectRef::Wildcard => "*".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, role: &str, actions: &[&str], condition: Option<&str>) -> StoredRule {
        StoredRule {
            id: id.to_string(),
            role: role.to_string(),
            resource: RUBIX_DASHBOARD_PAGE.to_string(),
            actions: actions.iter().map(|s| s.to_string()).collect(),
            condition: condition.map(String::from),
            effect: "allow".to_string(),
            priority: 100,
            created_by: "test".to_string(),
            tenant_id: Some("t1".to_string()),
        }
    }

    #[test]
    fn tier_for_actions_picks_highest_satisfied_set() {
        assert_eq!(
            tier_for_actions(RUBIX_DASHBOARD_PAGE, &["view".into()]),
            Some(PermissionTier::View)
        );
        assert_eq!(
            tier_for_actions(RUBIX_DASHBOARD_PAGE, &["view".into(), "edit".into()]),
            Some(PermissionTier::Edit)
        );
        assert_eq!(
            tier_for_actions(
                RUBIX_DASHBOARD_PAGE,
                &["view".into(), "edit".into(), "delete".into()]
            ),
            Some(PermissionTier::Manage)
        );
        assert_eq!(
            tier_for_actions(RUBIX_DASHBOARD_PAGE, &["*".into()]),
            Some(PermissionTier::Manage)
        );
    }

    #[test]
    fn buckets_grants_picks_highest_tier() {
        let r1 = rule("a", "team:hvac-ops", &["view"], None);
        let r2 = rule("b", "team:hvac-ops", &["view", "edit"], None);
        let acl = summarise(RUBIX_DASHBOARD_PAGE, &[&r1, &r2], None);
        assert_eq!(acl.grants.len(), 1);
        assert_eq!(acl.grants[0].tier, PermissionTier::Edit);
        assert_eq!(acl.share_scope, ShareScope::Specific);
    }

    #[test]
    fn flags_legacy_rules_with_conditions() {
        let r1 = rule(
            "a",
            "*",
            &["view"],
            Some("principal.teams contains \"hvac-ops\""),
        );
        let acl = summarise(RUBIX_DASHBOARD_PAGE, &[&r1], None);
        assert!(acl.has_legacy_rules);
        assert!(acl.grants.is_empty());
    }

    #[test]
    fn detects_tenant_share_scope_from_wildcard_subject() {
        let r1 = rule("a", "*", &["view"], None);
        let acl = summarise(RUBIX_DASHBOARD_PAGE, &[&r1], None);
        assert_eq!(acl.share_scope, ShareScope::Tenant);
        assert_eq!(acl.grants.len(), 1);
    }

    #[test]
    fn private_when_no_non_owner_grants() {
        let acl = summarise(
            RUBIX_DASHBOARD_PAGE,
            &[],
            Some(&InstanceOwner {
                subject: "alice".into(),
            }),
        );
        assert_eq!(acl.share_scope, ShareScope::Private);
        assert!(acl.grants.is_empty());
    }

    #[test]
    fn owner_self_user_grant_does_not_flip_to_specific() {
        // A user-grant where the subject is the owner shouldn't
        // alone mark the page as "specific" — it's still
        // effectively private.
        let r1 = rule("a", "user:alice", &["view", "edit"], None);
        let acl = summarise(
            RUBIX_DASHBOARD_PAGE,
            &[&r1],
            Some(&InstanceOwner {
                subject: "alice".into(),
            }),
        );
        assert_eq!(acl.share_scope, ShareScope::Private);
    }
}
