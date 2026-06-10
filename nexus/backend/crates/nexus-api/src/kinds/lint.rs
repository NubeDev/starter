//! Boot-time lints over a kind's SQL — the structural-isolation guarantee.
//!
//! Two checks run when a pack loads, and a failure aborts startup so a malformed
//! kind never ships:
//!
//! 1. **Declared-param lint:** every `$name` the SQL references must be a host
//!    token, a `$__` macro, or a param the schema declares. A typo'd `$regon`
//!    fails to load rather than becoming an undefined-variable error at runtime.
//! 2. **Tenant-predicate lint (§4.4):** the data-side DBs have no RLS, so a kind
//!    that reads a tenant-scoped `tables:` entry must isolate rows itself with a
//!    `$caller_tenant_id` predicate. A kind whose SQL omits it fails to load.

use super::error::KindError;
use super::kind::QueryKind;

/// The host-bound tokens the binder injects; never declared as params.
const HOST_TOKENS: [&str; 2] = ["caller_tenant_id", "caller_user_id"];

/// Run every load-time lint over `kind`. Returns the first failure.
pub fn check(kind: &QueryKind) -> Result<(), KindError> {
    let refs = referenced_names(&kind.sql);
    check_declared_params(kind, &refs)?;
    check_tenant_predicate(kind, &refs)?;
    Ok(())
}

/// Reject a `$name` reference that is neither a host token, a `$__` macro, nor a
/// declared param — the SQL would fail at bind time, so catch it at load.
fn check_declared_params(kind: &QueryKind, refs: &[String]) -> Result<(), KindError> {
    let declared = kind.declared_params();
    for name in refs {
        let known = HOST_TOKENS.contains(&name.as_str())
            || name.starts_with("__")
            || declared.contains(name);
        if !known {
            return Err(KindError::Lint {
                kind: kind.name.clone(),
                detail: format!(
                    "SQL references `${name}`, which is not a host token, a macro, or a declared param"
                ),
            });
        }
    }
    Ok(())
}

/// Enforce the mandatory `$caller_tenant_id` predicate when the kind declares any
/// `tables:`. Without RLS on the data side, this predicate is the only thing
/// isolating one tenant's rows from another's, so its absence is a load failure.
fn check_tenant_predicate(kind: &QueryKind, refs: &[String]) -> Result<(), KindError> {
    if kind.tables.is_empty() {
        // A kind that declares no tenant-scoped tables (e.g. `SELECT now()`) has
        // no rows to isolate; the predicate is not required.
        return Ok(());
    }
    if refs.iter().any(|n| n == "caller_tenant_id") {
        return Ok(());
    }
    Err(KindError::Lint {
        kind: kind.name.clone(),
        detail: format!(
            "reads tenant-scoped tables {:?} but the SQL omits the mandatory \
             `$caller_tenant_id` predicate (the data side has no RLS — §4.4)",
            kind.tables
        ),
    })
}

/// Collect every `$name` token the SQL references, stripping the `$__` macro
/// prefix to its bare name and ignoring `${...}` braces and stray `$`. Mirrors
/// the binder's token grammar so the lint sees exactly what the binder will.
fn referenced_names(sql: &str) -> Vec<String> {
    let bytes = sql.as_bytes();
    let mut names = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'$' {
            i += 1;
            continue;
        }
        let rest = &sql[i..];
        if rest.starts_with("${") {
            // A braced variable references a dashboard var, not a kind param; the
            // declared-param lint does not apply to it.
            if let Some(close) = rest.find('}') {
                i += close + 1;
            } else {
                i += 1;
            }
            continue;
        }
        // `$name` or `$__macro` — take the identifier run after the `$`.
        let after = &rest[1..];
        let ident = leading_ident(after);
        if ident.is_empty() {
            i += 1;
            continue;
        }
        names.push(ident.to_string());
        i += 1 + ident.len();
    }
    names
}

/// The leading `[A-Za-z_][A-Za-z0-9_]*` run of `s`, or `""` if none. A leading
/// `__` (a macro) is kept so the caller can detect macros by prefix.
fn leading_ident(s: &str) -> &str {
    let end = s
        .char_indices()
        .take_while(|(i, c)| {
            if *i == 0 {
                c.is_ascii_alphabetic() || *c == '_'
            } else {
                c.is_ascii_alphanumeric() || *c == '_'
            }
        })
        .count();
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build a kind with the given SQL, declared params, and tables for linting.
    fn kind(sql: &str, params: &[&str], tables: &[&str]) -> QueryKind {
        let props: serde_json::Map<String, serde_json::Value> = params
            .iter()
            .map(|p| ((*p).to_string(), json!({ "type": "string" })))
            .collect();
        QueryKind {
            name: "nexus.test.k".to_string(),
            sql: sql.to_string(),
            params_schema: json!({ "type": "object", "properties": props }),
            datasource_kind: "postgres".to_string(),
            tables: tables.iter().map(|t| (*t).to_string()).collect(),
            datasource_binding: None,
            description: None,
        }
    }

    #[test]
    fn referenced_names_collects_params_macros_and_host_tokens() {
        let names = referenced_names(
            "SELECT $__timeGroup(ts, $__interval) FROM h \
             WHERE tenant_id = $caller_tenant_id AND site_id = $site_id",
        );
        assert!(names.contains(&"__timeGroup".to_string()));
        assert!(names.contains(&"__interval".to_string()));
        assert!(names.contains(&"caller_tenant_id".to_string()));
        assert!(names.contains(&"site_id".to_string()));
    }

    #[test]
    fn referenced_names_ignores_braced_dashboard_vars() {
        let names = referenced_names("SELECT * FROM h WHERE x = ${dashvar:csv}");
        assert!(!names.iter().any(|n| n.contains("dashvar")));
    }

    #[test]
    fn declared_param_lint_accepts_known_references() {
        let k = kind(
            "SELECT * FROM m WHERE tenant_id = $caller_tenant_id AND site_id = $site_id",
            &["site_id"],
            &["m"],
        );
        check(&k).expect("all references are known");
    }

    #[test]
    fn declared_param_lint_rejects_undeclared_param() {
        let k = kind(
            "SELECT * FROM m WHERE tenant_id = $caller_tenant_id AND s = $regon",
            &["site_id"],
            &["m"],
        );
        let err = check(&k).expect_err("undeclared $regon must fail the lint");
        assert!(matches!(err, KindError::Lint { .. }));
        assert!(err.to_string().contains("regon"));
    }

    #[test]
    fn tenant_predicate_lint_requires_caller_tenant_id_for_tables() {
        let k = kind("SELECT * FROM m WHERE site_id = $site_id", &["site_id"], &["m"]);
        let err = check(&k).expect_err("a tenant-scoped table without the predicate must fail");
        assert!(matches!(err, KindError::Lint { .. }));
        assert!(err.to_string().contains("caller_tenant_id"));
    }

    #[test]
    fn tenant_predicate_lint_skipped_when_no_tables() {
        let k = kind("SELECT now() AS ts", &[], &[]);
        check(&k).expect("a kind with no tenant-scoped tables needs no predicate");
    }
}
