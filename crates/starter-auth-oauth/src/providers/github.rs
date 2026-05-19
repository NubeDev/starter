//! GitHub OAuth 2.0 provider impl.
//!
//! Per Hard rule R7, scopes and endpoints are compile-time constants:
//!
//! - `read:user` — needed by `/user` for the numeric `id` (our
//!   `provider_sub`) and the display name.
//! - `user:email` — load-bearing for Hard rule R3. **Only**
//!   `/user/emails` returns the per-email `verified` flag; the public
//!   `/user.email` field is user-edited and never used for linking.
//!
//! The token-exchange + userinfo round trip uses direct `reqwest`
//! calls. We hold a thin `reqwest::Client` configured to refuse
//! redirects (oauth2 example boilerplate calls this out: redirects
//! on the token endpoint are an SSRF vector).
//!
//! Per R2 the access token never leaves [`Self::fetch_identity`]:
//! the local `String` holding it is dropped at the end of the
//! function, no copy is ever returned, persisted, or logged.

use async_trait::async_trait;
use serde::Deserialize;

use crate::provider::{OAuthProvider, ProviderError, ProviderIdentity};

/// Provider id used in path segments and stored in
/// `oauth_identities.provider`. Must match the key consumers use in
/// `OAUTH_GITHUB_CLIENT_ID` etc.
pub const PROVIDER_ID: &str = "github";

/// OAuth 2.0 authorize endpoint.
pub const AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
/// OAuth 2.0 token endpoint.
pub const TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
/// REST `/user` endpoint; returns the numeric subject id and the
/// display name. The `email` field here is user-edited — never use
/// it for linking (Hard rule R3).
pub const USER_URL: &str = "https://api.github.com/user";
/// REST `/user/emails` endpoint; the only source of the per-email
/// `verified` flag R3 depends on.
pub const EMAILS_URL: &str = "https://api.github.com/user/emails";

/// Scopes requested at authorize time. Compile-time constant per R7;
/// an operator who needs a different scope set ships a code change,
/// not a config flag.
pub const SCOPES: &str = "read:user user:email";

/// GitHub provider. Constructed once at startup; cloning is cheap
/// because the inner `reqwest::Client` is `Arc`-counted.
#[derive(Clone)]
pub struct GitHubProvider {
    client_id: String,
    client_secret: String,
    /// Overrides the public github.com URLs in tests. Left `None` in
    /// production; `Some(base)` swaps every URL's host so a mock
    /// HTTP server can stand in for github.com.
    base_override: Option<String>,
    http: reqwest::Client,
}

impl GitHubProvider {
    /// Build a production-config provider against github.com.
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            base_override: None,
            http: build_client(),
        }
    }

    /// Build a provider whose API + auth URLs share a single base
    /// override. Used by tests; the prefix is treated as the host +
    /// scheme and concatenated with the path portions of the public
    /// URLs (`/login/...`, `/user`, `/user/emails`).
    ///
    /// The split is provider-side because individual paths
    /// (`/user`, `/login/oauth/access_token`, ...) have to survive
    /// host substitution without callers reasoning about which
    /// endpoint goes where.
    #[doc(hidden)]
    pub fn with_base_override(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        base: impl Into<String>,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            base_override: Some(base.into()),
            http: build_client(),
        }
    }

    fn authorize(&self) -> String {
        self.rewrite(AUTHORIZE_URL)
    }

    fn token(&self) -> String {
        self.rewrite(TOKEN_URL)
    }

    fn user(&self) -> String {
        self.rewrite(USER_URL)
    }

    fn emails(&self) -> String {
        self.rewrite(EMAILS_URL)
    }

    fn rewrite(&self, public: &str) -> String {
        match &self.base_override {
            None => public.to_string(),
            Some(base) => {
                // Reuse the path + query suffix of the public URL.
                // GitHub's auth host (`github.com`) and API host
                // (`api.github.com`) differ; the override collapses
                // them onto one mock host because tests don't care
                // about the split.
                let path = public.split_once("://").map(|(_, rest)| rest).unwrap_or(public);
                let path = path
                    .split_once('/')
                    .map(|(_, rest)| format!("/{rest}"))
                    .unwrap_or_else(|| "/".to_string());
                format!("{}{}", base.trim_end_matches('/'), path)
            }
        }
    }
}

fn build_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("starter-auth-oauth")
        // Redirects on the token endpoint are an SSRF vector; the
        // oauth2 crate's own examples set this explicitly.
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client builds with no I/O")
}

