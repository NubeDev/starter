// `button` — action button or, when `href` is set, a call-to-action
// link. Maps the IR's page-builder tokens (`variant`/`size`/`intent`)
// onto the UI-kit Button. Action wiring uses the shared action hook so a
// CTA and a form button share one code path.
import { Button, cn } from "@nube/starter-ui-kit";
import { registerRenderer } from "../headless/registry.js";
import { useSduiAction } from "../headless/hooks/use-action.js";
import { nodeStyleAttrs } from "./node-style.js";

// IR variant token → kit Button variant. `solid` is the IR default and
// maps to the kit's `default`; `destructive` is reachable via intent.
const VARIANT_MAP: Record<string, "default" | "outline" | "ghost" | "link"> = {
  solid: "default",
  outline: "outline",
  ghost: "ghost",
  link: "link",
};

const SIZE_MAP: Record<string, "sm" | "default" | "lg"> = {
  sm: "sm",
  md: "default",
  lg: "lg",
};

export function RenderButton({ node }: { node: import("@nube/starter-ui-ir").UiComponent }) {
  const label = typeof node.label === "string" ? node.label : "";
  const href = typeof node.href === "string" ? node.href : undefined;
  const disabled = node.disabled === true;
  const intent = typeof node.intent === "string" ? node.intent : undefined;
  let variant = VARIANT_MAP[node.variant as string] ?? "default";
  if (intent === "danger") variant = "outline"; // accent handled via style attr
  const size = SIZE_MAP[node.size as string] ?? "default";

  const action = useSduiAction();
  const styleAttrs = nodeStyleAttrs(node.style);
  const className = cn("sdui-button", node.style?.className);

  // href wins over action — a CTA navigates rather than firing a handler.
  if (href) {
    return (
      <Button asChild variant={variant} size={size} className={className} {...styleAttrs}>
        <a href={href} aria-disabled={disabled || undefined}>
          {label}
        </a>
      </Button>
    );
  }

  return (
    <Button
      variant={variant}
      size={size}
      disabled={disabled || action.isPending}
      className={className}
      {...styleAttrs}
      onClick={() => {
        const a = node.action as
          | { handler?: string; args?: Record<string, unknown> }
          | undefined;
        if (a?.handler) action.mutate({ handler: a.handler, args: a.args ?? {} });
      }}
    >
      {label}
    </Button>
  );
}

registerRenderer("button", RenderButton);
