import type { CSSProperties } from "react";
import type { NodeKindRegistry } from "../nodes/NodeRegistry.js";
import type { NodeKindSpec } from "../types.js";

interface NodePaletteProps {
  registry: NodeKindRegistry;
  /** Fired when the user clicks a kind in the palette. */
  onPick: (spec: NodeKindSpec) => void;
  style?: CSSProperties;
  className?: string;
}

/**
 * Minimal palette of available node kinds, grouped by `category`.
 * The host app wires `onPick` to (e.g.) place a new node at the
 * viewport centre or open a drag-and-drop affordance.
 */
export function NodePalette({ registry, onPick, style, className }: NodePaletteProps) {
  const grouped = new Map<string, NodeKindSpec[]>();
  for (const { spec } of registry.list()) {
    const cat = spec.category ?? "other";
    const arr = grouped.get(cat) ?? [];
    arr.push(spec);
    grouped.set(cat, arr);
  }

  return (
    <div
      className={className}
      style={{
        background: "var(--sf-palette-bg, #ffffff)",
        border: "1px solid var(--sf-palette-border, #e2e8f0)",
        borderRadius: 8,
        padding: 8,
        minWidth: 180,
        fontFamily: "var(--sf-font, ui-sans-serif, system-ui, sans-serif)",
        fontSize: 12,
        ...style,
      }}
    >
      {Array.from(grouped.entries()).map(([cat, specs]) => (
        <div key={cat} style={{ marginBottom: 8 }}>
          <div
            style={{
              fontSize: 10,
              textTransform: "uppercase",
              letterSpacing: 0.5,
              color: "#64748b",
              marginBottom: 4,
            }}
          >
            {cat}
          </div>
          {specs.map((spec) => (
            <button
              key={spec.kind}
              type="button"
              onClick={() => onPick(spec)}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 8,
                width: "100%",
                padding: "6px 8px",
                background: "transparent",
                border: "1px solid transparent",
                borderRadius: 6,
                cursor: "pointer",
                textAlign: "left",
              }}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = "var(--sf-palette-hover, #f1f5f9)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = "transparent";
              }}
            >
              <span
                style={{
                  width: 10,
                  height: 10,
                  borderRadius: 3,
                  background: spec.color ?? "#94a3b8",
                }}
              />
              <span>{spec.label}</span>
            </button>
          ))}
        </div>
      ))}
    </div>
  );
}
