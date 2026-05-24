import type { ReactNode } from "react";
import type { NodeKindSpec, NodeRunState, SlotName, SlotSpec } from "../types.js";
import { SlotHandle } from "../slots/SlotHandle.js";
import { cn } from "../lib/cn.js";
import { useFlowMessages } from "../i18n/context.js";

export interface BaseNodeProps {
  spec: NodeKindSpec;
  label?: string;
  state?: NodeRunState;
  selected?: boolean;
  /**
   * Live per-slot values, keyed by slot name. Renderers show each
   * value as a small monospaced badge adjacent to its slot label.
   * Currently rendered for output slots only — input badges land
   * once the engine emits input-write events.
   */
  slotValues?: Record<SlotName, unknown>;
  /** Optional body slot for kind-specific config preview. */
  children?: ReactNode;
}

/**
 * Visual frame for every node kind. Renders an iconified header, a
 * two-column slot grid (inputs left, outputs right), an optional
 * config slot (`children`), and a run-state ring.
 *
 * Visuals are 100% class-driven against the `--sf-*` variables defined
 * in `styles/flow.css`. Hosts can fully restyle by overriding those
 * variables or by targeting `.sf-node`, `.sf-node__header`, and
 * `[data-node-kind="…"]` in their own CSS.
 */
export function BaseNode({
  spec,
  label,
  state = "idle",
  selected,
  slotValues,
  children,
}: BaseNodeProps) {
  const messages = useFlowMessages();
  const accent = spec.color ?? "var(--sf-accent-default, #0ea5e9)";
  const resolvedLabel = label ?? messages.kindLabels?.[spec.kind] ?? spec.label;
  return (
    <div
      className={cn(
        "sf-node",
        `sf-node--${state}`,
        selected && "sf-node--selected",
      )}
      data-node-kind={spec.kind}
      data-node-state={state}
      style={{ ["--sf-accent" as string]: accent }}
    >
      <div className="sf-node__header">
        <span className="sf-node__icon" aria-hidden="true">
          <NodeIcon icon={spec.icon} fallback={spec.label} />
        </span>
        <span className="sf-node__title">{resolvedLabel}</span>
        <span className="sf-node__kind">{spec.kind}</span>
        <StateDot state={state} />
      </div>
      <div className="sf-node__body">
        <div className="sf-node__col sf-node__col--in">
          {spec.inputs.map((s) => (
            <SlotRow key={`in-${s.name}`} kindId={spec.kind} spec={s} side="input" value={slotValues?.[s.name]} />
          ))}
        </div>
        <div className="sf-node__col sf-node__col--out">
          {spec.outputs.map((s) => (
            <SlotRow key={`out-${s.name}`} kindId={spec.kind} spec={s} side="output" value={slotValues?.[s.name]} />
          ))}
        </div>
      </div>
      {children ? <div className="sf-node__extra">{children}</div> : null}
    </div>
  );
}

const BADGE_MAX = 48;

function SlotRow({
  kindId,
  spec,
  side,
  value,
}: {
  kindId: string;
  spec: SlotSpec;
  side: "input" | "output";
  value: unknown;
}) {
  const messages = useFlowMessages();
  const rendered = renderSlotValue(value);
  const labelOverride = messages.slotLabels?.[`${kindId}.${spec.name}`];
  return (
    <div className={cn("sf-slot-row", `sf-slot-row--${side}`)}>
      <SlotHandle spec={spec} side={side} labelOverride={labelOverride} />
      {rendered !== null ? (
        <span
          className="sf-slot__value"
          data-slot-kind={spec.kind}
          title={rendered}
        >
          {rendered.length > BADGE_MAX
            ? `${rendered.slice(0, BADGE_MAX - 1)}…`
            : rendered}
        </span>
      ) : null}
    </div>
  );
}

const STATE_LABEL: Record<NodeRunState, string> = {
  idle: "Idle",
  ready: "Ready",
  running: "Running",
  ok: "Succeeded",
  error: "Failed",
  cancelled: "Cancelled",
  skipped: "Skipped",
};

function StateDot({ state }: { state: NodeRunState }) {
  const messages = useFlowMessages();
  if (state === "idle") return null;
  const label = messages.state[state] ?? STATE_LABEL[state];
  return (
    <span
      className={cn("sf-node__state", `sf-node__state--${state}`)}
      aria-label={label}
      title={label}
    />
  );
}

/**
 * Lightweight icon renderer. `spec.icon` is a free-form string
 * identifier (e.g. "sparkles"). The package stays icon-library-free;
 * we render a 1–2 char monogram derived from the icon string (or the
 * label as fallback). Consumers who want lucide / phosphor / heroicons
 * can supply their own NodeKindComponent and ignore BaseNode entirely.
 */
function NodeIcon({ icon, fallback }: { icon: string | undefined; fallback: string }) {
  const seed = (icon ?? fallback ?? "").trim();
  if (!seed) return <span className="sf-node__icon-glyph" />;
  // Take the first letter of each dash-separated word, max 2 chars.
  const initials = seed
    .split(/[-\s_]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((p) => p[0]?.toUpperCase() ?? "")
    .join("");
  return <span className="sf-node__icon-glyph">{initials || seed[0]?.toUpperCase()}</span>;
}

/**
 * Project an arbitrary slot value to a compact display string, or
 * `null` if the badge should be hidden entirely.
 *
 * Null/undefined collapse the badge. Strings render verbatim. Numbers
 * and booleans use their canonical `String(...)`. Everything else
 * falls through to `JSON.stringify`, with a fallback to `String(v)`
 * when the value contains a cycle or otherwise refuses to serialize.
 */
function renderSlotValue(v: unknown): string | null {
  if (v === null || v === undefined) return null;
  if (typeof v === "string") return v;
  if (typeof v === "number" || typeof v === "boolean" || typeof v === "bigint") {
    return String(v);
  }
  try {
    return JSON.stringify(v);
  } catch {
    return String(v);
  }
}
