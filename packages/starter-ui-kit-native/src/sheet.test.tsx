import { describe, expect, it } from "vitest";

import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "./sheet.js";
import { mount } from "./test-utils.js";

describe("<Sheet>", () => {
  it("trigger gets a button role; content renders when open", () => {
    const root = mount(
      <Sheet defaultOpen>
        <SheetTrigger accessibilityLabel="Open">…</SheetTrigger>
        <SheetContent>
          <SheetHeader>
            <SheetTitle>Title</SheetTitle>
          </SheetHeader>
        </SheetContent>
      </Sheet>,
    );
    const triggers = root.querySelectorAll('[accessibilityrole="button"]');
    expect(triggers.length).toBeGreaterThan(0);
    expect(root.querySelector('[data-slot="modal"]')).not.toBeNull();
    expect(root).toMatchSnapshot();
  });
});
