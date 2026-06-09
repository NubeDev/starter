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
export type DatasourceSchema = S["DatasourceSchema"];
export type SchemaTable = S["SchemaTable"];
export type SchemaColumn = S["SchemaColumn"];

export type QueryRequest = S["QueryRequest"];
export type QueryResponse = S["QueryResponse"];
export type QueryKindList = S["QueryKindList"];
export type QueryKindSummary = S["QueryKindSummary"];
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
export type PanelDetail = S["PanelDetail"];
export type CreatePanelRequest = S["CreatePanelRequest"];
export type UpdatePanelRequest = S["UpdatePanelRequest"];

export type FlowSummary = S["FlowSummary"];
export type FlowDetail = S["FlowDetail"];
export type CreateFlowRequest = S["CreateFlowRequest"];
export type UpdateFlowRequest = S["UpdateFlowRequest"];

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
