//! Process-flavour real `*Backend` impls — every capability handle
//! goes through [`crate::host_rpc::HostRpc::call_sync`] to reach the
//! supervisor's installed [`HostMethodHandler`].
//!
//! Earlier slices kept process-flavour callers on `Stub<Cap>`
//! backends that returned `Error::Capability("…not wired in v0.1
//! process flavour")`. With the host-side dispatch in place
//! (`RubixHostMethods`) and the outbound machinery here, the four
//! Row-5-adjacent backends now plumb through to real host bodies:
//!
//! - [`RealDashboardBackend`] — `dashboard.read` / `dashboard.write`
//! - [`RealAuthzBackend`]     — `authz.check`
//! - [`RealWarehouseReadBackend`] — `warehouse.query`
//! - [`RealEventBusBackend`]  — `event_bus.publish`
//!
//! The five other backends ([`SecretsBackend`], [`HttpOutBackend`],
//! [`FsBackend`], [`WallClockBackend`], [`TracingBackend`]) stay on
//! the `Stub<...>` shape from [`crate::process`] until their host
//! methods (`secrets.get`, `http.request`, `fs.read`,
//! `clock.now`, `tracing.event`) light up. Wiring those is the
//! same shape as the four below — one `match` arm in
//! `RubixHostMethods` and one `Real*Backend` here.
//!
//! ## Sync trait, async transport
//!
//! Each `Real*Backend` impls the SDK's *sync* `*Backend` trait —
//! extension code calls `ctx.dashboard().read(...)` without
//! `.await`. The async transport is hidden behind
//! `HostRpc::call_sync`, which uses `block_in_place` +
//! `block_on` to bridge.

use starter_ext_spi::authz::{AuthzCheckRequest, AuthzCheckResponse};
use starter_ext_spi::dashboard::{
    DashboardReadRequest, DashboardReadResponse, DashboardWriteRequest,
};
use starter_ext_spi::event_bus::EventBusPublishRequest;
use starter_ext_spi::fs_ext::{FsReadRequest, FsReadResponse};
use starter_ext_spi::http_out::HttpRequest;
use starter_ext_spi::secrets::{SecretsGetRequest, SecretsGetResponse};
use starter_ext_spi::tracing_ext::TracingEventRequest;
use starter_ext_spi::wall_clock::{WallClockNowRequest, WallClockNowResponse};
use starter_ext_spi::warehouse::{
    Row, TemplateSpec, WarehouseDeleteRequest, WarehouseDeleteResponse, WarehouseReadRequest,
    WarehouseReadResponse, WarehouseUpdateRequest, WarehouseUpdateResponse, WarehouseWriteRequest,
    WarehouseWriteResponse,
};
use starter_ext_spi::{Error, Result};

use starter_ext_spi::datasource::{
    DatasourceExecuteRequest, DatasourceExecuteResponse, DatasourceQueryRequest,
    DatasourceQueryResponse,
};

use crate::ctx::{
    AuthzBackend, DashboardBackend, DatasourceBackend, EventBusBackend, FsBackend, HttpOutBackend,
    SecretsBackend, TracingBackend, WallClockBackend, WarehouseReadBackend, WarehouseWriteBackend,
};
use crate::host_rpc::HostRpc;

/// `DashboardBackend` whose `read` / `write` hop to the host via
/// `dashboard.read` / `dashboard.write`.
#[derive(Debug, Clone)]
pub struct RealDashboardBackend {
    rpc: HostRpc,
}

impl RealDashboardBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl DashboardBackend for RealDashboardBackend {
    fn read(&self, page_id: &str) -> Result<serde_json::Value> {
        let req = DashboardReadRequest {
            page_id: page_id.to_owned(),
        };
        let params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding dashboard.read: {e}")))?;
        let raw = self.rpc.call_sync("dashboard.read", params)?;
        let res: DashboardReadResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding dashboard.read: {e}")))?;
        Ok(res.body)
    }

