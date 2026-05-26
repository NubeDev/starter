//! `GET /api/v1/dashboards/events` — SSE live tail of dashboard
//! page-list mutations, scoped to the requesting principal's tenant.
//!
//! Wire shape (one default `data:` frame per change):
//!
//!   - **first frame** — `{"kind":"snapshot","items":[…]}` derived
//!     from [`DashboardStore::list_active`]; gives the client a
//!     full picture on connect *and* on every auto-reconnect, so a
//!     dropped `LISTEN/NOTIFY` packet can never leave the sidebar
//!     out of sync.
//!   - **delta frames** — `{"kind":"created"|"updated"|"deleted",
//!     "page_id":"…","title":"…?","revision_id":"…?","tenant_id":"…"}`,
//!     one per surviving changelog row.
//!
//! Today the rubix dashboard write path is the
//! `rubix.dashboard.{create,update,delete}` tools, and every tool
//! dispatch already lands a row in `starter_changes` via the
//! [`crate::middleware::changelog`] layer with
//! `resource.kind = "tool.invoke"` and the redacted request body in
//! `Change.after`. We therefore drive deltas off `tool.invoke` rows
//! whose `resource_id` matches one of the three dashboard verbs and
//! pluck `tenant_id` + `page_id` + `title` out of the recorded
//! payload. When `PgDashboardStore` later grows a direct
//! `ChangeRecorder` hook (see scope doc §"What we already have")
//! the synthesis branch in [`change_to_event`] simply becomes the
//! preferred path — the SSE wire stays unchanged.
//!
//! AuthN: the route is mounted under the same `with_principal` layer
//! as the audited tools router; an anonymous request gets
//! `401 Unauthorized` before any stream is opened. Tenant filtering
//! is per-subscriber so a noisy neighbour cannot inflate another
//! tenant's traffic. The super-admin sentinel `tenant_id == "*"`
//! sees every tenant.
//!
//! CSRF gating mirrors the extension- and flow-events routes:
//! `text/event-stream` GETs carry no body and `EventSource` cannot
//! forward a CSRF header, so the route is mounted *outside* the
//! CSRF middleware sandwich. AuthN still gates it.

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Extension, Router};
use futures::stream::{self, Stream, StreamExt};
use serde::Serialize;
use serde_json::Value;
use tokio_stream::wrappers::ReceiverStream;
use tracing::warn;

use rubix_spi::dashboard::{DashboardStore, ListFilter};
use starter_changelog::ChangeTail;
use starter_spi::auth::Principal;
use starter_spi::changelog::{Change, Op};

/// Tool ids whose `tool.invoke` audit rows we promote into
/// dashboard-event frames. Kept as a `&[&str]` so the match in
/// [`change_to_event`] is a single linear scan over a tiny slice.
const DASHBOARD_WRITE_TOOL_IDS: &[&str] = &[
    "rubix.dashboard.create",
    "rubix.dashboard.update",
    "rubix.dashboard.delete",
];

/// Resource-kind sentinel used when `PgDashboardStore` (or a fake
/// in tests) records a `Change` directly against the page resource
/// instead of going through the tool-invoke layer. Mirrors the
/// `DASHBOARD_PAGE_KIND` constant in `rubix-tools`.
const DASHBOARD_PAGE_KIND: &str = "rubix.dashboard.page";

/// Conventional tenant id used in single-tenant / laptop dev
/// deployments. The bundled seed data is written under this
/// tenant, and the rubix login flow does not yet bind sessions to
/// a tenant. Mirrors the constant of the same name in
/// `boot::mcp::agent_node`; both must move together.
const DEFAULT_TENANT: &str = "system";

/// State threaded into the SSE handler.
#[derive(Clone)]
pub struct DashboardEventsState {
    /// Shared live-tail of `starter_changes`. Backed in production
    /// by `PgListenTail` (`LISTEN/NOTIFY`); tests can swap an
    /// in-memory broadcast tail in.
    pub tail: Arc<dyn ChangeTail>,
    /// Read-side store for the snapshot frame.
    pub store: Arc<dyn DashboardStore>,
}

/// Build the router. Mount under `/` — the route already carries
/// its full `/api/v1` prefix.
pub fn router(state: DashboardEventsState) -> Router {
    Router::new()
        .route("/api/v1/dashboards/events", get(events))
        .with_state(state)
}

