// `rubix.alert.send` client method.
//
// Dispatches through the generic tool route
// `POST /api/v1/tools/rubix.alert.send` exposed by rubix-agent. Wire
// shape mirrors the Rust DTO in
// `rubix-spi/src/dto/system/alert_send.rs`.

import { fetchJson } from "@nube/starter-client-ts";

import { RubixClient } from "../client/client.js";
import type { Diagnostic } from "./system.js";

/** Severity to attach to the emitted alert. Mirrors tracing levels. */
export type AlertSeverity = "info" | "warn" | "error";

export interface AlertSendRequest {
  severity: AlertSeverity;
  message: string;
}

export interface AlertSendResponse {
  summary: Diagnostic;
  severity: AlertSeverity;
  delivered_chars: number;
  probed_at_ms: number;
}

declare module "../client/client.js" {
  interface RubixClient {
    send(request: AlertSendRequest): Promise<AlertSendResponse>;
  }
}

RubixClient.prototype.send = function send(
  this: RubixClient,
  request: AlertSendRequest,
): Promise<AlertSendResponse> {
  return fetchJson<AlertSendResponse>(this.starter, `/api/v1/tools/rubix.alert.send`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(request),
  });
};
