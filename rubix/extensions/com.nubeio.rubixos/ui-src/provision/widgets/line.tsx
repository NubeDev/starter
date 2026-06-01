// `line.tsx` — small sparkline-style trend preview using recharts. Mock series.
import * as React from "react";
import { Line, LineChart, ResponsiveContainer } from "recharts";
import type { WidgetProps } from "./registry";

const MOCK = [4, 6, 5, 8, 7, 9, 6, 10, 8, 11].map((v, i) => ({ i, v }));

export function LineWidget({ title, unit }: WidgetProps): React.ReactElement {
  return (
    <div className="flex flex-col gap-1">
      <span className="text-xs text-muted-foreground">
        {title}
        {unit ? ` (${unit})` : ""}
      </span>
      <div className="h-12 w-full text-primary">
        <ResponsiveContainer width="100%" height="100%">
          <LineChart data={MOCK}>
            <Line
              type="monotone"
              dataKey="v"
              stroke="currentColor"
              strokeWidth={2}
              dot={false}
              isAnimationActive={false}
            />
          </LineChart>
        </ResponsiveContainer>
      </div>
    </div>
  );
}
