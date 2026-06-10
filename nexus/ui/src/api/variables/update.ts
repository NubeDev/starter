import { fetchJson, readCsrfHeader } from "@nube/starter-client-ts";
import type { StarterClient } from "@nube/starter-client-ts";

import type { UpdateVariableRequest, VariableDetail } from "@/api/types";

// `PATCH /api/v1/variables/{id}` — partial update of a variable. The common
// case is a `current`-only patch when the user picks a new value in the bar;
// the variable editor patches the rest. Omitted fields are left untouched.
export function updateVariable(
  client: StarterClient,
  id: string,
  request: UpdateVariableRequest,
): Promise<VariableDetail> {
  return fetchJson<VariableDetail>(
    client,
    `${client.apiPrefix}/variables/${encodeURIComponent(id)}`,
    {
      method: "PATCH",
      headers: { "content-type": "application/json", ...readCsrfHeader() },
      body: JSON.stringify(request),
    },
  );
}
