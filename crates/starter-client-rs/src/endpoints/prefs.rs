//! `/v1/{me,orgs,units}/…` client methods.
//!
//! Mirror of the four preference endpoints + the public unit
//! registry endpoint shipped by `starter-prefs`. Owned by SCOPE.md
//! "API surface".
//!
//! Auth: the `/v1/me` and `/v1/orgs/{id}` endpoints require an
//! authenticated [`Principal`] server-side; this client attaches
//! whatever bearer / cookie credentials were configured on the
//! [`Client`] (if any) to every call. Single-tenant deployments
//! reach `/v1/me/preferences` with no `org` parameter and the server
//! falls back to the `"@starter/default"` workspace sentinel per
//! R3.

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use starter_spi::preferences::{PreferencesPatch, ResolvedPreferences};

use crate::{client::Client, error::ClientError};

/// `GET /v1/units` payload — wire-mirror of `starter_prefs`'s
/// `UnitsDocument`. Kept here as a local DTO so this crate does not
/// have to depend on `starter-prefs` (which pulls in axum / sqlx
/// behind its feature flags).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UnitsResponse {
    /// Per-quantity definition. Order matches the server's stable
    /// `Quantity::ALL` traversal.
    pub quantities: Vec<UnitsQuantity>,
}

/// One entry of [`UnitsResponse::quantities`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct UnitsQuantity {
    /// Wire identifier (e.g. `"temperature"`).
    pub quantity: String,
    /// Canonical SI unit identifier.
    pub canonical: String,
    /// Every unit accepted on the wire for this quantity.
    pub allowed: Vec<String>,
}

impl Client {
    /// `GET /v1/me/preferences[?org=…]`.
    ///
    /// `org`: workspace to resolve against. `None` lets the server
    /// pick from `principal.extra["active_workspace"]`, falling back
    /// to `"@starter/default"`.
    pub async fn get_my_preferences(
        &self,
        org: Option<&str>,
    ) -> Result<ResolvedPreferences, ClientError> {
        let url = format!("{}/v1/me/preferences", self.base_url);
        let mut req = self.attach_auth(self.http.get(&url));
        if let Some(o) = org {
            req = req.query(&[("org", o)]);
        }
        decode_json(req).await
    }

    /// `PATCH /v1/me/preferences[?org=…]`.
    ///
    /// `null` in any field of `patch` reverts that field to inherit
    /// from the org → default chain per R3.
    pub async fn patch_my_preferences(
        &self,
        org: Option<&str>,
        patch: PreferencesPatch,
    ) -> Result<(), ClientError> {
        let url = format!("{}/v1/me/preferences", self.base_url);
        let mut req = self.attach_auth(self.http.patch(&url)).json(&patch);
        if let Some(o) = org {
            req = req.query(&[("org", o)]);
        }
        expect_no_content(req).await
    }

    /// Like [`Self::patch_my_preferences`], but sends a raw JSON
    /// value. `PreferencesPatch` cannot carry an explicit `null`
    /// field (serde collapses `null` and `missing` into `None`),
    /// so callers wanting to *revert to inherit* per R3 must reach
    /// for this entry point.
    pub async fn patch_my_preferences_raw(
        &self,
        org: Option<&str>,
        body: serde_json::Value,
    ) -> Result<(), ClientError> {
        let url = format!("{}/v1/me/preferences", self.base_url);
        let mut req = self.attach_auth(self.http.patch(&url)).json(&body);
        if let Some(o) = org {
            req = req.query(&[("org", o)]);
        }
        expect_no_content(req).await
    }

    /// `GET /v1/orgs/{id}/preferences`. Admin-only on the server.
    pub async fn get_org_preferences(
        &self,
        workspace_id: &str,
    ) -> Result<ResolvedPreferences, ClientError> {
        let url = format!("{}/v1/orgs/{workspace_id}/preferences", self.base_url);
        decode_json(self.attach_auth(self.http.get(&url))).await
    }

    /// `PATCH /v1/orgs/{id}/preferences`. Admin-only on the server.
    pub async fn patch_org_preferences(
        &self,
        workspace_id: &str,
        patch: PreferencesPatch,
    ) -> Result<(), ClientError> {
        let url = format!("{}/v1/orgs/{workspace_id}/preferences", self.base_url);
        let req = self.attach_auth(self.http.patch(&url)).json(&patch);
        expect_no_content(req).await
    }

    /// `GET /v1/units`. Public — no auth required. Returns the
    /// closed [`UnitsResponse`] registry.
    pub async fn get_units(&self) -> Result<UnitsResponse, ClientError> {
        let url = format!("{}/v1/units", self.base_url);
        decode_json(self.http.get(&url)).await
    }

    fn attach_auth(&self, mut req: RequestBuilder) -> RequestBuilder {
        if let Some(b) = &self.bearer {
            req = req.bearer_auth(b);
        }
        if let Some(c) = &self.cookie {
            req = req.header(reqwest::header::COOKIE, format!("starter_session={c}"));
        }
        req
    }
}

async fn decode_json<T: serde::de::DeserializeOwned>(
    req: RequestBuilder,
) -> Result<T, ClientError> {
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        let bytes = resp.bytes().await.unwrap_or_default();
        return Err(server_error(status.as_u16(), &bytes));
    }
    Ok(resp.json().await?)
}

async fn expect_no_content(req: RequestBuilder) -> Result<(), ClientError> {
    let resp = req.send().await?;
    let status = resp.status();
    if !status.is_success() {
        let bytes = resp.bytes().await.unwrap_or_default();
        return Err(server_error(status.as_u16(), &bytes));
    }
    Ok(())
}

fn server_error(status: u16, bytes: &[u8]) -> ClientError {
    let problem = serde_json::from_slice::<starter_spi::dto::Problem>(bytes).ok();
    let message = problem
        .as_ref()
        .map(|p| p.title.clone())
        .unwrap_or_else(|| format!("HTTP {status}"));
    ClientError::Server {
        status,
        message,
        problem,
    }
}
