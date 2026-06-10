// Wire types for the extension admin API (`/api/v1/extensions`, WS-14).
// Hand-written until the routes land in openapi.json / codegen.

export type LifecycleState =
  | "validated"
  | "starting"
  | "running"
  | "stopping"
  | "stopped"
  | "failed"
  | "disabled";

export type RuntimeKind = "builtin" | "wasm" | "process";

export interface ContributesSummary {
  tools: number;
  cli: number;
  rest: number;
  grpc: number;
  workers: number;
  nodes: number;
  skills: number;
  ui?: unknown;
}

export interface ExtensionSummary {
  id: string;
  version?: string | null;
  display_name?: string | null;
  state: LifecycleState;
  runtime_kind?: RuntimeKind | null;
  restart_count: number;
  capability_violations: number;
  enabled: "enabled" | "disabled";
  restart_required: boolean;
  contributes?: ContributesSummary;
}

export interface EnablementResponse {
  id: string;
  enabled: "enabled" | "disabled";
  state: LifecycleState;
}

export type CleanupItemKind =
  | "warehouse_table"
  | "enablement_row"
  | "ui_cache"
  | "i18n_cache"
  | "skill"
  | "subscription";

export interface CleanupItem {
  kind: CleanupItemKind;
  label: string;
  bytes?: number | null;
}

export interface CleanupBundle {
  path: string;
  will_delete: boolean;
}

export interface CleanupPreview {
  id: string;
  items: CleanupItem[];
  total_bytes: number;
  bundle: CleanupBundle;
}

export interface PurgeResponse {
  id: string;
  code: "cleanup.succeeded";
  removed: CleanupItem[];
  bundle: CleanupBundle;
}

export interface UninstallResponse {
  id: string;
  code: "uninstall.succeeded" | "uninstall.not_found";
  pending_restart?: boolean;
}

export interface InstallResponse {
  id: string;
  code: "install.succeeded";
  pending_restart: boolean;
}
