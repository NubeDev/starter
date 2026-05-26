// Placeholder renderer used by PR1's generated PuckConfig.
//
// Scope §B2 ("Placeholder mode is **new work**") defers the real
// PlaceholderRender to its own PR. PR1's placeholder is the
// quick-and-dirty stringify path the user prompt explicitly calls
// out — render the variant name + JSON.stringify of props so the
// canvas works (drag, drop, see the node) without pretending to be
// the real renderer.

import { createElement, type ComponentType } from "react";

export function makePlaceholderRenderer(
  variant: string,
): ComponentType<Record<string, unknown>> {
  const Placeholder = (props: Record<string, unknown>) => {
    // Strip Puck-internal props (`id`, `editMode`, `puck`) before
    // stringifying so the placeholder displays the IR-shaped subset
    // an author actually edited.
    const { id: _id, editMode: _em, puck: _p, ...rest } = props as Record<
      string,
      unknown
    >;
    void _id;
    void _em;
    void _p;
    return createElement(
      "div",
      {
        "data-puck-placeholder": variant,
        style: {
          padding: "0.5rem 0.75rem",
          border: "1px dashed #94a3b8",
          borderRadius: "0.375rem",
          fontFamily: "ui-monospace, SFMono-Regular, monospace",
          fontSize: "0.75rem",
          background: "#f8fafc",
          color: "#0f172a",
          margin: "0.25rem 0",
        },
      },
      createElement(
        "div",
        { style: { fontWeight: 600, marginBottom: "0.25rem" } },
        variant,
      ),
      createElement(
        "pre",
        { style: { margin: 0, whiteSpace: "pre-wrap", wordBreak: "break-all" } },
        safeStringify(rest),
      ),
    );
  };
  Placeholder.displayName = `PuckPlaceholder(${variant})`;
  return Placeholder;
}

function safeStringify(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2) ?? "undefined";
  } catch {
    return "[unstringifiable]";
  }
}
