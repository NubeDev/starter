// Stream builder: validate and run a full pipeline config.

import { postJson } from "./client";

/** Mirrors ArkFlow's StreamConfig shape (output is filled in by the server on run). */
export interface StreamConfig {
  input: { type: string; [k: string]: unknown };
  buffer?: { type: string; [k: string]: unknown };
  pipeline: { thread_num: number; processors: { type: string; [k: string]: unknown }[] };
  output: { type: string; [k: string]: unknown };
}

export interface ValidateResponse {
  ok: boolean;
  error: string | null;
}

export interface RunResponse {
  ok: boolean;
  error: string | null;
  row_count: number;
  rows: Record<string, unknown>[];
  cancelled: boolean;
}

export function validateStream(config: StreamConfig): Promise<ValidateResponse> {
  return postJson<ValidateResponse>("/api/streams/validate", config);
}

export function runStream(config: StreamConfig, timeoutMs = 3000): Promise<RunResponse> {
  return postJson<RunResponse>("/api/streams/run", { ...config, timeout_ms: timeoutMs });
}
