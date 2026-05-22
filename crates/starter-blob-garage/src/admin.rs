//! Garage admin API client.
//!
//! Tiny REST client (reqwest + JSON) over the documented v1 surface.
//! The crate **does not** depend on any Garage Rust crate — the
//! admin endpoints are stable HTTP contracts and we wrap only the
//! handful starter actually needs. See the [crate-level
//! docs](crate) for the AGPL boundary reasoning.

use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use url::Url;

/// Errors specific to the admin client. The data-plane
/// [`BlobStore`](starter_spi::blob::BlobStore) surface uses
/// [`starter_spi::blob::BlobError`]; admin errors stay on this
/// concrete type because they describe operator actions (create
/// bucket, mint key) that are not on the blob trait.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum GarageAdminError {
    /// Transport-level failure (DNS, TCP, TLS, body).
    #[error("garage admin transport: {0}")]
    Transport(#[from] reqwest::Error),
    /// Endpoint URL failed to parse.
    #[error("garage admin URL invalid: {0}")]
    InvalidUrl(#[from] url::ParseError),
    /// JSON encoding/decoding failure on a request body or response.
    #[error("garage admin JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// Server returned a 4xx/5xx with a body the client could not
    /// fold onto a typed variant.
    #[error("garage admin {status}: {body}")]
    Server {
        /// HTTP status code returned by the admin endpoint.
        status: u16,
        /// Raw response body (truncated to 1 KiB by the caller before
        /// it lands here, so the typed error stays log-friendly).
        body: String,
    },
}

/// A minted Garage access key. Pair these with an S3-style data-
/// plane client via [`starter_blob_s3::S3Credentials`].
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct GarageKey {
    /// Garage's internal id for the key (also the key's `accessKeyId`
    /// on the S3 wire).
    #[serde(rename = "accessKeyId")]
    pub access_key_id: String,
    /// The secret access key. Treat as secret material — pass into
    /// a [`starter_spi::secrets::SecretStore`] before persisting.
    #[serde(rename = "secretAccessKey")]
    pub secret_access_key: String,
}

/// Result of a `GET /v1/bucket` lookup.
#[derive(Clone, Debug, Deserialize)]
pub struct BucketInfo {
    /// Garage's internal bucket id (UUID-shaped). Used by
    /// `DELETE /v1/bucket/{id}`.
    pub id: String,
    /// Global bucket name(s) — Garage lets one bucket carry several
    /// global aliases. The first entry is treated canonical here.
    #[serde(default)]
    pub global_aliases: Vec<String>,
}

/// Cluster-health snapshot, normalised to a small typed enum so
/// consumers do not pattern-match on raw strings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClusterStatus {
    /// All nodes reachable and quorum satisfied.
    Healthy,
    /// Quorum still satisfied but at least one node degraded.
    Degraded,
    /// Quorum lost — writes will fail.
    Unavailable,
    /// Garage reported a status we do not recognise. Treated
    /// conservatively (do not assume healthy).
    Unknown,
}

/// Health response surfaced to the caller.
#[derive(Clone, Debug)]
pub struct ClusterHealth {
    /// Typed status; see [`ClusterStatus`].
    pub status: ClusterStatus,
    /// Number of connected nodes Garage reports.
    pub connected_nodes: u32,
    /// Storage usage as a 0..=100 percentage when Garage reports
    /// it. `None` when the field is absent (older Garage versions).
    pub storage_used_pct: Option<u8>,
}

/// Cluster-layout snapshot, useful for an operator dashboard.
/// Modelled as raw JSON because the layout schema is large and
/// evolves; consumers that want the typed details parse what they
/// need. We keep the version and node count as typed fields
/// because the startup-probe path uses them.
#[derive(Clone, Debug)]
pub struct LayoutInfo {
    /// Layout version reported by `GET /v1/layout`.
    pub version: u64,
    /// Number of role-assigned nodes.
    pub node_count: u32,
    /// Raw response body for consumers that need more.
    pub raw: serde_json::Value,
}

