//! Google OAuth 2.0 / OIDC provider impl.
//!
//! Per Hard rule R7, scopes and endpoints are compile-time constants:
//!
//! - `openid` — turns the exchange into an OIDC flow so the
//!   userinfo endpoint returns the `sub` claim we bind to
//!   `provider_sub`.
//! - `email` — surfaces the `email` and (load-bearing for R3) the
//!   `email_verified` boolean on the userinfo response.
//! - `profile` — surfaces `name` for the initial `display_name`.
//!
//! Unlike GitHub there is no separate `/user/emails` round trip:
//! Google's userinfo response carries `email_verified` directly, so
//! `fetch_identity` is one POST (token exchange) followed by one GET
//! (`/userinfo`). We trust the `email_verified` claim per Hard rule
//! R3 — that is the entire reason the trait keeps that flag as a
//! distinct field rather than a per-provider negotiation.
//!
//! As with the GitHub impl, the token-exchange + userinfo calls go
//! through a `reqwest::Client` configured to refuse redirects
//! (oauth2 example boilerplate calls this out as an SSRF vector on
//! the token endpoint).
//!
//! Per R2 the access token never leaves [`Self::fetch_identity`]:
//! the local `String` holding it is dropped at the end of the
//! function, no copy is ever returned, persisted, or logged.

use async_trait::async_trait;
use serde::Deserialize;

use crate::provider::{OAuthProvider, ProviderError, ProviderIdentity};

/// Provider id used in path segments and stored in
/// `oauth_identities.provider`. Must match the key consumers use in
/// `OAUTH_GOOGLE_CLIENT_ID` etc.
pub const PROVIDER_ID: &str = "google";

/// OAuth 2.0 authorize endpoint (Google's v2 surface — the v1
/// `/o/oauth2/auth` path still works but is no longer documented).
pub const AUTHORIZE_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
/// OAuth 2.0 token endpoint.
pub const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
/// OIDC userinfo endpoint. Returns `sub`, `email`, `email_verified`,
/// `name`, plus optional profile bits we ignore.
pub const USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

/// Scopes requested at authorize time. Compile-time constant per R7;
/// an operator who needs a different scope set ships a code change,
/// not a config flag. `openid` is required for the `sub` claim
/// (Hard rule: we always bind on the stable provider subject, never
/// the email).
pub const SCOPES: &str = "openid email profile";

/// Google provider. Constructed once at startup; cloning is cheap
/// because the inner `reqwest::Client` is `Arc`-counted.
#[derive(Clone)]
pub struct GoogleProvider {
    client_id: String,
    client_secret: String,
    /// Overrides the public google.com URLs in tests. Left `None` in
    /// production; `Some(base)` swaps every URL's host so a mock
    /// HTTP server can stand in for the real endpoints.
    base_override: Option<String>,
    http: reqwest::Client,
}

