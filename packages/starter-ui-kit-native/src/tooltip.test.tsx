import { describe, expect, it } from "vitest";

import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "./tooltip.js";
import { a11y, bySlot, mount } from "./test-utils.js";

describe("<Tooltip>", () => {
  it("trigger ships button role + long-press hint", () => {
    const root = mount(
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger accessibilityLabel="Info">…</TooltipTrigger>
          <TooltipContent>Hint</TooltipContent>
        </Tooltip>
      </TooltipProvider>,
    );
    const el = bySlot(root, "pressable");
    expect(a11y(el, "accessibilityRole")).toBe("button");
    expect(a11y(el, "accessibilityLabel")).toBe("Info");
    expect(a11y(el, "accessibilityHint")).toMatch(/long-press/i);
  });

  it("snapshot stable (closed)", () => {
    expect(
      mount(
        <Tooltip>
          <TooltipTrigger accessibilityLabel="?">?</TooltipTrigger>
          <TooltipContent>Hint</TooltipContent>
        </Tooltip>,
      ),
    ).toMatchSnapshot();
  });
});
