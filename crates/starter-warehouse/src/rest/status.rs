//! `GET /api/warehouse/status` — W11 envelope under `dimensions` +
//! W16 read-after-write bound under `ingest.async_insert_oldest_age_ms`.
//! HTTP 503 when any dictionary status='failed_refresh'.

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json};
use serde::Serialize;

use crate::dim_freshness::{DictFreshness, DimensionFreshness, Status};
use crate::nodes::runtime::WarehouseRuntime;

#[derive(Serialize)]
pub struct IngestStatus {
    pub async_insert_oldest_age_ms: i64,
    pub async_insert_backlog: i64,
}

#[derive(Serialize)]
pub struct WarehouseStatusBody {
    pub dimensions: DimensionFreshness,
    pub ingest: IngestStatus,
}

pub async fn warehouse_status(
    State(rt): State<Arc<WarehouseRuntime>>,
) -> impl IntoResponse {
    let dims = match rt.freshness().await {
        Ok(d) => d,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error": e.to_string()})),
            )
                .into_response();
        }
    };
    let ingest = query_ingest(&rt).await.unwrap_or(IngestStatus {
        async_insert_oldest_age_ms: 0,
        async_insert_backlog: 0,
    });

    let body = WarehouseStatusBody { dimensions: dims, ingest };
    let code = match body.dimensions.entities_dict.status {
        Status::FailedRefresh => StatusCode::SERVICE_UNAVAILABLE,
        _ => StatusCode::OK,
    };
    (code, Json(serde_json::to_value(&body).unwrap())).into_response()
}

async fn query_ingest(
    rt: &WarehouseRuntime,
) -> Result<IngestStatus, starter_store_clickhouse::ChClientError> {
    // W16: read-after-write bound = oldest pending part age. Query
    // `system.asynchronous_inserts`. The exact column shape varies
    // by CH version; we coerce defensively.
    #[derive(clickhouse::Row, serde::Deserialize)]
    struct IngestRow {
        total_bytes: u64,
        oldest_age_ms: i64,
    }
    let rows = rt
        .ch
        .inner()
        .query(
            "SELECT \
                toUInt64(sum(total_bytes)) AS total_bytes, \
                toInt64(max(dateDiff('millisecond', first_update, now64(3)))) AS oldest_age_ms \
             FROM system.asynchronous_inserts",
        )
        .fetch_all::<IngestRow>()
        .await
        .unwrap_or_default();
    let (b, age) = rows
        .into_iter()
        .next()
        .map(|r| (r.total_bytes as i64, r.oldest_age_ms))
        .unwrap_or((0, 0));
    Ok(IngestStatus {
        async_insert_oldest_age_ms: age,
        async_insert_backlog: b,
    })
}

// Silence the unused-import warning when neither warehouse path
// runs — `DictFreshness` is re-exported for downstream consumers
// who construct the envelope by hand.
#[allow(dead_code)]
fn _ensure_freshness_re_exported(_: &DictFreshness) {}