#[async_trait]
impl OAuthProvider for GitHubProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn authorize_url(&self, state: &str, pkce_challenge: &str, redirect_uri: &str) -> String {
        let mut url = url::Url::parse(&self.authorize()).expect("AUTHORIZE_URL is valid");
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("scope", SCOPES)
            .append_pair("state", state)
            // GitHub honours PKCE since 2023; pass S256 challenge so
            // a stolen `code` cannot be exchanged without the
            // verifier the state store guards.
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256")
            // Always show the consent screen on link-mode flows so
            // the user is reminded what permissions they granted; for
            // sign-in flows GitHub will skip it after first consent
            // regardless of this hint.
            .append_pair("allow_signup", "true");
        url.into()
    }

    async fn fetch_identity(
        &self,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
    ) -> Result<ProviderIdentity, ProviderError> {
        // 1. Code → access token. POST application/x-www-form-urlencoded;
        // GitHub's docs say the JSON-body variant works too but the
        // form variant has no surprise quirks across regional gateways.
        let form = [
            ("client_id", self.client_id.as_str()),
            ("client_secret", self.client_secret.as_str()),
            ("code", code),
            ("redirect_uri", redirect_uri),
            ("code_verifier", pkce_verifier),
            ("grant_type", "authorization_code"),
        ];
        let token_resp: TokenResponse = self
            .http
            .post(self.token())
            .header("Accept", "application/json")
            .form(&form)
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("token body: {e}")))?;

        // 2. Userinfo. The access token is read-only here; the local
        // binding is dropped at function exit (R2).
        let access_token = token_resp.access_token;
        let user: GitHubUser = self
            .http
            .get(self.user())
            .bearer_auth(&access_token)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("user body: {e}")))?;

        // 3. Emails. Only addresses with `verified: true` count
        // (Hard rule R3). The `primary` flag picks which verified
        // address to bind when the user has more than one; if no
        // primary is verified, fall back to the first verified
        // entry (still safe per R3).
        let emails: Vec<GitHubEmail> = self
            .http
            .get(self.emails())
            .bearer_auth(&access_token)
            .header("Accept", "application/vnd.github+json")
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("emails body: {e}")))?;

        let picked = pick_email(&emails).ok_or(ProviderError::UnverifiedEmail)?;

        Ok(ProviderIdentity {
            provider_sub: user.id.to_string(),
            email: picked.to_string(),
            email_verified: true,
            display_name: user.name.or(Some(user.login)),
        })
    }
}

fn pick_email(emails: &[GitHubEmail]) -> Option<&str> {
    emails
        .iter()
        .find(|e| e.verified && e.primary)
        .or_else(|| emails.iter().find(|e| e.verified))
        .map(|e| e.email.as_str())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct GitHubUser {
    /// Numeric id — the stable subject across email / username
    /// changes. This is what backs `provider_sub` in
    /// `oauth_identities`.
    id: u64,
    /// Login (username). Not stable — users rename — so it is only
    /// used as a fallback display name.
    login: String,
    /// Real-name field. Optional even on populated profiles.
    #[serde(default)]
    name: Option<String>,
}

#[derive(Deserialize)]
struct GitHubEmail {
    email: String,
    primary: bool,
    verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_contains_compile_time_scopes() {
        let p = GitHubProvider::new("client-id", "client-secret");
        let url = p.authorize_url("state-xyz", "challenge-abc", "https://app/cb");
        assert!(url.starts_with("https://github.com/login/oauth/authorize?"));
        assert!(url.contains("scope=read%3Auser+user%3Aemail"));
        assert!(url.contains("state=state-xyz"));
        assert!(url.contains("code_challenge=challenge-abc"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fapp%2Fcb"));
    }

    #[test]
    fn pick_email_prefers_primary_verified() {
        let emails = vec![
            GitHubEmail {
                email: "noreply@github.com".into(),
                primary: false,
                verified: true,
            },
            GitHubEmail {
                email: "me@example.com".into(),
                primary: true,
                verified: true,
            },
        ];
        assert_eq!(pick_email(&emails), Some("me@example.com"));
    }

    #[test]
    fn pick_email_skips_unverified_even_when_primary() {
        // GitHub will list an unverified primary email under
        // `/user/emails`; per R3 that address must NOT be the one we
        // bind to a local user.
        let emails = vec![
            GitHubEmail {
                email: "primary-but-unverified@example.com".into(),
                primary: true,
                verified: false,
            },
            GitHubEmail {
                email: "secondary-verified@example.com".into(),
                primary: false,
                verified: true,
            },
        ];
        assert_eq!(pick_email(&emails), Some("secondary-verified@example.com"));
    }

    #[test]
    fn pick_email_returns_none_when_all_unverified() {
        let emails = vec![GitHubEmail {
            email: "x@y".into(),
            primary: true,
            verified: false,
        }];
        assert!(pick_email(&emails).is_none());
    }
}