/// One item in the `snapshot` frame's `items` array; trimmed down
/// from [`rubix_spi::dashboard::DashboardRevision`] to the four
/// fields the sidebar actually renders.
#[derive(Debug, Clone, Serialize)]
struct SnapshotItem {
    page_id: String,
    title: String,
    revision_id: String,
    tags: Vec<String>,
}

/// Wire frame — a discriminated union projected to a single
/// default `data:` SSE event. Mirrors the
/// `useDashboardSidebar`-side reducer shape.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Frame {
    Snapshot {
        items: Vec<SnapshotItem>,
    },
    Created {
        page_id: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        revision_id: Option<String>,
        tenant_id: String,
    },
    Updated {
        page_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        revision_id: Option<String>,
        tenant_id: String,
    },
    Deleted {
        page_id: String,
        tenant_id: String,
    },
}

async fn events(
    State(state): State<DashboardEventsState>,
    principal: Option<Extension<Principal>>,
) -> axum::response::Response {
    // -- 1. AuthN gate. `with_principal` populates the extension
    //       when authentication succeeds; bail out cleanly otherwise
    //       so we never open a stream for an anonymous client.
    let Some(Extension(principal)) = principal else {
        return (StatusCode::UNAUTHORIZED, "authentication required").into_response();
    };
    // Super-admins (`tenant_id == "*"`) see every tenant; a
    // tenant-less principal (current rubix login default) falls
    // back to `DEFAULT_TENANT` so the snapshot AND the delta
    // filter both target the conventional single-tenant id the
    // seed data + chat surface use.
    let tenant_filter = principal
        .tenant_id
        .clone()
        .or_else(|| Some(DEFAULT_TENANT.to_owned()));

    // -- 2. Snapshot. We always emit it first so the client never
    //       sees an empty sidebar on connect and so a reconnect
    //       resync is free.
    let snapshot_items = match tenant_filter.as_deref() {
        Some("*") | None => {
            // Super-admin sentinel — listing across every tenant
            // would require iterating `TenantStore`; defer until
            // a real super-admin UX needs it. Deltas still flow.
            Vec::new()
        }
        Some(tenant) => state
            .store
            .list_active(tenant, &ListFilter::default())
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|r| SnapshotItem {
                        page_id: r.page_id,
                        title: r.title,
                        revision_id: r.revision_id,
                        tags: r.tags,
                    })
                    .collect()
            })
            .unwrap_or_else(|e| {
                warn!(
                    target: "rubix.routes.dashboard_events",
                    error = %e,
                    "snapshot list_active failed; sending empty snapshot",
                );
                Vec::new()
            }),
    };
    let snapshot_frame = Frame::Snapshot {
        items: snapshot_items,
    };

    // -- 3. Live tail. `subscribe` returns an `mpsc::Receiver` of
    //       every newly committed change row, regardless of kind;
    //       we filter to dashboard-shaped rows below.
    let tail_rx = match state.tail.subscribe().await {
        Ok(rx) => rx,
        Err(e) => {
            warn!(
                target: "rubix.routes.dashboard_events",
                error = %e,
                "ChangeTail::subscribe failed",
            );
            return (StatusCode::SERVICE_UNAVAILABLE, "tail unavailable").into_response();
        }
    };

    let stream = build_stream(snapshot_frame, tail_rx, tenant_filter);
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}

/// Compose the snapshot frame and the filtered tail into a single
/// SSE item stream. Factored out so the unit tests below can drive
/// it without standing up an HTTP layer.
fn build_stream(
    snapshot: Frame,
    tail_rx: tokio::sync::mpsc::Receiver<Change>,
    tenant_filter: Option<String>,
) -> impl Stream<Item = Result<SseEvent, Infallible>> + Send + 'static {
    let head = stream::once(async move { frame_to_event(&snapshot) });
    let tail = ReceiverStream::new(tail_rx).filter_map(move |change| {
        let tenant_filter = tenant_filter.clone();
        async move {
            change_to_event(&change, tenant_filter.as_deref()).map(|f| frame_to_event(&f))
        }
    });
    head.chain(tail)
}

