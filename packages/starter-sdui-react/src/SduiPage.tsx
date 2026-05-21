/**
 * `SduiPage` — resolves a `ui.page` via `POST /api/v1/ui/resolve`,
 * checks the IR-version capability handshake, mounts an
 * `SduiProvider`, and hands the tree to the `Renderer`. The
 * resolve transport is **host-provided** (`SduiResolver` /
 * `SduiActionDispatcher` passed as props) — the renderer does not
 * own the HTTP client, the auth headers, or the retry policy.
 *
 * This is the "live page" entry; for pre-resolved trees (AI builder
 * preview, static fixtures, embed-in-blog), use `SduiRenderPage`.
 */
import { useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { Skeleton } from "@nube/starter-ui-kit";
import { SduiProvider, globalCustomRegistry } from "./context.js";
import { Renderer } from "./Renderer.js";
import { checkIrVersion } from "./capability.js";
import { useSubscriptions, type SubscriptionTransport } from "./useSubscriptions.js";
import type { UiActionResponse, UiResolveResponse } from "./types.js";
import { SduiDialogHost } from "./SduiDialogHost.js";

export type SduiResolver = (req: {
  page_ref: string;
  stack: string[];
  page_state: Record<string, unknown>;
  user_claims: Record<string, unknown>;
  target_node_id?: string;
  dry_run: boolean;
}) => Promise<UiResolveResponse>;

export type SduiActionDispatcher = (req: {
  handler: string;
  args: unknown;
  context: { stack: string[]; page_state: Record<string, unknown> };
}) => Promise<UiActionResponse>;

export interface SduiPageProps {
  pageRef: string;
  targetNodeId?: string;
  resolve: SduiResolver;
  dispatchAction: SduiActionDispatcher;
  transport?: SubscriptionTransport;
  userClaims?: Record<string, unknown>;
}

function formatError(e: unknown): string {
  if (!e) return "unknown error";
  if (e instanceof Error) return e.message;
  if (typeof e === "string") return e;
  try {
    return JSON.stringify(e);
  } catch {
    return String(e);
  }
}

function PageSkeleton() {
  return (
    <div className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-6 py-8">
      <div className="flex flex-col gap-3">
        <Skeleton className="h-8 w-1/3" />
        <Skeleton className="h-px w-full" />
      </div>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {Array.from({ length: 4 }).map((_, i) => (
          <Skeleton key={i} className="h-24 w-full" />
        ))}
      </div>
      <Skeleton className="h-64 w-full" />
    </div>
  );
}

export function SduiPage({
  pageRef,
  targetNodeId,
  resolve,
  dispatchAction: dispatch,
  transport,
  userClaims,
}: SduiPageProps) {
  const [pageState, setPageState] = useState<Record<string, unknown>>({});
  const claims = useMemo(() => userClaims ?? {}, [userClaims]);

  // NOTE: the host transport (resolve / dispatch / SSE) is
  // intentionally NOT in the queryKey. React-Query hashes the key
  // via JSON.stringify; stateful transport objects (e.g. an SSE
  // listener carrying a `_lastSeq` counter) would drift on every
  // event and trigger spurious re-resolves that lose optimistic
  // writes against `treeQueryKey`. The host remounts the subtree on
  // a transport swap, which already unmounts these queries.
  const queryKey = [
    "sdui-resolve",
    pageRef,
    targetNodeId,
    pageState,
    claims,
  ] as const;

  const { data, isLoading, isError, error } = useQuery<UiResolveResponse>({
    queryKey,
    queryFn: () =>
      resolve({
        page_ref: pageRef,
        stack: [],
        page_state: pageState,
        user_claims: claims,
        dry_run: false,
        ...(targetNodeId !== undefined && { target_node_id: targetNodeId }),
      }),
    staleTime: 0,
    enabled: !!pageRef,
  });

  const subscriptions =
    data && "render" in data ? data.subscriptions : undefined;
  useSubscriptions(queryKey, subscriptions, transport);

  const dispatchAction = useMemo(
    () =>
      async (handler: string, args?: unknown): Promise<UiActionResponse> => {
        try {
          return await dispatch({
            handler,
            args: args ?? null,
            context: { stack: [], page_state: pageState },
          });
        } catch (err) {
          return {
            type: "toast",
            intent: "danger",
            message: formatError(err),
          };
        }
      },
    [dispatch, pageState],
  );

  const mergePageState = useMemo(
    () => (patch: Record<string, unknown>) =>
      setPageState((prev) => ({ ...prev, ...patch })),
    [],
  );

  if (isLoading) return <PageSkeleton />;

  if (isError || !data) {
    return (
      <div className="p-6">
        <p className="text-sm text-destructive">
          Failed to resolve page: {formatError(error)}
        </p>
      </div>
    );
  }

  if ("errors" in data) {
    return (
      <div className="p-6">
        <p className="mb-2 text-sm font-semibold">Dry-run issues:</p>
        <ul className="flex flex-col gap-1">
          {data.errors.map((e, i) => (
            <li key={i} className="text-sm text-destructive">
              <code>{e.location}</code>: {e.message}
            </li>
          ))}
        </ul>
      </div>
    );
  }

  const mismatch = checkIrVersion(data.render);
  if (mismatch) {
    return (
      <div className="p-6">
        <p className="text-sm text-destructive">
          Capability mismatch: server emitted{" "}
          <code>ir_version={mismatch.received}</code>, client supports up to{" "}
          <code>{mismatch.supported}</code>. Upgrade the frontend to render this page.
        </p>
      </div>
    );
  }

  return (
    <SduiProvider
      dispatchAction={dispatchAction}
      customRegistry={globalCustomRegistry}
      pageState={pageState}
      setPageState={mergePageState}
      treeQueryKey={queryKey}
      writePlan={data.writes ?? []}
    >
      <Renderer node={data.render.root} />
      <SduiDialogHost />
    </SduiProvider>
  );
}
