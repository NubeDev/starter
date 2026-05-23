//! The built-in `/reports` resource: one read endpoint and one
//! read/write endpoint. Authz is enforced by `with_permission`
//! layers wrapping each route — `read` action on GET, `create`
//! on POST. The route itself is unaware of authz; the middleware
//! pulls the engine + principal out of request extensions.

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use starter_authz::with_permission;
use starter_spi::auth::Principal;
use starter_store_sqlite::Pool;
use uuid::Uuid;

#[derive(Clone)]
pub struct ReportsState {
    pub pool: Pool,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub id: String,
    pub owner: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Deserialize)]
pub struct NewReport {
    pub title: String,
    pub body: String,
}

pub fn router<S: Clone + Send + Sync + 'static>(state: ReportsState) -> Router<S> {
    // The `list` route is gated by `read` on the `reports` kind.
    let list = Router::new()
        .route("/reports", get(list_reports))
        .with_state(state.clone());
    let list = with_permission(list, "reports", "read");

    // The `create` route is gated by `create` on the `reports` kind.
    let create = Router::new()
        .route("/reports", post(create_report))
        .with_state(state);
    let create = with_permission(create, "reports", "create");

    Router::new().merge(list).merge(create)
}

async fn list_reports(
    State(state): State<ReportsState>,
) -> Result<Json<Vec<Report>>, (StatusCode, String)> {
    let rows = sqlx::query("SELECT id, owner, title, body FROM reports ORDER BY created_at ASC")
        .fetch_all(state.pool.sqlx())
        .await
        .map_err(internal)?;
    let out = rows
        .into_iter()
        .map(|r| Report {
            id: r.get(0),
            owner: r.get(1),
            title: r.get(2),
            body: r.get(3),
        })
        .collect();
    Ok(Json(out))
}

async fn create_report(
    State(state): State<ReportsState>,
    axum::Extension(principal): axum::Extension<Principal>,
    Json(input): Json<NewReport>,
) -> Result<(StatusCode, Json<Report>), (StatusCode, String)> {
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO reports (id, owner, title, body) VALUES (?1, ?2, ?3, ?4)")
        .bind(&id)
        .bind(&principal.subject)
        .bind(&input.title)
        .bind(&input.body)
        .execute(state.pool.sqlx())
        .await
        .map_err(internal)?;
    Ok((
        StatusCode::CREATED,
        Json(Report {
            id,
            owner: principal.subject.clone(),
            title: input.title,
            body: input.body,
        }),
    ))
}

fn internal<E: std::fmt::Display>(e: E) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
}
