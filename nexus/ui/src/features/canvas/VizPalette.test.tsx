import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, within } from "@testing-library/react";

import { WIDGET_CATALOG, WIDGET_TYPES } from "@/features/widgets/catalog";
import { VizPalette } from "@/features/canvas/VizPalette";

// Unmount between cases so each render's palette is queried in isolation
// (this suite isn't using a global auto-cleanup).
afterEach(cleanup);

describe("VizPalette", () => {
  it("renders one tile per catalog widget type", () => {
    const { container } = render(<VizPalette onPick={() => {}} />);
    const q = within(container);
    for (const type of WIDGET_TYPES) {
      // Each tile's title is "Drag to add a <Label> panel".
      expect(q.getByTitle(`Drag to add a ${WIDGET_CATALOG[type].label} panel`)).toBeInTheDocument();
    }
  });

  it("reports the dragged type on drag start", () => {
    const onPick = vi.fn();
    const { container } = render(<VizPalette onPick={onPick} />);
    const tile = within(container).getByTitle("Drag to add a Pie panel");
    // jsdom's DataTransfer is minimal; supply a stub so the handler's
    // setData call doesn't throw.
    fireEvent.dragStart(tile, {
      dataTransfer: { setData: vi.fn(), effectAllowed: "" },
    });
    expect(onPick).toHaveBeenCalledWith("pie");
  });
});
