import type { components } from "@/api/generated";

// Named wire types lifted from the codegen'd OpenAPI schema (F2 — the
// single source of truth; never hand-edit these shapes). Bindings and
// hooks import from here so call sites read in domain terms rather than
// `components["schemas"][...]`.
type S = components["schemas"];

export type MeResponse = S["MeResponse"];
export type Problem = S["Problem"];

export type DatasourceSummary = S["DatasourceSummary"];
export type DatasourceDetail = S["DatasourceDetail"];
export type DatasourceKind = S["DatasourceKind"];
export type CreateDatasourceRequest = S["CreateDatasourceRequest"];
export type UpdateDatasourceRequest = S["UpdateDatasourceRequest"];
export type TestDatasourceResponse = S["TestDatasourceResponse"];
export type TestConnectionRequest = S["TestConnectionRequest"];
export type DatasourceKindList = S["DatasourceKindList"];
export type DatasourceKindSummary = S["DatasourceKindSummary"];
export type DatasourceSchema = S["DatasourceSchema"];
export type SchemaTable = S["SchemaTable"];
export type SchemaColumn = S["SchemaColumn"];

export type QueryRequest = S["QueryRequest"];
export type QueryResponse = S["QueryResponse"];
// Federation + insight refs that ride on a QueryRequest (additive contract).
export type FederatedSourceRef = S["FederatedSourceRef"];
export type InsightRef = S["InsightRef"];
export type QueryKindList = S["QueryKindList"];
export type QueryKindSummary = S["QueryKindSummary"];
export type QueryKindDetail = S["QueryKindDetail"];
export type CreateQueryKindRequest = S["CreateQueryKindRequest"];
export type QueryStats = S["QueryStats"];
export type ColumnSchema = S["ColumnSchema"];
export type ResultColumnType = S["ResultColumnType"];
export type QueryTimeRange = S["QueryTimeRange"];
export type QueryVariable = S["QueryVariable"];

export type QueryHistoryEntry = S["QueryHistoryEntry"];
export type QueryHistoryList = S["QueryHistoryList"];
export type StarQueryRequest = S["StarQueryRequest"];

export type CreateStreamRequest = S["CreateStreamRequest"];
export type CreateStreamResponse = S["CreateStreamResponse"];
export type StreamEvent = S["StreamEvent"];

export type DashboardSummary = S["DashboardSummary"];
export type DashboardDetail = S["DashboardDetail"];
export type CreateDashboardRequest = S["CreateDashboardRequest"];
export type UpdateDashboardRequest = S["UpdateDashboardRequest"];
export type DashboardExport = S["DashboardExport"];
export type PanelExport = S["PanelExport"];
export type VariableExport = S["VariableExport"];
export type PanelDetail = S["PanelDetail"];
export type CreatePanelRequest = S["CreatePanelRequest"];
export type UpdatePanelRequest = S["UpdatePanelRequest"];

// Dashboard organisation (WS-05).
export type FolderSummary = S["FolderSummary"];
export type CreateFolderRequest = S["CreateFolderRequest"];
export type UpdateFolderRequest = S["UpdateFolderRequest"];

// Insights (Rhai transforms applied to query results).
export type InsightSummary = S["InsightSummary"];
export type CreateInsightRequest = S["CreateInsightRequest"];
export type UpdateInsightRequest = S["UpdateInsightRequest"];
// Insights Workbench: live preview (rows-in / rows-out) + the curated
// function surface that feeds the cheatsheet and editor autocomplete.
export type PreviewInsightRequest = S["PreviewInsightRequest"];
export type PreviewInsightResponse = S["PreviewInsightResponse"];
export type PreviewInsightError = S["PreviewInsightError"];
export type InsightFunctionCatalog = S["InsightFunctionCatalog"];
export type InsightFunctionDoc = S["InsightFunctionDoc"];