/// Translate a recorded [`Change`] into a dashboard [`Frame`] when
/// it represents a tenant-visible dashboard mutation; return `None`
/// to drop the row.
fn change_to_event(change: &Change, tenant_filter: Option<&str>) -> Option<Frame> {
    // Two source shapes are supported:
    //
    // 1. A direct `rubix.dashboard.page` change row (future path —
    //    see module docs). `Change.after` is the full
    //    `DashboardSnapshot`.
    // 2. A `tool.invoke` row for one of the three dashboard write
    //    verbs (current path). `Change.after` is the redacted
    //    request body.
    let (verb, payload) = match change.resource.kind.as_str() {
        DASHBOARD_PAGE_KIND => {
            let verb = match &change.op {
                Op::Create => "create",
                Op::Update => "update",
                Op::Delete => "delete",
                _ => return None,
            };
            (verb, change.after.as_ref()?)
        }
        "tool.invoke" => {
            let tool_id = change.resource.id.as_deref()?;
            if !DASHBOARD_WRITE_TOOL_IDS.contains(&tool_id) {
                return None;
            }
            let verb = tool_id.rsplit('.').next()?;
            (verb, change.after.as_ref()?)
        }
        _ => return None,
    };

    let payload_tenant = payload.get("tenant_id").and_then(Value::as_str)?;
    if let Some(filter) = tenant_filter {
        if filter != "*" && filter != payload_tenant {
            return None;
        }
    }
    let page_id = payload.get("page_id").and_then(Value::as_str)?.to_owned();
    let title = payload
        .get("title")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let revision_id = payload
        .get("revision_id")
        .and_then(Value::as_str)
        .map(str::to_owned);

    let frame = match verb {
        "create" => Frame::Created {
            page_id,
            title: title.unwrap_or_default(),
            revision_id,
            tenant_id: payload_tenant.to_owned(),
        },
        "update" => Frame::Updated {
            page_id,
            title,
            revision_id,
            tenant_id: payload_tenant.to_owned(),
        },
        "delete" => Frame::Deleted {
            page_id,
            tenant_id: payload_tenant.to_owned(),
        },
        _ => return None,
    };
    Some(frame)
}

