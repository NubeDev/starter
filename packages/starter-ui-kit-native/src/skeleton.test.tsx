import { describe, expect, it } from "vitest";

import { Skeleton } from "./skeleton.js";
import { mount } from "./test-utils.js";

describe("<Skeleton>", () => {
  it("is hidden from a11y tree by default (accessibilityRole=none)", () => {
    const root = mount(<Skeleton />);
    const el = root.querySelector("[data-moti=view]");
    expect(el?.getAttribute("accessibilityrole")).toBe("none");
  });

  it("opts in to progressbar role when a label is supplied", () => {
    const root = mount(<Skeleton accessibilityLabel="Loading rows" />);
    const el = root.querySelector("[data-moti=view]");
    expect(el?.getAttribute("accessibilityrole")).toBe("progressbar");
    expect(el?.getAttribute("accessibilitylabel")).toBe("Loading rows");
  });

  it("snapshot stable", () => {
    expect(mount(<Skeleton width={100} height={20} />)).toMatchSnapshot();
  });
});
