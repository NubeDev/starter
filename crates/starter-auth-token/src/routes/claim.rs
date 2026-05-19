//! `POST /auth/claim` handler.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{claim::claim_pending, store::ClaimStore, ClaimError, ClaimedToken};

/// Body for `POST /auth/claim`.
#[derive(Debug, Deserialize)]
pub struct ClaimRequest {
    /// The pending claim token the operator was handed on first
    /// boot.
    pub token: String,
}

/// Body for the `200 OK` response on successful claim. The
/// plaintext owner token appears here exactly once.
#[derive(Debug, Serialize)]
pub struct ClaimResponse {
    /// The new `Principal.subject`.
    pub claim_id: String,
    /// Plaintext owner token. Save — the server only keeps a digest.
    pub owner_token: String,
}

/// State the route needs. A boxed [`ClaimStore`] so the consumer can
/// wire whichever backend they built.
pub type ClaimState = Arc<dyn ClaimStore>;

/// Handler. Public so the consumer can mount it on a custom route.
pub async fn handler(State(store): State<ClaimState>, Json(req): Json<ClaimRequest>) -> Response {
    match claim_pending(&*store, &req.token).await {
        Ok(ClaimedToken {
            claim_id,
            plaintext,
        }) => Json(ClaimResponse {
            claim_id,
            owner_token: plaintext,
        })
        .into_response(),
        Err(ClaimError::NoPending) => (
            StatusCode::CONFLICT,
            "no pending claim token; reset to re-issue",
        )
            .into_response(),
        Err(ClaimError::AlreadyClaimed) => {
            (StatusCode::CONFLICT, "server already claimed").into_response()
        }
        Err(ClaimError::InvalidToken) => {
            (StatusCode::UNAUTHORIZED, "invalid claim token").into_response()
        }
        Err(ClaimError::Store(msg)) => {
            tracing::error!(error = %msg, "claim store error");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