export type FlowSummary = S["FlowSummary"];
export type FlowDetail = S["FlowDetail"];
export type FlowMetrics = S["FlowMetrics"];
export type CreateFlowRequest = S["CreateFlowRequest"];
// Flow visual builder (WS-06): node palette + bounded dry-run.
export type NodeType = S["NodeType"];
export type NodeTypeList = S["NodeTypeList"];
export type NodeCategory = S["NodeCategory"];
export type DryRunRequest = S["DryRunRequest"];
export type DryRunResponse = S["DryRunResponse"];
export type UpdateFlowRequest = S["UpdateFlowRequest"];
// Flow portability (share/import): the self-contained, secret-redacted model.
export type FlowExport = S["FlowExport"];
// Flow debug & values (live per-node tap): the SSE event union and toggle DTOs.
export type FlowDebugEvent = S["FlowDebugEvent"];
export type FlowDebugStatus = S["FlowDebugStatus"];
export type FlowDebugEnableResponse = S["FlowDebugEnableResponse"];
export type FlowTableQueryRequest = S["FlowTableQueryRequest"];
export type NodeCounters = S["NodeCounters"];
export type NodeRole = S["NodeRole"];
export type LogLevel = S["LogLevel"];

export type AlertRuleDetail = S["AlertRuleDetail"];
export type CreateAlertRuleRequest = S["CreateAlertRuleRequest"];
export type UpdateAlertRuleRequest = S["UpdateAlertRuleRequest"];
export type ChannelDetail = S["ChannelDetail"];
export type CreateChannelRequest = S["CreateChannelRequest"];
export type SilenceDetail = S["SilenceDetail"];
export type CreateSilenceRequest = S["CreateSilenceRequest"];
export type AlertEvent = S["AlertEvent"];

export type Tag = S["Tag"];
export type TaggableKind = S["TaggableKind"];
export type SetTagsRequest = S["SetTagsRequest"];
export type TaggedEntity = S["TaggedEntity"];

// The caller's freeform per-user settings bag (`GET`/`PUT /api/v1/me/settings`).
// An opaque envelope the frontend owns: starred dashboards live under
// `starredDashboards`. `PUT` is a full replace, so callers read-modify-write.
export type UserSettings = S["UserSettings"];

// Nav tree (WS-13): a node mounts a dashboard page (with context) or a static
// route into the access-gated navigation tree.
export type NavNodeDetail = S["NavNodeDetail"];
export type NavTarget = S["NavTarget"];
export type NavContext = S["NavContext"];
export type StaticRoute = S["StaticRoute"];
export type CreateNavNodeRequest = S["CreateNavNodeRequest"];
export type UpdateNavNodeRequest = S["UpdateNavNodeRequest"];

export type VariableDetail = S["VariableDetail"];
export type VariableKind = S["VariableKind"];
export type CreateVariableRequest = S["CreateVariableRequest"];
export type UpdateVariableRequest = S["UpdateVariableRequest"];

// Audit + undo/redo (WS-12).
export type Change = S["Change"];
export type ChangePage = S["ChangePage"];
export type Actor = S["Actor"];
export type Op = S["Op"];
export type UndoResponse = S["UndoResponse"];
export type ForgetRequest = S["ForgetRequest"];
export type ForgetResponse = S["ForgetResponse"];

// AI agents + sessions (agent CRUD, chatbot sessions, SSE).
export type AgentSummary = S["AgentSummary"];
export type AgentDetail = S["AgentDetail"];
export type CreateAgentRequest = S["CreateAgentRequest"];
export type UpdateAgentRequest = S["UpdateAgentRequest"];
export type CreateSessionRequest = S["CreateSessionRequest"];
export type CreateSessionResponse = S["CreateSessionResponse"];
export type SessionDetail = S["SessionDetail"];

// AI assist (synchronous, task-typed: SQL gen, panel/dashboard suggest).
export type AssistRequest = S["AssistRequest"];
export type AssistResponse = S["AssistResponse"];
export type AssistTask = S["AssistTask"];
