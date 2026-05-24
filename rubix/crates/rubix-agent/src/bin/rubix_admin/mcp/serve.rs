//! `rubix-admin mcp` — run starter-mcp's stdio JSON-RPC loop
//! wrapping the same [`FlowAsTool`] registry the HTTP MCP router
//! mounts. Framing is owned upstream by `starter-jsonrpc-stdio`
//! (and re-exported through `starter-mcp::run_stdio`); this file is
//! pure wiring.
//!
//! ## Responsibilities
//!
//!   1. Build the shared MCP tool registry via
//!      [`rubix_agent::boot::mcp::build_tool_registry`] so the tool
//!      catalogue is identical to the HTTP path.
//!   2. Resolve the stdio session principal from
//!      `RUBIX_PRINCIPAL_EMAIL`. When the agent has a Postgres DSN
//!      (`RUBIX_DATABASE_URL` or `[database_url]` in the loaded
//!      [`AgentConfig`]) the email is looked up via
//!      [`starter_auth_users::store::PgUserStore`]; missing user →
//!      a localised [`Diagnostic`] on stderr and a non-zero exit.
//!      When no DSN is configured the binary still boots so a
//!      developer can drive `tools/list` and `tools/call` against
//!      a laptop without a database — same tolerance the rubix-
//!      agent HTTP binary already applies.
//!   3. Bind a session-wide locale fallback from `LANG`. Per-call
//!      `params._meta.acceptLanguage` (handled inside
//!      `starter-mcp::run_stdio`) still wins; the LANG fallback
//!      only activates when the MCP client did not negotiate a
//!      locale on `initialize`. Final fallback is `"en"`.
//!   4. Bind the resolved [`Principal`] on starter-mcp's
//!      task-local for the lifetime of the stdio loop so any
//!      audited tool body sees the actor for changelog rows.
//!
//! `stdout` is reserved for MCP framing — every operator-visible
//! message goes to `stderr`. Exit code is `0` on a clean stdin
//! close, `1` on a fatal configuration or I/O error.

use anyhow::Result;

use rubix_agent::boot::{mcp::prefs_from_locale, AgentConfig};
use starter_auth_users::store::{PgUserStore, UserStore};
use starter_spi::auth::{Principal, Role};
use starter_spi::i18n::{Diagnostic, LanguageTag, MessageKey};
use starter_store_postgres::pool::connect as pg_connect;

use super::Args;

const PRINCIPAL_EMAIL_ENV: &str = "RUBIX_PRINCIPAL_EMAIL";

pub async fn run(_args: Args) -> Result<()> {
    // Read locale fallback once at process startup — the inner
    // dispatcher overrides per-call via `_meta.acceptLanguage`.
    let lang_env = std::env::var("LANG").ok();
    let session_locale = lang_env
        .as_deref()
        .and_then(parse_lang_env)
        .unwrap_or_else(|| LanguageTag::parse("en").expect("'en' parses"));

    let cfg = AgentConfig::load()
        .map_err(|e| anyhow::anyhow!("load AgentConfig: {e}"))?;

    // Resolve the actor. Errors here render through the rubix
    // bundle in the session locale and exit non-zero — `stdout`
    // is reserved for MCP framing.
    let principal = match resolve_principal(&cfg, &session_locale).await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e.rendered);
            std::process::exit(1);
        }
    };

    // Shared composition — identical tool catalogue to the HTTP
    // surface. `run_stdio` consumes the registry by value.
    let tools = rubix_agent::boot::mcp::build_tool_registry()
        .await
        .map_err(|e| anyhow::anyhow!("build MCP tool registry: {e}"))?;

    // Wrap the loop in the locale + principal task-locals. The
    // inner dispatcher re-scopes the locale per call when the MCP
    // client supplied `_meta.acceptLanguage`; the principal is
    // session-wide.
    let principal_for_scope = principal.clone();
    let result = starter_mcp::with_principal(principal_for_scope, async move {
        starter_mcp::with_locale(session_locale, starter_mcp::run_stdio(tools)).await
    })
    .await;

    match result {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("rubix-admin mcp: stdio loop failed: {e}");
            std::process::exit(1);
        }
    }
}

/// Outcome of [`resolve_principal`] in the error case — a
/// pre-rendered string the caller writes to `stderr`.
struct PrincipalError {
    rendered: String,
}

