//! The schema surface of the contract, as a `utoipa`-derived document.
//!
//! This registers every DTO component but no paths — paths live with their
//! handlers in `nexus-api`, which merges this document's `components` into the
//! full API doc. Keeping the schema list here lets the contract's *type* surface
//! be generated and diffed from `nexus-spi` alone, independent of any handler.

use utoipa::OpenApi;

use crate::dto::dashboard::{
    CreateDashboardRequest, DashboardDetail, DashboardSummary, UpdateDashboardRequest,
};
use crate::dto::datasource::{
    CreateDatasourceRequest, DatasourceDetail, DatasourceKind, DatasourceSummary,
    RedactedConnection, TestDatasourceResponse, UpdateDatasourceRequest,
};
use crate::dto::me::MeResponse;
use crate::dto::panel::{CreatePanelRequest, PanelDetail, UpdatePanelRequest};
use crate::dto::query::{ColumnSchema, QueryRequest, QueryResponse, QueryStats, ResultColumnType};
use crate::dto::stream::{CreateStreamRequest, CreateStreamResponse, StreamEvent};
use crate::Problem;

/// Aggregates every nexus DTO into a schema-only OpenAPI document.
#[derive(OpenApi)]
#[openapi(components(schemas(
    Problem,
    MeResponse,
    QueryRequest,
    QueryResponse,
    QueryStats,
    ColumnSchema,
    ResultColumnType,
    DatasourceSummary,
    DatasourceDetail,
    CreateDatasourceRequest,
    UpdateDatasourceRequest,
    TestDatasourceResponse,
    DatasourceKind,
    RedactedConnection,
    CreateStreamRequest,
    CreateStreamResponse,
    StreamEvent,
    DashboardSummary,
    DashboardDetail,
    CreateDashboardRequest,
    UpdateDashboardRequest,
    PanelDetail,
    CreatePanelRequest,
    UpdatePanelRequest,
)))]
pub struct Schemas;
