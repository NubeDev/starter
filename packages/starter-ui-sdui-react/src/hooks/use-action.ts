// `useSduiAction` — fires `transport.action()`. Returns the typed
// response; callers (form/button renderers) decide how to surface
// `toast` / `redirect` / `patch` / `diagnostics`.

import { useMutation } from "@tanstack/react-query";
import type { UseMutationResult } from "@tanstack/react-query";
import type { ActionRequest, UiActionResponse } from "@nube/starter-ui-ir";
import { useSduiTransport } from "../provider/sdui-provider.js";

export function useSduiAction(): UseMutationResult<
  UiActionResponse,
  Error,
  ActionRequest
> {
  const transport = useSduiTransport();
  return useMutation<UiActionResponse, Error, ActionRequest>({
    mutationFn: (req) => transport.action(req),
  });
}
