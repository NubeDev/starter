import { useEffect, useRef, useState } from "react";
import { useStarterClient } from "@nube/starter-client-react";

import { previewInsight } from "@/api/insights/preview";
import type { PreviewInsightResponse, QueryResponse } from "@/api/types";

// The Workbench's live loop. Debounces script/sample/params changes (~400ms),
// then POSTs to `/insights/preview` with the sample rows. A *script* error
// comes back as HTTP 200 with `ok: false` — so we read it off the resolved
// response (the `error` arm), NOT a thrown error; a thrown error here means a
// transport failure, which we surface separately.
//
// The response is the untagged union keyed by `ok`; we narrow on the presence
// of `result` so the success/error arms are concrete to callers.
export type PreviewState =
  | { status: "idle" }
  | { status: "loading" }
  | {
      status: "ok";
      result: QueryResponse;
      rowCountIn: number;
    }
  | {
      status: "script-error";
      kind: string;
      message: string;
    }
  | { status: "transport-error"; message: string };

type SuccessArm = Extract<PreviewInsightResponse, { result: QueryResponse }>;
type ErrorArm = Exclude<PreviewInsightResponse, { result: QueryResponse }>;

function isSuccess(res: PreviewInsightResponse): res is SuccessArm {
  return res.ok === true && "result" in res;
}

export function usePreviewInsight({
  sample,
  script,
  params,
  enabled,
  debounceMs = 400,
}: {
  sample: QueryResponse | null;
  script: string;
  /** Already-parsed params object (caller parses + gates on JSON errors). */
  params: unknown;
  /** Gate preview off when there's no sample or params don't parse. */
  enabled: boolean;
  debounceMs?: number;
}): PreviewState {
  const client = useStarterClient();
  const [state, setState] = useState<PreviewState>({ status: "idle" });
  // Bumped on every change so a stale in-flight response can't overwrite a
  // newer one (no AbortController on fetchJson, so we guard by request id).
  const reqId = useRef(0);

  useEffect(() => {
    if (!enabled || !sample || !script.trim()) {
      setState({ status: "idle" });
      return;
    }
    const id = ++reqId.current;
    setState({ status: "loading" });
    const handle = setTimeout(() => {
      previewInsight(client, {
        script,
        rows: sample.rows,
        ...(params === undefined ? {} : { params }),
      })
        .then((res) => {
          if (id !== reqId.current) return;
          if (isSuccess(res)) {
            setState({
              status: "ok",
              result: res.result,
              rowCountIn: res.row_count_in,
            });
          } else {
            const err = (res as ErrorArm).error;
            setState({
              status: "script-error",
              kind: err.kind,
              message: err.message,
            });
          }
        })
        .catch((e: unknown) => {
          if (id !== reqId.current) return;
          setState({
            status: "transport-error",
            message: e instanceof Error ? e.message : "Preview request failed.",
          });
        });
    }, debounceMs);
    return () => clearTimeout(handle);
  }, [client, enabled, sample, script, params, debounceMs]);

  return state;
}