    fn write(&self, page_id: &str, body: serde_json::Value) -> Result<()> {
        let req = DashboardWriteRequest {
            page_id: page_id.to_owned(),
            body,
        };
        let params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding dashboard.write: {e}")))?;
        let _raw = self.rpc.call_sync("dashboard.write", params)?;
        // Response is currently an empty struct — we don't need its
        // value, but we *do* need to wait for it to confirm the
        // write committed.
        Ok(())
    }
}

/// `AuthzBackend` whose `check` hops to the host via `authz.check`.
#[derive(Debug, Clone)]
pub struct RealAuthzBackend {
    rpc: HostRpc,
}

impl RealAuthzBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl AuthzBackend for RealAuthzBackend {
    fn check(&self, action: &str, resource: &str) -> Result<bool> {
        let req = AuthzCheckRequest {
            action: action.to_owned(),
            resource: resource.to_owned(),
        };
        let params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding authz.check: {e}")))?;
        let raw = self.rpc.call_sync("authz.check", params)?;
        let res: AuthzCheckResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding authz.check: {e}")))?;
        Ok(res.allowed)
    }
}

/// `WarehouseReadBackend` whose `query` / `count` / `describe`
/// hop to the host via `warehouse.query`. `count` is implemented
/// as `query(...).len()` for now — a v2 host method
/// (`warehouse.count`) is the natural follow-up if the row count
/// alone is enough to skip the row payload.
#[derive(Debug, Clone)]
pub struct RealWarehouseReadBackend {
    rpc: HostRpc,
}

impl RealWarehouseReadBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }

    fn do_query(&self, template: &str, params: serde_json::Value) -> Result<Vec<Row>> {
        let req = WarehouseReadRequest {
            template: template.to_owned(),
            params,
        };
        let wire_params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding warehouse.query: {e}")))?;
        let raw = self.rpc.call_sync("warehouse.query", wire_params)?;
        let res: WarehouseReadResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding warehouse.query: {e}")))?;
        Ok(res.rows)
    }
}

impl WarehouseReadBackend for RealWarehouseReadBackend {
    fn query(&self, template: &str, params: serde_json::Value) -> Result<Vec<Row>> {
        self.do_query(template, params)
    }

    fn count(&self, template: &str, params: serde_json::Value) -> Result<u64> {
        // No `warehouse.count` host method in v1 — derive the count
        // from the result set so extensions that only need a tally
        // still work. v2 adds a dedicated count method so the host
        // can skip materialising rows it won't return.
        let rows = self.do_query(template, params)?;
        Ok(rows.len() as u64)
    }

    fn describe(&self, _template: &str) -> Result<Option<TemplateSpec>> {
        // No `warehouse.describe` host method in v1. Future slices
        // can add one; for now extensions that need spec
        // introspection at runtime should fall back to the
        // contributed `block.yaml` they already shipped.
        Err(Error::capability(
            "warehouse_read.describe: no host method in v0.1 process flavour",
        ))
    }
}

/// `WarehouseWriteBackend` whose `insert` hops to the host via
/// `warehouse.write`. Mirror of [`RealWarehouseReadBackend`].
#[derive(Debug, Clone)]
pub struct RealWarehouseWriteBackend {
    rpc: HostRpc,
}

impl RealWarehouseWriteBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl WarehouseWriteBackend for RealWarehouseWriteBackend {
    fn insert(&self, table: &str, rows: Vec<Row>) -> Result<u64> {
        let req = WarehouseWriteRequest {
            table: table.to_owned(),
            rows,
        };
        let wire_params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding warehouse.write: {e}")))?;
        let raw = self.rpc.call_sync("warehouse.write", wire_params)?;
        let res: WarehouseWriteResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding warehouse.write: {e}")))?;
        Ok(res.rows_inserted)
    }

