/**
 * `button` / `link` — the two interactive leaf primitives.
 *
 * `button` carries a `label`, an optional shadcn variant, and an
 * `on_click` action handler. The dispatch goes through `useSdui`'s
 * action function so the host wires it to `POST /api/v1/ui/action`
 * (one endpoint, one round-trip — **R5**).
 *
 * `link` is the non-action navigation primitive: opens an `href`
 * (optionally in a new tab). Internal vs external routing is the
 * host's responsibility — the renderer just emits an `<a>`.
 */
import { useState } from "react";
import { Button } from "@nube/starter-ui-kit";
import type { ComponentSpec } from "../registry/types.js";
import { useOptimisticAction } from "../useOptimisticAction.js";
import type { OptimisticHint, UiComponent } from "../types.js";

type BtnVariant =
  | "default"
  | "secondary"
  | "destructive"
  | "outline"
  | "ghost"
  | "link";

export interface ButtonNode extends UiComponent {
  type: "button";
  label: string;
  variant?: BtnVariant;
  disabled?: boolean;
  /** Action handler name — sent to `POST /api/v1/ui/action`. */
  on_click?: string;
  /** Optional action arguments. */
  args?: unknown;
  /** Optional confirm prompt rendered as `window.confirm` for now —
   *  the AlertDialog primitive lands with the dialog component in a
   *  follow-up batch. */
  confirm?: string;
  /**
   * Optional optimistic-update hint. Applied to the cached tree
   * (via React-Query `setQueryData` + `mergeAt`) **before** the
   * round-trip fires. The server's authoritative reply replaces
   * through the same helpers; a thrown dispatch error rolls back
   * to the pre-dispatch snapshot. See SCOPE.md § R9.
   */
  optimistic?: OptimisticHint;
}

export const buttonSpec: ComponentSpec<ButtonNode> = {
  kind: "button",
  Component: ({ node }) => {
    const dispatch = useOptimisticAction();
    const [pending, setPending] = useState(false);

    const onClick = async () => {
      if (!node.on_click) return;
      if (node.confirm && !window.confirm(node.confirm)) return;
      setPending(true);
      try {
        await dispatch(node.on_click, node.args, node.optimistic ?? null);
      } finally {
        setPending(false);
      }
    };

    return (
      <Button
        variant={node.variant ?? "default"}
        disabled={!!node.disabled || pending}
        onClick={node.on_click ? onClick : undefined}
        className={node.style?.className}
      >
        {node.label}
      </Button>
    );
  },
};

export interface LinkNode extends UiComponent {
  type: "link";
  label: string;
  href: string;
  external?: boolean;
}

export const linkSpec: ComponentSpec<LinkNode> = {
  kind: "link",
  Component: ({ node }) => (
    <a
      href={node.href}
      target={node.external ? "_blank" : undefined}
      rel={node.external ? "noreferrer noopener" : undefined}
      className={`text-sm font-medium text-primary underline-offset-4 hover:underline ${
        node.style?.className ?? ""
      }`}
    >
      {node.label}
    </a>
  ),
};
