// Thin wrapper around `StarterClient` for the extension admin slice
// shipped by `starter-ext-server`. Same pattern as `NotesClient`: we
// compose, we do not declaration-merge.

import { StarterClient, StarterError } from "@nube/starter-client-ts";

export type LifecycleState =
  | "discovered"
  | "validated"
  | "starting"
  | "running"
  | "stopping"
  | "stopped"
  | "crashed"
  | "failed";

export type EnablementState = "enabled" | "disabled";
export type RuntimeKind = "builtin" | "wasm" | "process";

export interface ExtensionSummary {
  id: string;
  version: string | null;
  display_name: string | null;
  state: LifecycleState;
  runtime_kind: RuntimeKind | null;
  restart_count: number;
  capability_violations: number;
  enabled: EnablementState;
}

export interface ContributeTool {
  id: string;
  description?: string;
}
export interface ContributeRest {
  id: string;
  method: string;
  path: string;
}
export interface ContributeCli {
  id: string;
  name: string;
}
export interface ContributeUiExpose {
  id: string;
  slot?: string;
  module: string;
}

export interface ExtensionDetail {
  id: string;
  state: LifecycleState;
  enabled: EnablementState;
  manifest: {
    id: string;
    version: string;
    display_name: string;
    runtime: { kind: RuntimeKind };
    contributes?: {
      tools?: ContributeTool[];
      rest?: ContributeRest[];
      cli?: ContributeCli[];
      ui?: { exposes?: ContributeUiExpose[] };
    };
  } | null;
  failure: string | null;
  restart_count: number;
  capability_violations: number;
  events_cursor: number;
  workers: unknown[];
}

export interface ToggleResponse {
  id: string;
  enabled: EnablementState;
  state: LifecycleState;
}

export class ExtensionsClient {
  constructor(public readonly starter: StarterClient) {}

  async list(): Promise<ExtensionSummary[]> {
    const res = await this.starter.fetch(`${this.starter.baseUrl}/extensions`, {
      headers: this.starter.headers,
    });
    if (!res.ok) throw await StarterError.fromResponse(res);
    return (await res.json()) as ExtensionSummary[];
  }

  async get(id: string): Promise<ExtensionDetail> {
    const res = await this.starter.fetch(
      `${this.starter.baseUrl}/extensions/${encodeURIComponent(id)}`,
      { headers: this.starter.headers },
    );
    if (!res.ok) throw await StarterError.fromResponse(res);
    return (await res.json()) as ExtensionDetail;
  }

  async setEnabled(id: string, enabled: boolean): Promise<ToggleResponse> {
    const action = enabled ? "enable" : "disable";
    const res = await this.starter.fetch(
      `${this.starter.baseUrl}/extensions/${encodeURIComponent(id)}/${action}`,
      { method: "POST", headers: this.starter.headers },
    );
    if (!res.ok) throw await StarterError.fromResponse(res);
    return (await res.json()) as ToggleResponse;
  }
}
