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
  /** `true` once purged this run but still lingering in the sealed registry
   * until the next boot — its persisted state (kinds, enablement row, owned
   * tables) is already gone, so the row is dead/stale, not healthy. The UI
   * renders these distinctly and disables their lifecycle actions. */
  uninstalled?: boolean;
  contributes?: ContributesSummary;
}

// `GET /api/v1/extensions/{id}/process` — live stats for a process-flavour
// extension's current child. The server returns 404 `ext.process.not_running`
// for builtin/wasm/stopped (mapped to `null` by the client). Field shapes match
// Rust's serde defaults: `SystemTime` → { secs_since_epoch, nanos_since_epoch },
// `Duration` → { secs, nanos }.
export interface SystemTimeJson {
  secs_since_epoch: number;
  nanos_since_epoch: number;
}

export interface DurationJson {
  secs: number;
  nanos: number;
}

export interface ProcessStats {
  pid: number;
  started_at: SystemTimeJson;
  uptime: DurationJson;
  rss_bytes?: number | null;
  cpu_pct?: number | null;
  restarts: number;
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
