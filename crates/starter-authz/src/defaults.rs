//! Built-in role defaults. Loaded before user-supplied rules so a
//! consumer who enables `starter-authz` with no policy file
//! behaves identically to one using the old `require_role`
//! middleware (SCOPE.md R7).
//!
//! The defaults are deliberately conservative:
//!
//! - `Reader`  → `read` on every registered resource.
//! - `Writer`  → `read | create | update` on every registered
//!   resource except the sensitive set
//!   (`users | sessions | tokens | secrets |
//!   oauth_identities`), where they may only update
//!   their own row.
//! - `Admin`   → `*` on every registered resource.
//!
//! Resource kinds are matched with `"*"` because resources are
//! registered at boot and the defaults must apply to extension
//! resources without anyone re-listing them.

use crate::config::{Effect, Rule};

const SENSITIVE: &[&str] = &["users", "sessions", "tokens", "secrets", "oauth_identities"];

/// The default rule set layered in by
/// [`crate::AuthzConfig::default_policy`].
pub fn built_in_rules() -> Vec<Rule> {
    let mut rules = Vec::new();

    // Admin: everything.
    rules.push(Rule {
        id: Some("default-admin-all".into()),
        role: "admin".into(),
        resource: "*".into(),
        actions: vec!["*".into()],
        condition: None,
        effect: Effect::Allow,
        priority: 0,
    });

    // Reader: read everything.
    rules.push(Rule {
        id: Some("default-reader-read".into()),
        role: "reader".into(),
        resource: "*".into(),
        actions: vec!["read".into()],
        condition: None,
        effect: Effect::Allow,
        priority: 0,
    });

    // Writer: read everything.
    rules.push(Rule {
        id: Some("default-writer-read".into()),
        role: "writer".into(),
        resource: "*".into(),
        actions: vec!["read".into()],
        condition: None,
        effect: Effect::Allow,
        priority: 0,
    });

    // Writer: create / update on non-sensitive resources.
    for kind_glob in non_sensitive_writer_rules() {
        rules.push(Rule {
            id: Some(format!("default-writer-cu-{kind_glob}")),
            role: "writer".into(),
            resource: kind_glob.into(),
            actions: vec!["create".into(), "update".into()],
            condition: None,
            effect: Effect::Allow,
            priority: 0,
        });
    }

    // Writer: deny anything beyond `read` on sensitive resources,
    // except update on own row.
    for kind in SENSITIVE {
        rules.push(Rule {
            id: Some(format!("default-writer-own-update-{kind}")),
            role: "writer".into(),
            resource: (*kind).into(),
            actions: vec!["update".into()],
            condition: Some("owner".into()),
            effect: Effect::Allow,
            priority: 0,
        });
        // The writer-cu-* allow above covers `update` on every
        // kind; we need to take it back for sensitive rows the
        // writer does not own. Deny wins per SCOPE.md R3.
        rules.push(Rule {
            id: Some(format!("default-writer-deny-other-update-{kind}")),
            role: "writer".into(),
            resource: (*kind).into(),
            actions: vec!["update".into()],
            condition: Some("subject != object.owner".into()),
            effect: Effect::Deny,
            priority: 0,
        });
        rules.push(Rule {
            id: Some(format!("default-writer-deny-{kind}")),
            role: "writer".into(),
            resource: (*kind).into(),
            actions: vec!["create".into(), "delete".into()],
            condition: None,
            effect: Effect::Deny,
            priority: 0,
        });
    }

    rules
}

/// Wildcard list for the writer create/update allow. `"*"` here is
/// fine because the sensitive deny rules above override per
/// SCOPE.md R3 (deny-overrides).
fn non_sensitive_writer_rules() -> Vec<&'static str> {
    vec!["*"]
}