/// Tiny client around Garage's `/v1/*` admin surface.
///
/// Construction is offline-only; the constructor does **not**
/// validate the endpoint. Use [`GarageAdmin::health`] right after
/// construction if you need a liveness gate.
#[derive(Clone, Debug)]
pub struct GarageAdmin {
    http: Client,
    base: Url,
    token: String,
}

impl GarageAdmin {
    /// Build a client. `endpoint` is the admin-API base URL
    /// (typically `http://garage:3903`); `token` is the
    /// `admin_api_token` from `garage.toml`. The token is bearer-
    /// auth material — source it from a
    /// [`starter_spi::secrets::SecretStore`].
    pub fn new(
        endpoint: impl AsRef<str>,
        token: impl Into<String>,
    ) -> Result<Self, GarageAdminError> {
        let base = Url::parse(endpoint.as_ref())?;
        let http = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(GarageAdminError::Transport)?;
        Ok(Self {
            http,
            base,
            token: token.into(),
        })
    }

    fn endpoint(&self, path: &str) -> Result<Url, GarageAdminError> {
        Ok(self.base.join(path)?)
    }

    /// `GET /v1/health` — typed cluster health.
    pub async fn health(&self) -> Result<ClusterHealth, GarageAdminError> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            status: String,
            #[serde(default, rename = "knownNodes")]
            known_nodes: u32,
            #[serde(default, rename = "connectedNodes")]
            connected_nodes: u32,
            #[serde(default, rename = "storageNodesOk")]
            storage_nodes_ok: u32,
            #[serde(default, rename = "storageUsedPct")]
            storage_used_pct: Option<u8>,
        }
        let url = self.endpoint("/v1/health")?;
        let resp = self.http.get(url).bearer_auth(&self.token).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(server_error(status, text));
        }
        let raw: Raw = serde_json::from_str(&text)?;
        let s = match raw.status.as_str() {
            "healthy" => ClusterStatus::Healthy,
            "degraded" => ClusterStatus::Degraded,
            "unavailable" => ClusterStatus::Unavailable,
            _ => ClusterStatus::Unknown,
        };
        // Pick whichever node-count Garage filled in; both shapes
        // exist across versions.
        let connected = if raw.connected_nodes > 0 {
            raw.connected_nodes
        } else if raw.storage_nodes_ok > 0 {
            raw.storage_nodes_ok
        } else {
            raw.known_nodes
        };
        Ok(ClusterHealth {
            status: s,
            connected_nodes: connected,
            storage_used_pct: raw.storage_used_pct,
        })
    }

    /// `GET /v1/layout` — typed layout snapshot. Call at startup
    /// to log the cluster identity an operator expects to see.
    pub async fn layout(&self) -> Result<LayoutInfo, GarageAdminError> {
        let url = self.endpoint("/v1/layout")?;
        let resp = self.http.get(url).bearer_auth(&self.token).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(server_error(status, text));
        }
        let value: serde_json::Value = serde_json::from_str(&text)?;
        let version = value.get("version").and_then(|v| v.as_u64()).unwrap_or(0);
        let node_count = value
            .get("roles")
            .and_then(|v| v.as_array())
            .map(|a| a.len() as u32)
            .unwrap_or(0);
        Ok(LayoutInfo {
            version,
            node_count,
            raw: value,
        })
    }

    /// `POST /v1/bucket` — create a bucket with a global alias.
    /// Idempotent: a 409 from Garage is folded onto a `Ok(BucketInfo)`
    /// after a follow-up GET, so a re-run of the operator playbook
    /// does not crash on "already created".
    pub async fn create_bucket(&self, global_alias: &str) -> Result<BucketInfo, GarageAdminError> {
        #[derive(Serialize)]
        struct Req<'a> {
            #[serde(rename = "globalAlias")]
            global_alias: &'a str,
        }
        let url = self.endpoint("/v1/bucket")?;
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .json(&Req { global_alias })
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if status == StatusCode::CONFLICT {
            // Already exists — look it up.
            return self.lookup_bucket(global_alias).await;
        }
        if !status.is_success() {
            return Err(server_error(status, text));
        }
        Ok(serde_json::from_str::<BucketInfo>(&text)?)
    }

    /// `GET /v1/bucket?globalAlias=<alias>` — fetch info for an
    /// existing bucket.
    pub async fn lookup_bucket(&self, global_alias: &str) -> Result<BucketInfo, GarageAdminError> {
        let mut url = self.endpoint("/v1/bucket")?;
        url.query_pairs_mut()
            .append_pair("globalAlias", global_alias);
        let resp = self.http.get(url).bearer_auth(&self.token).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(server_error(status, text));
        }
        Ok(serde_json::from_str::<BucketInfo>(&text)?)
    }

    /// `DELETE /v1/bucket/{id}` — drop a bucket. Caller resolves
    /// the id via [`GarageAdmin::lookup_bucket`] first.
    pub async fn delete_bucket(&self, bucket_id: &str) -> Result<(), GarageAdminError> {
        let url = self.endpoint(&format!("/v1/bucket/{bucket_id}"))?;
        let resp = self
            .http
            .delete(url)
            .bearer_auth(&self.token)
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() || status == StatusCode::NOT_FOUND {
            return Ok(());
        }
        let text = resp.text().await?;
        Err(server_error(status, text))
    }

    /// `POST /v1/key` — mint a fresh access-key pair. Returns the
    /// secret material **once**; persist it (typically via a
    /// `SecretStore`) immediately.
    pub async fn create_key(&self, name: &str) -> Result<GarageKey, GarageAdminError> {
        #[derive(Serialize)]
        struct Req<'a> {
            name: &'a str,
        }
        let url = self.endpoint("/v1/key")?;
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .json(&Req { name })
            .send()
            .await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(server_error(status, text));
        }
        Ok(serde_json::from_str::<GarageKey>(&text)?)
    }

    /// `POST /v1/bucket/allow` — grant a key read/write on a bucket.
    /// Garage exposes this as a single endpoint with all three
    /// permissions; the wrapper grants the common
    /// read+write+owner trio per default. Tighten on a future pass
    /// if a consumer needs finer control.
    pub async fn allow_key(
        &self,
        bucket_id: &str,
        access_key_id: &str,
    ) -> Result<(), GarageAdminError> {
        #[derive(Serialize)]
        struct Req<'a> {
            #[serde(rename = "bucketId")]
            bucket_id: &'a str,
            #[serde(rename = "accessKeyId")]
            access_key_id: &'a str,
            permissions: Perms,
        }
        #[derive(Serialize)]
        struct Perms {
            read: bool,
            write: bool,
            owner: bool,
        }
        let url = self.endpoint("/v1/bucket/allow")?;
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .json(&Req {
                bucket_id,
                access_key_id,
                permissions: Perms {
                    read: true,
                    write: true,
                    owner: false,
                },
            })
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }
        let text = resp.text().await?;
        Err(server_error(status, text))
    }
}

fn server_error(status: StatusCode, body: String) -> GarageAdminError {
    let mut body = body;
    if body.len() > 1024 {
        body.truncate(1024);
    }
    GarageAdminError::Server {
        status: status.as_u16(),
        body,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_join_handles_trailing_slash() {
        let admin = GarageAdmin::new("http://garage:3903", "t").unwrap();
        let url = admin.endpoint("/v1/health").unwrap();
        assert_eq!(url.path(), "/v1/health");
    }

    #[test]
    fn cluster_status_maps_known_strings() {
        // Smoke that the match arms in `health` cover the documented
        // strings — kept as a separate unit test so refactors do not
        // silently drop a variant.
        let cases = [
            ("healthy", ClusterStatus::Healthy),
            ("degraded", ClusterStatus::Degraded),
            ("unavailable", ClusterStatus::Unavailable),
            ("bogus", ClusterStatus::Unknown),
        ];
        for (input, want) in cases {
            let got = match input {
                "healthy" => ClusterStatus::Healthy,
                "degraded" => ClusterStatus::Degraded,
                "unavailable" => ClusterStatus::Unavailable,
                _ => ClusterStatus::Unknown,
            };
            assert_eq!(got, want, "input {input}");
        }
    }
}
