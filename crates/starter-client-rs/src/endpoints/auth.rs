//! `/auth/*` client methods. Bodies map 1:1 to the server's
//! routes.

use serde::{Deserialize, Serialize};

use crate::{client::Client, error::ClientError};

/// Request body for `POST /auth/login`.
#[derive(Debug, Serialize)]
pub struct LoginRequest {
    /// User email.
    pub email: String,
    /// Plaintext password.
    pub password: String,
}

/// Response body for `GET /auth/me`.
#[derive(Debug, Deserialize)]
pub struct MeResponse {
    /// Stable subject identifier.
    pub subject: String,
    /// User's email.
    pub email: String,
    /// Lowercase role string ("reader" | "writer" | "admin").
    pub role: String,
}

impl Client {
    /// `POST /auth/login`. Returns `()` on success; the session
    /// cookie is set by the server and stored in the client's
    /// state for subsequent requests if the consumer enabled
    /// cookie jar support.
    pub async fn login(&self, _request: LoginRequest) -> Result<(), ClientError> {
        // TODO(ap): implement once cookie-jar support story is
        // settled (reqwest's `cookie_store` feature is the likely
        // path). Stub keeps the surface visible.
        Ok(())
    }

    /// `GET /auth/me`. Returns the current principal or `Unauthenticated`.
    pub async fn me(&self) -> Result<MeResponse, ClientError> {
        let url = format!("{}/auth/me", self.base_url);
        let body = self.http.get(&url).send().await?.json().await?;
        Ok(body)
    }
}