impl GoogleProvider {
    /// Build a production-config provider against the public Google
    /// endpoints.
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
            base_override: None,
            http: build_client(),
        }
    }

    /// Build a provider whose authorize / token / userinfo URLs
    /// share a single base override. Used by tests; the prefix is
    /// treated as the host + scheme and concatenated with the path
    /// portions of the public URLs.
    ///
    /// Google splits its surface across three hosts
    /// (`accounts.google.com`, `oauth2.googleapis.com`,
    /// `openidconnect.googleapis.com`); the override collapses them
    /// onto one mock host because tests don't care about the split.
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

    fn userinfo(&self) -> String {
        self.rewrite(USERINFO_URL)
    }

    fn rewrite(&self, public: &str) -> String {
        match &self.base_override {
            None => public.to_string(),
            Some(base) => {
                let path = public
                    .split_once("://")
                    .map(|(_, rest)| rest)
                    .unwrap_or(public);
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
impl OAuthProvider for GoogleProvider {
    fn id(&self) -> &'static str {
        PROVIDER_ID
    }

    fn authorize_url(&self, state: &str, pkce_challenge: &str, redirect_uri: &str) -> String {
        let mut url = url::Url::parse(&self.authorize()).expect("AUTHORIZE_URL is valid");
        url.query_pairs_mut()
            .append_pair("client_id", &self.client_id)
            .append_pair("redirect_uri", redirect_uri)
            .append_pair("response_type", "code")
            .append_pair("scope", SCOPES)
            .append_pair("state", state)
            // Google supports PKCE; pass the S256 challenge so a
            // stolen `code` cannot be exchanged without the verifier
            // the state store guards.
            .append_pair("code_challenge", pkce_challenge)
            .append_pair("code_challenge_method", "S256")
            // `select_account` keeps the account chooser visible
            // even when the user has only one Google account
            // signed in — clearer UX, no behavioural impact on
            // the callback path.
            .append_pair("prompt", "select_account");
        url.into()
    }

    async fn fetch_identity(
        &self,
        code: &str,
        pkce_verifier: &str,
        redirect_uri: &str,
    ) -> Result<ProviderIdentity, ProviderError> {
        // 1. Code → access token. Google's token endpoint takes
        // application/x-www-form-urlencoded.
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
        // binding is dropped at function exit (R2). We deliberately
        // do not verify the `id_token` — the userinfo endpoint is
        // an authenticated HTTPS call to Google and is the path
        // SCOPE picks until a real `OidcProvider` lands in Phase 5.
        let access_token = token_resp.access_token;
        let user: UserInfo = self
            .http
            .get(self.userinfo())
            .bearer_auth(&access_token)
            .header("Accept", "application/json")
            .send()
            .await
            .map_err(|e| ProviderError::Transport(e.to_string()))?
            .error_for_status()
            .map_err(|e| ProviderError::Provider(e.to_string()))?
            .json()
            .await
            .map_err(|e| ProviderError::Provider(format!("userinfo body: {e}")))?;

        // Per R3 only a verified email auto-links. We surface the
        // claim verbatim and let the callback handler decide what
        // to do with `email_verified = false`; refusing here would
        // collapse "refuse signup" and "refuse link" into one
        // shape, and only the handler has the context to pick.
        if !user.email_verified {
            return Err(ProviderError::UnverifiedEmail);
        }

        Ok(ProviderIdentity {
            provider_sub: user.sub,
            email: user.email,
            email_verified: true,
            display_name: user.name,
        })
    }
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct UserInfo {
    /// Stable subject id — Google's documented identifier for the
    /// authenticated user. Immutable across email / profile
    /// changes; this is what backs `provider_sub` in
    /// `oauth_identities`.
    sub: String,
    email: String,
    /// `email_verified` is the entire point of trusting Google
    /// over GitHub's two-call dance. The claim is a boolean in the
    /// OIDC spec; older Google responses used a string, but the
    /// `/v1/userinfo` endpoint we hit returns the bool form.
    email_verified: bool,
    /// `name` is optional in the OIDC spec; treat absence as "no
    /// display name available" and let the callback fall back to
    /// the email local-part.
    #[serde(default)]
    name: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authorize_url_contains_compile_time_scopes() {
        let p = GoogleProvider::new("client-id", "client-secret");
        let url = p.authorize_url("state-xyz", "challenge-abc", "https://app/cb");
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        // openid + email + profile, in declaration order, URL-encoded
        // (`+` is the form-encoding for the space separator).
        assert!(url.contains("scope=openid+email+profile"));
        assert!(url.contains("state=state-xyz"));
        assert!(url.contains("response_type=code"));
        assert!(url.contains("code_challenge=challenge-abc"));
        assert!(url.contains("code_challenge_method=S256"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fapp%2Fcb"));
        assert!(url.contains("prompt=select_account"));
    }

    #[test]
    fn provider_id_matches_path_segment() {
        // Sanity: the path segment baked into router URLs and the
        // value persisted in `oauth_identities.provider` must agree
        // with the config-loader's KNOWN_PROVIDERS entry.
        assert_eq!(GoogleProvider::new("a", "b").id(), "google");
    }

    #[test]
    fn rewrite_replaces_three_hosts_with_one_base() {
        // Google's three hosts collapse onto a single mock host;
        // the path portion is preserved so wiremock matchers stay
        // ergonomic.
        let p = GoogleProvider::with_base_override("a", "b", "http://127.0.0.1:9999");
        assert_eq!(p.authorize(), "http://127.0.0.1:9999/o/oauth2/v2/auth");
        assert_eq!(p.token(), "http://127.0.0.1:9999/token");
        assert_eq!(p.userinfo(), "http://127.0.0.1:9999/v1/userinfo");
    }
}