    fn update(&self, table: &str, key_column: &str, rows: Vec<Row>) -> Result<u64> {
        let req = WarehouseUpdateRequest {
            table: table.to_owned(),
            key_column: key_column.to_owned(),
            rows,
        };
        let wire_params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding warehouse.update: {e}")))?;
        let raw = self.rpc.call_sync("warehouse.update", wire_params)?;
        let res: WarehouseUpdateResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding warehouse.update: {e}")))?;
        Ok(res.rows_affected)
    }

    fn delete(&self, table: &str, key_column: &str, keys: Vec<serde_json::Value>) -> Result<u64> {
        let req = WarehouseDeleteRequest {
            table: table.to_owned(),
            key_column: key_column.to_owned(),
            keys,
        };
        let wire_params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding warehouse.delete: {e}")))?;
        let raw = self.rpc.call_sync("warehouse.delete", wire_params)?;
        let res: WarehouseDeleteResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding warehouse.delete: {e}")))?;
        Ok(res.rows_affected)
    }
}

/// `DatasourceBackend` whose `query` / `execute` hop to the host via
/// `datasource.query` / `datasource.execute` (WS-17 Wave B).
#[derive(Debug, Clone)]
pub struct RealDatasourceBackend {
    rpc: HostRpc,
}

