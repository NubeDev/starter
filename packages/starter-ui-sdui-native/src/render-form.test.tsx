import { describe, expect, it } from "vitest";
import { RenderForm } from "./render-form.js";
import { Providers } from "./test-wrappers.js";
import { byKit, mount } from "./test-utils.js";

describe("RenderForm", () => {
  it("renders a Column + submit Button when submit is defined", () => {
    const root = mount(
      <Providers>
        <RenderForm
          node={{
            type: "form",
            submit: { handler: "save", label: "Save" },
            children: [],
          }}
        />
      </Providers>,
    );
    expect(byKit(root, "column")).not.toBeNull();
    expect(byKit(root, "button")?.textContent).toBe("Save");
  });

  it("omits the submit button when submit is missing", () => {
    const root = mount(
      <Providers>
        <RenderForm node={{ type: "form", children: [] }} />
      </Providers>,
    );
    expect(byKit(root, "button")).toBeNull();
  });
});
