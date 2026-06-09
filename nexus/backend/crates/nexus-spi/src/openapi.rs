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
    CreateDatasourceRequest, DatasourceDetail, DatasourceKind, DatasourceSchema, DatasourceSummary,
    RedactedConnection, SchemaColumn, SchemaTable, TestDatasourceResponse, UpdateDatasourceRequest,
};
use crate::dto::alert::{
    AlertEvent, AlertRuleDetail, ChannelDetail, CreateAlertRuleRequest, CreateChannelRequest,
    CreateSilenceRequest, SilenceDetail, UpdateAlertRuleRequest,
};
use crate::dto::flow::{CreateFlowRequest, FlowDetail, FlowSummary, UpdateFlowRequest};
use crate::dto::me::MeResponse;
use crate::dto::panel::{CreatePanelRequest, PanelDetail, UpdatePanelRequest};
use crate::dto::query::{ColumnSchema, QueryRequest, QueryResponse, QueryStats, ResultColumnType};
use crate::dto::stream::{CreateStreamRequest, CreateStreamResponse, StreamEvent};
use crate::dto::tag::{SetTagsRequest, Tag, TaggableKind, TaggedEntity};
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
    DatasourceSchema,
    SchemaTable,
    SchemaColumn,
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
    FlowSummary,
    FlowDetail,
    CreateFlowRequest,
    UpdateFlowRequest,
    AlertRuleDetail,
    CreateAlertRuleRequest,
    UpdateAlertRuleRequest,
    AlertEvent,
    ChannelDetail,
    CreateChannelRequest,
    SilenceDetail,
    CreateSilenceRequest,
    Tag,
    SetTagsRequest,
    TaggableKind,
    TaggedEntity,
)))]
pub struct Schemas;