async fn resolve_principal(
    cfg: &AgentConfig,
    locale: &LanguageTag,
) -> Result<Principal, PrincipalError> {
    let email = std::env::var(PRINCIPAL_EMAIL_ENV)
        .ok()
        .filter(|s| !s.trim().is_empty());

    let email = match email {
        Some(e) => e,
        None => {
            return Err(render_error(
                locale,
                "rubix.admin.mcp.principal.missing",
                &[],
            ));
        }
    };

    // No DSN configured → degrade gracefully. Mirrors the HTTP
    // binary's tolerance of an unset `database_url` so developers
    // can drive the stdio surface on a laptop without Postgres.
    let Some(dsn) = cfg.database_url.as_deref() else {
        eprintln!(
            "rubix-admin mcp: database_url unset — synthesising stdio principal for {email} without a user-store lookup",
        );
        return Ok(synthetic_principal(&email));
    };

    let pool = pg_connect(dsn).await.map_err(|e| {
        // Connection failures render as not_found so the operator
        // sees a single, actionable error rather than a panic.
        eprintln!("rubix-admin mcp: connect to database_url failed: {e}");
        render_error(
            locale,
            "rubix.admin.mcp.principal.not_found",
            &[("email", &email)],
        )
    })?;

    let store = PgUserStore::new(pool);
    let user = store
        .find_by_email(&email)
        .await
        .map_err(|e| {
            eprintln!("rubix-admin mcp: find_by_email failed: {e}");
            render_error(
                locale,
                "rubix.admin.mcp.principal.not_found",
                &[("email", &email)],
            )
        })?
        .ok_or_else(|| {
            render_error(
                locale,
                "rubix.admin.mcp.principal.not_found",
                &[("email", &email)],
            )
        })?;

    Ok(Principal {
        subject: user.id,
        role: user.role,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    })
}

fn synthetic_principal(email: &str) -> Principal {
    Principal {
        // No user-store row to anchor the subject to; use the
        // email as a stable, human-readable id. Production callers
        // always go through the user-store lookup path above.
        subject: format!("email:{email}"),
        role: Role::Admin,
        scopes: Vec::new(),
        tenant_id: None,
        teams: Vec::new(),
        extra: serde_json::Value::Null,
    }
}

fn render_error(
    locale: &LanguageTag,
    key: &str,
    params: &[(&str, &str)],
) -> PrincipalError {
    let bundle = match rubix_spi::i18n::rubix_bundle() {
        Ok(b) => b,
        Err(e) => {
            return PrincipalError {
                rendered: format!("rubix-admin mcp: i18n bundle failed to load: {e}"),
            };
        }
    };
    let message_key = match MessageKey::parse(key) {
        Ok(k) => k,
        Err(e) => {
            return PrincipalError {
                rendered: format!("rubix-admin mcp: invalid message key {key}: {e}"),
            };
        }
    };
    let mut diag = Diagnostic::new(message_key);
    for (name, value) in params {
        diag = diag.with_param(
            *name,
            starter_spi::i18n::DiagnosticParam::String((*value).to_owned()),
        );
    }
    let prefs = prefs_from_locale(locale);
    let rendered = bundle.render_diagnostic(locale, &diag, &prefs);
    PrincipalError { rendered }
}

/// Parse a POSIX `LANG` value (`es_AR.UTF-8`, `en_US`, `C`) into
/// a BCP-47 [`LanguageTag`]. Returns `None` for `C` / `POSIX` /
/// empty / unparseable values so the caller falls back to `"en"`.
fn parse_lang_env(raw: &str) -> Option<LanguageTag> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "C" || trimmed == "POSIX" {
        return None;
    }
    // Strip codeset / modifier: `es_AR.UTF-8@euro` → `es_AR`.
    let head = trimmed.split(['.', '@']).next().unwrap_or(trimmed);
    // POSIX uses `_` between language + region; BCP-47 uses `-`.
    let bcp47 = head.replace('_', "-");
    LanguageTag::parse(&bcp47).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_posix_lang_with_codeset() {
        let tag = parse_lang_env("es_AR.UTF-8").expect("es_AR.UTF-8 parses");
        assert_eq!(tag.as_str(), "es-AR");
    }

    #[test]
    fn parses_bare_language() {
        let tag = parse_lang_env("en").expect("en parses");
        assert_eq!(tag.as_str(), "en");
    }

    #[test]
    fn rejects_c_locale() {
        assert!(parse_lang_env("C").is_none());
        assert!(parse_lang_env("POSIX").is_none());
        assert!(parse_lang_env("").is_none());
    }

    #[test]
    fn synthetic_principal_uses_email_subject() {
        let p = synthetic_principal("op@example.com");
        assert_eq!(p.subject, "email:op@example.com");
        assert_eq!(p.role, Role::Admin);
    }
}