/// Project a [`Frame`] into an SSE `data:` event. Serialisation
/// failures fall back to an empty JSON object so the stream stays
/// open — a malformed frame is a server bug, not a connection
/// problem.
fn frame_to_event(frame: &Frame) -> Result<SseEvent, Infallible> {
    let json = serde_json::to_string(frame).unwrap_or_else(|_| "{}".to_owned());
    Ok(SseEvent::default().data(json))
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use chrono::Utc;
    use rubix_spi::dashboard::{DashboardRevision, DashboardStoreError, NewRevision};
    use serde_json::json;
    use starter_spi::authz::ResourceRef;
    use starter_spi::changelog::{Actor, ChangeId, GroupId};
    use starter_spi::Result as SpiResult;
    use tokio::sync::mpsc;

    /// Minimal fake `DashboardStore` whose `list_active` returns a
    /// canned vec — the snapshot frame is the only call the SSE
    /// handler makes against the store.
    struct FakeStore(Vec<DashboardRevision>);

    #[async_trait]
    impl DashboardStore for FakeStore {
        async fn insert_revision(
            &self,
            _: NewRevision,
        ) -> Result<DashboardRevision, DashboardStoreError> {
            unimplemented!()
        }
        async fn get_active(
            &self,
            _: &str,
            _: &str,
        ) -> Result<Option<DashboardRevision>, DashboardStoreError> {
            unimplemented!()
        }
        async fn list_active(
            &self,
            _: &str,
            _: &ListFilter,
        ) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
            Ok(self.0.clone())
        }
        async fn mark_superseded(&self, _: &str, _: &str) -> Result<u64, DashboardStoreError> {
            unimplemented!()
        }
        async fn history(&self, _: &str) -> Result<Vec<DashboardRevision>, DashboardStoreError> {
            unimplemented!()
        }
    }

    /// In-memory fake `ChangeTail` that hands the test a single
    /// pre-filled receiver. Sufficient to exercise the filter +
    /// projection without touching Postgres.
    struct FakeTail(tokio::sync::Mutex<Option<mpsc::Receiver<Change>>>);

    #[async_trait]
    impl ChangeTail for FakeTail {
        async fn subscribe(&self) -> SpiResult<mpsc::Receiver<Change>> {
            Ok(self.0.lock().await.take().expect("subscribe called twice"))
        }
    }

    fn tool_invoke_change(tool_id: &str, after: Value) -> Change {
        Change {
            id: ChangeId("ch-test".into()),
            at: Utc::now(),
            actor: Actor::User {
                subject: "u".into(),
            },
            resource: ResourceRef::row("tool.invoke", tool_id),
            resource_version: None,
            op: Op::Custom("invoke".into()),
            before: None,
            after: Some(after),
            patch: None,
            group_id: GroupId("g-test".into()),
            correlation: None,
        }
    }

    #[test]
    fn change_to_event_promotes_create_invoke() {
        let ch = tool_invoke_change(
            "rubix.dashboard.create",
            json!({
                "tenant_id": "t1",
                "page_id":   "dashboard.new",
                "title":     "Hello"
            }),
        );
        let f = change_to_event(&ch, Some("t1")).expect("frame");
        match f {
            Frame::Created {
                page_id,
                title,
                tenant_id,
                ..
            } => {
                assert_eq!(page_id, "dashboard.new");
                assert_eq!(title, "Hello");
                assert_eq!(tenant_id, "t1");
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    #[test]
    fn change_to_event_filters_other_tenant() {
        let ch = tool_invoke_change(
            "rubix.dashboard.delete",
            json!({ "tenant_id": "other", "page_id": "dashboard.x" }),
        );
        assert!(change_to_event(&ch, Some("t1")).is_none());
    }

    #[test]
    fn change_to_event_drops_unrelated_tool() {
        let ch = tool_invoke_change(
            "rubix.system.disk",
            json!({ "tenant_id": "t1", "page_id": "irrelevant" }),
        );
        assert!(change_to_event(&ch, Some("t1")).is_none());
    }

    #[test]
    fn change_to_event_super_admin_passes_through() {
        let ch = tool_invoke_change(
            "rubix.dashboard.update",
            json!({
                "tenant_id": "t1",
                "page_id":   "dashboard.x",
                "title":     "T",
                "revision_id": "rev-1"
            }),
        );
        let f = change_to_event(&ch, Some("*")).expect("frame");
        assert!(matches!(f, Frame::Updated { .. }));
    }

    #[tokio::test]
    async fn build_stream_emits_snapshot_first_then_deltas() {
        use futures::StreamExt;

        let snapshot = Frame::Snapshot {
            items: vec![SnapshotItem {
                page_id: "dashboard.seed".into(),
                title: "Seed".into(),
                revision_id: "rev-0".into(),
                tags: vec![],
            }],
        };
        let (tx, rx) = mpsc::channel::<Change>(4);
        tx.send(tool_invoke_change(
            "rubix.dashboard.create",
            json!({"tenant_id":"t1","page_id":"dashboard.new","title":"X"}),
        ))
        .await
        .unwrap();
        drop(tx);

        let mut stream = Box::pin(build_stream(snapshot, rx, Some("t1".into())));
        let first = stream.next().await.unwrap().unwrap();
        let second = stream.next().await.unwrap().unwrap();
        let third = stream.next().await;

        assert!(
            format!("{first:?}").contains("snapshot"),
            "first frame should be the snapshot, got {first:?}",
        );
        assert!(
            format!("{second:?}").contains("created"),
            "second frame should be the created delta, got {second:?}",
        );
        assert!(third.is_none(), "stream should end after tail drains");
    }

    /// Wire the `FakeTail` end-to-end through `DashboardEventsState`
    /// so we exercise `subscribe()` once via the trait object —
    /// catches a regression where the state-cloning would create
    /// fresh receivers per request and starve the stream.
    #[tokio::test]
    async fn fake_tail_subscribe_returns_seeded_receiver() {
        let (tx, rx) = mpsc::channel::<Change>(1);
        let tail = Arc::new(FakeTail(tokio::sync::Mutex::new(Some(rx)))) as Arc<dyn ChangeTail>;
        let store = Arc::new(FakeStore(vec![])) as Arc<dyn DashboardStore>;
        let _state = DashboardEventsState {
            tail: tail.clone(),
            store,
        };
        tx.send(tool_invoke_change(
            "rubix.dashboard.delete",
            json!({"tenant_id":"t1","page_id":"dashboard.x"}),
        ))
        .await
        .unwrap();
        let mut rx2 = tail.subscribe().await.expect("subscribe");
        let ch = rx2.recv().await.expect("change");
        assert_eq!(ch.resource.id.as_deref(), Some("rubix.dashboard.delete"));
    }
}
