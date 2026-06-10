import { Badge } from "@nube/starter-ui-kit/components/badge";

import type { NodeCategory, NodeType } from "@/api/types";

// The builder palette: the registered node types grouped by category. A node
// is added by clicking it (the canvas places it) — click rather than HTML5
// drag so it works the same on touch and keeps the canvas the single place
// that owns positions.

const CATEGORY_ORDER: NodeCategory[] = ["input", "processor", "output"];
const CATEGORY_LABEL: Record<NodeCategory, string> = {
  input: "Inputs",
  processor: "Processors",
  output: "Outputs",
};

export function Palette({
  nodeTypes,
  onAdd,
}: {
  nodeTypes: NodeType[];
  onAdd: (type: NodeType) => void;
}) {
  const grouped = CATEGORY_ORDER.map((category) => ({
    category,
    items: nodeTypes.filter((n) => n.category === category),
  })).filter((g) => g.items.length > 0);

  return (
    <div className="flex flex-col gap-4">
      {grouped.map(({ category, items }) => (
        <div key={category} className="space-y-1.5">
          <p className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
            {CATEGORY_LABEL[category]}
          </p>
          <div className="flex flex-col gap-1.5">
            {items.map((type) => (
              <button
                key={type.kind}
                type="button"
                onClick={() => onAdd(type)}
                title={type.description}
                className="glass flex w-full items-start gap-2 rounded-lg px-2.5 py-2 text-left transition-colors hover:bg-accent/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
              >
                <div className="min-w-0 flex-1">
                  <p className="truncate text-sm font-medium text-foreground">
                    {type.label}
                  </p>
                  {type.description ? (
                    <p className="truncate text-[11px] text-muted-foreground">
                      {type.description}
                    </p>
                  ) : null}
                </div>
                <Badge variant="outline" className="shrink-0 text-[10px]">
                  {type.kind}
                </Badge>
              </button>
            ))}
          </div>
        </div>
      ))}
    </div>
  );
}