impl RealDatasourceBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl DatasourceBackend for RealDatasourceBackend {
    fn query(
        &self,
        datasource_id: &str,
        sql: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<Vec<Row>> {
        let req = DatasourceQueryRequest {
            datasource_id: datasource_id.to_owned(),
            sql: sql.to_owned(),
            params,
        };
        let wire_params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding datasource.query: {e}")))?;
        let raw = self.rpc.call_sync("datasource.query", wire_params)?;
        let res: DatasourceQueryResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding datasource.query: {e}")))?;
        Ok(res.rows)
    }

    fn execute(
        &self,
        datasource_id: &str,
        statement: &str,
        params: Vec<serde_json::Value>,
    ) -> Result<u64> {
        let req = DatasourceExecuteRequest {
            datasource_id: datasource_id.to_owned(),
            statement: statement.to_owned(),
            params,
        };
        let wire_params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding datasource.execute: {e}")))?;
        let raw = self.rpc.call_sync("datasource.execute", wire_params)?;
        let res: DatasourceExecuteResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding datasource.execute: {e}")))?;
        Ok(res.rows_affected)
    }
}

/// `EventBusBackend` whose `publish` hops to the host via
/// `event_bus.publish`.
#[derive(Debug, Clone)]
pub struct RealEventBusBackend {
    rpc: HostRpc,
}

impl RealEventBusBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl EventBusBackend for RealEventBusBackend {
    fn publish(&self, topic: &str, payload: serde_json::Value) -> Result<()> {
        let req = EventBusPublishRequest {
            topic: topic.to_owned(),
            payload,
        };
        let wire_params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding event_bus.publish: {e}")))?;
        let _raw = self.rpc.call_sync("event_bus.publish", wire_params)?;
        Ok(())
    }
}

/// `SecretsBackend` whose `get` hops to the host via `secrets.get`.
#[derive(Debug, Clone)]
pub struct RealSecretsBackend {
    rpc: HostRpc,
}

impl RealSecretsBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl SecretsBackend for RealSecretsBackend {
    fn get(&self, name: &str) -> Result<String> {
        let req = SecretsGetRequest {
            name: name.to_owned(),
        };
        let params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding secrets.get: {e}")))?;
        let raw = self.rpc.call_sync("secrets.get", params)?;
        let res: SecretsGetResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding secrets.get: {e}")))?;
        Ok(res.value)
    }
}

/// `WallClockBackend` whose `now_unix_ms` hops to the host via
/// `wall_clock.now_ms`.
#[derive(Debug, Clone)]
pub struct RealWallClockBackend {
    rpc: HostRpc,
}

impl RealWallClockBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl WallClockBackend for RealWallClockBackend {
    fn now_unix_ms(&self) -> Result<u64> {
        let params = serde_json::to_value(WallClockNowRequest::default())
            .map_err(|e| Error::transport(format!("encoding wall_clock.now_ms: {e}")))?;
        let raw = self.rpc.call_sync("wall_clock.now_ms", params)?;
        let res: WallClockNowResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding wall_clock.now_ms: {e}")))?;
        Ok(res.now_unix_ms)
    }
}

/// `TracingBackend` whose `event` hops to the host via
/// `tracing.event`. The SDK trait returns `()` so any wire error
/// is swallowed — losing a log line is preferable to losing a
/// handler frame. The host always succeeds at the wire level
/// (the backing log macro is infallible); transport errors only
/// fire when stdout is broken, by which point the extension is
/// about to crash anyway.
#[derive(Debug, Clone)]
pub struct RealTracingBackend {
    rpc: HostRpc,
}

impl RealTracingBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl TracingBackend for RealTracingBackend {
    fn event(&self, level: &str, msg: &str, fields: serde_json::Value) {
        let req = TracingEventRequest {
            level: level.to_owned(),
            msg: msg.to_owned(),
            fields,
        };
        let Ok(params) = serde_json::to_value(&req) else {
            return;
        };
        let _ = self.rpc.call_sync("tracing.event", params);
    }
}

/// `HttpOutBackend` whose `request` hops to the host via
/// `http.request`. The `req` JSON value is forwarded verbatim;
/// the host validates it against the SPI `HttpRequest` shape
/// and the manifest's `Capability::HttpOut { authorities }`
/// allowlist before issuing the call.
#[derive(Debug, Clone)]
pub struct RealHttpOutBackend {
    rpc: HostRpc,
}

impl RealHttpOutBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl HttpOutBackend for RealHttpOutBackend {
    fn request(&self, req: serde_json::Value) -> Result<serde_json::Value> {
        // Sanity-check the shape at the SDK boundary so an
        // extension author that hand-rolls a malformed JSON gets
        // a clear local error instead of a wire-side
        // `Error::Validation`. Don't *re-serialise* — pass the
        // original value through so unknown fields survive the
        // round-trip (forwards-compatible with v2 fields the
        // host might add).
        let _: HttpRequest = serde_json::from_value(req.clone()).map_err(|e| {
            Error::validation(format!(
                "http.request body does not match HttpRequest shape: {e}"
            ))
        })?;
        self.rpc.call_sync("http.request", req)
    }
}

/// `FsBackend` whose `read` hops to the host via `fs.read`. The
/// host returns base64 bytes; this backend decodes before
/// returning so extensions see the same `Vec<u8>` shape that
/// `std::fs::read` would yield.
#[derive(Debug, Clone)]
pub struct RealFsBackend {
    rpc: HostRpc,
}

impl RealFsBackend {
    /// Construct over the shared `HostRpc`.
    pub fn new(rpc: HostRpc) -> Self {
        Self { rpc }
    }
}

impl FsBackend for RealFsBackend {
    fn read(&self, path: &str) -> Result<Vec<u8>> {
        let req = FsReadRequest {
            path: path.to_owned(),
        };
        let params = serde_json::to_value(&req)
            .map_err(|e| Error::transport(format!("encoding fs.read: {e}")))?;
        let raw = self.rpc.call_sync("fs.read", params)?;
        let res: FsReadResponse = serde_json::from_value(raw)
            .map_err(|e| Error::transport(format!("decoding fs.read: {e}")))?;
        // Decode base64 → bytes. Failure means the host emitted
        // a malformed response — surface as transport.
        use base64::Engine;
        base64::engine::general_purpose::STANDARD
            .decode(res.bytes_b64.as_bytes())
            .map_err(|e| Error::transport(format!("fs.read body not valid base64: {e}")))
    }
}
