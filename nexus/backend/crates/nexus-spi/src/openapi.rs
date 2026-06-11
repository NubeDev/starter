//! The schema surface of the contract, as a `utoipa`-derived document.
//!
//! This registers every DTO component but no paths — paths live with their
//! handlers in `nexus-api`, which merges this document's `components` into the
//! full API doc. Keeping the schema list here lets the contract's *type* surface
//! be generated and diffed from `nexus-spi` alone, independent of any handler.

use utoipa::OpenApi;

use crate::dto::agent::{
    AgentDetail, AgentSummary, CreateAgentRequest, CreateSessionRequest, CreateSessionResponse,
    SessionDetail, UpdateAgentRequest,
};
use crate::dto::ai::{AssistRequest, AssistResponse, AssistTask};
use crate::dto::audit::{ForgetRequest, ForgetResponse, UndoResponse};
use crate::dto::dashboard::{
    CreateDashboardRequest, DashboardDetail, DashboardExport, DashboardSummary, PanelExport,
    UpdateDashboardRequest, VariableExport,
};
use crate::dto::folder::{CreateFolderRequest, FolderSummary, UpdateFolderRequest};
use crate::dto::insight::{
    CreateInsightRequest, InsightFunctionCatalog, InsightFunctionDoc, InsightRef, InsightSummary,
    PreviewInsightError, PreviewInsightRequest, PreviewInsightResponse, UpdateInsightRequest,
};
use crate::dto::nav::{
    CreateNavNodeRequest, NavContext, NavNodeDetail, NavTarget, StaticRoute, UpdateNavNodeRequest,
};
use crate::dto::datasource::{
    CreateDatasourceRequest, DatasourceDetail, DatasourceKind, DatasourceKindList,
    DatasourceKindSummary, DatasourceSchema, DatasourceSummary, RedactedConnection, SchemaColumn,
    SchemaTable, TestConnectionRequest, TestDatasourceResponse, UpdateDatasourceRequest,
};
use crate::dto::detection::{
    ChannelDetail, CreateChannelRequest, CreateDetectionRequest, CreateSilenceRequest,
    DetectionDetail, DetectionStats, Finding, FindingActionRequest, NotifyEvent, SilenceDetail,
    UpdateDetectionRequest,
};
use crate::dto::flow::{
    CreateFlowRequest, DryRunRequest, DryRunResponse, FlowDebugEnableResponse, FlowDebugEvent,
    FlowDebugStatus, FlowDetail, FlowExport, FlowMetrics, FlowSummary, LogLevel, NodeCategory,
    NodeCounters, NodeRole, NodeType, NodeTypeList, UpdateFlowRequest,
};
use crate::dto::ingest::IngestAccepted;
use crate::dto::me::{MeResponse, UserSettings};
use crate::dto::query_history::{QueryHistoryEntry, QueryHistoryList, StarQueryRequest};
use crate::dto::query_kind::{CreateQueryKindRequest, QueryKindDetail, UpdateQueryKindRequest};
use crate::dto::panel::{CreatePanelRequest, PanelDetail, UpdatePanelRequest};
use crate::dto::query::{
    ColumnSchema, FederatedSourceRef, QueryKindList, QueryKindSummary, QueryRequest, QueryResponse,
    QueryStats, QueryTimeRange, QueryVariable, ResultColumnType,
};
use crate::dto::stream::{CreateStreamRequest, CreateStreamResponse, StreamEvent};
use crate::dto::tag::{SetTagsRequest, Tag, TaggableKind, TaggedEntity};
use crate::dto::variable::{
    CreateVariableRequest, UpdateVariableRequest, VariableDetail, VariableKind,
};
use crate::Problem;

/// Aggregates every nexus DTO into a schema-only OpenAPI document.
#[derive(OpenApi)]
#[openapi(components(schemas(
    Problem,
    MeResponse,
    UserSettings,
    QueryRequest,
    QueryTimeRange,
    QueryVariable,
    FederatedSourceRef,
    QueryResponse,
    QueryStats,
    QueryKindList,
    QueryKindSummary,
    CreateQueryKindRequest,
    UpdateQueryKindRequest,
    QueryKindDetail,
    ColumnSchema,
    ResultColumnType,
    QueryHistoryEntry,
    QueryHistoryList,
    StarQueryRequest,
    DatasourceSummary,
    DatasourceDetail,
    CreateDatasourceRequest,
    UpdateDatasourceRequest,
    TestDatasourceResponse,
    TestConnectionRequest,
    DatasourceKind,
    DatasourceKindList,
    DatasourceKindSummary,
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
    DashboardExport,
    PanelExport,
    VariableExport,
    FolderSummary,
    CreateFolderRequest,
    UpdateFolderRequest,
    InsightSummary,
    CreateInsightRequest,
    UpdateInsightRequest,
    InsightRef,
    PreviewInsightRequest,
    PreviewInsightResponse,
    PreviewInsightError,
    InsightFunctionCatalog,
    InsightFunctionDoc,
    NavNodeDetail,
    NavTarget,
    StaticRoute,
    NavContext,
    CreateNavNodeRequest,
    UpdateNavNodeRequest,
    PanelDetail,
    CreatePanelRequest,
    UpdatePanelRequest,
    FlowSummary,
    FlowDetail,
    FlowMetrics,
    FlowDebugEvent,
    FlowDebugStatus,
    FlowDebugEnableResponse,
    NodeCounters,
    NodeRole,
    LogLevel,
    CreateFlowRequest,
    UpdateFlowRequest,
    FlowExport,
    NodeType,
    NodeTypeList,
    NodeCategory,
    IngestAccepted,
    DryRunRequest,
    DryRunResponse,
    CreateDetectionRequest,
    UpdateDetectionRequest,
    DetectionDetail,
    DetectionStats,
    Finding,
    FindingActionRequest,
    ChannelDetail,
    CreateChannelRequest,
    SilenceDetail,
    CreateSilenceRequest,
    NotifyEvent,
    Tag,
    SetTagsRequest,
    TaggableKind,
    TaggedEntity,
    AgentSummary,
    AgentDetail,
    CreateAgentRequest,
    UpdateAgentRequest,
    CreateSessionRequest,
    CreateSessionResponse,
    SessionDetail,
    AssistRequest,
    AssistResponse,
    AssistTask,
    VariableDetail,
    VariableKind,
    CreateVariableRequest,
    UpdateVariableRequest,
    UndoResponse,
    ForgetRequest,
    ForgetResponse,
)))]
pub struct Schemas;
