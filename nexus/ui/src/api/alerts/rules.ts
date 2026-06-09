import { fetchJson, fetchVoid, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { AlertRuleDetail, CreateAlertRuleRequest } from "@/api/types";

// Alert rules — a query-based threshold check (`query op threshold`, held
// `for_secs`, evaluated every `interval_secs`) that notifies its channels.
// `GET/POST /api/v1/alerts/rules`, `DELETE …/{id}`.
export function listAlertRules(
  client: StarterClient,
): Promise<AlertRuleDetail[]> {
  return fetchJson<AlertRuleDetail[]>(client, `${client.apiPrefix}/alerts/rules`);
}

export function createAlertRule(
  client: StarterClient,
  request: CreateAlertRuleRequest,
): Promise<AlertRuleDetail> {
  return fetchJson<AlertRuleDetail>(client, `${client.apiPrefix}/alerts/rules`, {
    method: "POST",
    headers: { "content-type": "application/json", ...readCsrfHeader() },
    body: JSON.stringify(request),
  });
}

export async function removeAlertRule(
  client: StarterClient,
  id: string,
): Promise<void> {
  await fetchVoid(
    client,
    `${client.apiPrefix}/alerts/rules/${encodeURIComponent(id)}`,
    { method: "DELETE", headers: readCsrfHeader() },
  );
}
