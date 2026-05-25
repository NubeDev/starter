import { describe, expect, it } from "vitest";

import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "./dialog.js";
import { mount } from "./test-utils.js";

describe("<Dialog>", () => {
  it("renders content with accessibilityRole=alert when open", () => {
    const root = mount(
      <Dialog defaultOpen>
        <DialogTrigger accessibilityLabel="Open">…</DialogTrigger>
        <DialogContent accessibilityLabel="Confirm">
          <DialogHeader>
            <DialogTitle>Confirm</DialogTitle>
            <DialogDescription>Are you sure?</DialogDescription>
          </DialogHeader>
        </DialogContent>
      </Dialog>,
    );
    expect(root.querySelector('[accessibilityrole="alert"]')).not.toBeNull();
    expect(root).toMatchSnapshot();
  });
});
