/**
 * `useSubscriptions` — runs the subscription plan emitted by the
 * resolver. For every subject the host's transport (SSE, WebSocket,
 * polling) opens a stream; when a slot update lands, the consumer's
 * component is patched in-place inside the cached tree under
 * `treeQueryKey`.
 *
 * The transport itself is **host-provided** — this hook reads the
 * subscribe function from a `useSdui`-adjacent context the host
 * mounts. When no transport is registered (test, static render),
 * the hook is a no-op; the page renders the resolver's snapshot
 * and never live-updates.
 *
 * Per the SCOPE the transport surface is intentionally narrow:
 *   subscribe(subject) → unsubscribe
 *   onUpdate(subject, value) → writes via `mergeAt`
 *
 * The renderer does not own the transport, the protocol, or the
 * back-pressure strategy.
 */
import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { mergeAt } from "./applyPatch.js";
import type { SubscriptionPlan, UiComponentTree } from "./types.js";

/**
 * Transport injected by the host (Studio, the public SDUI app,
 * a test harness). The signature is intentionally generic — the
 * binding engine emits subscription **subjects** (opaque keys),
 * and the host decides how to fetch their values.
 */
export interface SubscriptionTransport {
  subscribe(
    subject: { key: string; target_node_id: string; slot: string; field?: string },
    onValue: (value: unknown) => void,
  ): () => void;
}

/**
 * Mount the subscription plan against the host transport. Each
 * subject's value updates patch the consumer components in the
 * cached tree.
 *
 * `transport` is optional; without it the hook is a no-op (the
 * page renders the resolver's snapshot and stays static).
 */
export function useSubscriptions(
  treeQueryKey: readonly unknown[],
  plan: SubscriptionPlan | undefined,
  transport?: SubscriptionTransport,
): void {
  const qc = useQueryClient();

  useEffect(() => {
    if (!plan || !transport) return;
    const unsubs: Array<() => void> = [];
    for (const subject of plan.subjects) {
      const unsub = transport.subscribe(subject, (value) => {
        qc.setQueryData<unknown>(treeQueryKey, (prev: unknown) => {
          const tree = readTree(prev);
          if (!tree) return prev;
          let next = tree;
          for (const c of subject.consumers) {
            const fields = buildPatchFields(subject, value);
            next = mergeAt(next, c.component_id, fields);
          }
          return writeTree(prev, next);
        });
      });
      unsubs.push(unsub);
    }
    return () => {
      for (const u of unsubs) u();
    };
    // `treeQueryKey` is a stable readonly tuple per resolve cycle.
  }, [qc, plan, transport, treeQueryKey]);
}

/**
 * Build the patch fields to merge into a consumer component when a
 * subscribed slot updates. The default mapping writes the value
 * under `value` — components that need a different field set their
 * own patch shape via a custom renderer.
 */
function buildPatchFields(
  subject: { slot: string; field?: string },
  value: unknown,
): Record<string, unknown> {
  if (subject.field) {
    return { [subject.field]: value };
  }
  return { value };
}

function readTree(cached: unknown): UiComponentTree | null {
  if (!cached || typeof cached !== "object") return null;
  if ("render" in cached) {
    const c = cached as { render?: UiComponentTree };
    return c.render ?? null;
  }
  if ("ir_version" in cached && "root" in cached) {
    return cached as UiComponentTree;
  }
  return null;
}

function writeTree(prev: unknown, tree: UiComponentTree): unknown {
  if (!prev || typeof prev !== "object") return prev;
  if ("render" in prev) {
    return { ...(prev as object), render: tree };
  }
  return tree;
}
