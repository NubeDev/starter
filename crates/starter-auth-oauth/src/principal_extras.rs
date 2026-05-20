//! `OAuthPrincipalExtras` — the
//! [`starter_auth_users::PrincipalExtrasLookup`] impl that stamps
//! the reserved `oauth.*` block on `Principal.extra` for every
//! authenticated request whose user has at least one linked
//! identity in `oauth_identities`. Companion to
//! [`crate::OAuthLinkedProviders`].
//!
//! See `DOCS/auth/authz/SCOPE.md` R8 for the wire shape:
//!
//! ```jsonc
//! {
//!   "oauth": {
//!     "provider":         "google",
//!     "provider_sub":     "1234567890",
//!     "email":            "alice@acme.com",
//!     "email_domain":     "acme.com",
//!     "email_verified":   true,
//!     "linked_providers": ["github", "google"]
//!   }
//! }
//! ```
//!
//! Resolution rules:
//!
//! - The user has no OAuth identities → return `Value::Null`.
//!   `Principal.extra` then stays `Value::Null`, identical to the
//!   pre-authz behaviour.
//! - The user has one or more identities → return a JSON object
//!   carrying the `oauth.*` block. `provider`, `provider_sub`,
//!   and `email` describe the user's *most recently linked*
//!   identity (the last row in
//!   [`IdentityStore::list_for_user`]'s oldest-first list);
//!   `linked_providers` enumerates *every* provider the user has
//!   linked, de-duplicated and ordered oldest-first.
//! - `email_verified` is always `true` for rows that survived the
//!   callback path — Hard rule R3 of the OAuth crate refuses to
//!   auto-link an unverified email, so a row in
//!   `oauth_identities` is by construction a verified email.
//!   (Once `OAuthIdentity` carries an explicit `email_verified`
//!   column we can stop synthesising it.)
//! - Rows with `email = None` (provider returned no usable email)
//!   emit `email_domain = null` and `email = null` rather than
//!   omitting the keys, so rules that match on `oauth.email ==
//!   ""` or `oauth.email_verified` evaluate deterministically.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{json, Value};
use starter_auth_users::{PrincipalExtrasError, PrincipalExtrasLookup};

use crate::identity_store::IdentityStore;

/// Stamps the `oauth.*` block on `Principal.extra` by reading
/// `oauth_identities` for the principal's user id.
pub struct OAuthPrincipalExtras {
    store: Arc<dyn IdentityStore>,
}

impl OAuthPrincipalExtras {
    /// Wrap an [`IdentityStore`] so the auth-users session-mint
    /// path can use it as a `PrincipalExtrasLookup`.
    pub fn new(store: Arc<dyn IdentityStore>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl PrincipalExtrasLookup for OAuthPrincipalExtras {
    async fn extras_for(&self, user_id: &str) -> Result<Value, PrincipalExtrasError> {
        let rows = self
            .store
            .list_for_user(user_id)
            .await
            .map_err(|e| PrincipalExtrasError::Backend(e.to_string()))?;

        if rows.is_empty() {
            return Ok(Value::Null);
        }

        // `list_for_user` returns oldest-first. The most recently
        // linked identity is the wire-shape "primary" — it is the
        // one the user most likely just signed in with.
        let primary = rows.last().expect("non-empty checked above").clone();

        // De-duplicate provider ids while preserving oldest-first
        // order; mirrors `OAuthLinkedProviders::linked_providers`.
        let mut linked_providers: Vec<String> = Vec::with_capacity(rows.len());
        for r in &rows {
            if !linked_providers.contains(&r.provider) {
                linked_providers.push(r.provider.clone());
            }
        }

        let email = primary.email.clone();
        let email_domain = email
            .as_deref()
            .and_then(|e| e.rsplit_once('@').map(|(_, d)| d.to_string()));

        let obj = json!({
            "oauth": {
                "provider":         primary.provider,
                "provider_sub":     primary.provider_sub,
                "email":            email,
                "email_domain":     email_domain,
                // See module docs: rows in `oauth_identities` only
                // exist for verified emails, by callback construction.
                "email_verified":   true,
                "linked_providers": linked_providers,
            }
        });

        Ok(obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity_store::{IdentityStoreError, OAuthIdentity};
    use chrono::{Duration, Utc};

    struct StubStore {
        rows: Vec<OAuthIdentity>,
    }

    #[async_trait]
    impl IdentityStore for StubStore {
        async fn find(
            &self,
            _provider: &str,
            _provider_sub: &str,
        ) -> Result<Option<OAuthIdentity>, IdentityStoreError> {
            Ok(None)
        }
        async fn insert(&self, _i: &OAuthIdentity) -> Result<(), IdentityStoreError> {
            Ok(())
        }
        async fn delete(
            &self,
            _provider: &str,
            _provider_sub: &str,
        ) -> Result<(), IdentityStoreError> {
            Ok(())
        }
        async fn list_for_user(
            &self,
            _user_id: &str,
        ) -> Result<Vec<OAuthIdentity>, IdentityStoreError> {
            Ok(self.rows.clone())
        }
    }

    fn row(provider: &str, sub: &str, email: Option<&str>, offset_secs: i64) -> OAuthIdentity {
        OAuthIdentity {
            provider: provider.into(),
            provider_sub: sub.into(),
            user_id: "u1".into(),
            email: email.map(String::from),
            display_name: None,
            linked_at: Utc::now() + Duration::seconds(offset_secs),
        }
    }

    #[tokio::test]
    async fn no_identities_returns_null() {
        let store = Arc::new(StubStore { rows: vec![] });
        let lookup = OAuthPrincipalExtras::new(store);
        let v = lookup.extras_for("u1").await.unwrap();
        assert_eq!(v, Value::Null);
    }

    #[tokio::test]
    async fn single_identity_populates_oauth_block() {
        let store = Arc::new(StubStore {
            rows: vec![row("google", "g-1", Some("alice@acme.com"), 0)],
        });
        let lookup = OAuthPrincipalExtras::new(store);
        let v = lookup.extras_for("u1").await.unwrap();
        let oauth = &v["oauth"];
        assert_eq!(oauth["provider"], "google");
        assert_eq!(oauth["provider_sub"], "g-1");
        assert_eq!(oauth["email"], "alice@acme.com");
        assert_eq!(oauth["email_domain"], "acme.com");
        assert_eq!(oauth["email_verified"], true);
        assert_eq!(oauth["linked_providers"], json!(["google"]));
    }

    #[tokio::test]
    async fn most_recent_identity_is_primary() {
        let store = Arc::new(StubStore {
            rows: vec![
                row("github", "gh-1", Some("alice@acme.com"), -10),
                row("google", "g-1", Some("alice@acme.com"), 0),
            ],
        });
        let lookup = OAuthPrincipalExtras::new(store);
        let v = lookup.extras_for("u1").await.unwrap();
        assert_eq!(v["oauth"]["provider"], "google");
        // Order preserved oldest-first.
        assert_eq!(v["oauth"]["linked_providers"], json!(["github", "google"]));
    }

    #[tokio::test]
    async fn missing_email_is_explicit_null() {
        let store = Arc::new(StubStore {
            rows: vec![row("github", "gh-1", None, 0)],
        });
        let lookup = OAuthPrincipalExtras::new(store);
        let v = lookup.extras_for("u1").await.unwrap();
        assert_eq!(v["oauth"]["email"], Value::Null);
        assert_eq!(v["oauth"]["email_domain"], Value::Null);
    }
}
