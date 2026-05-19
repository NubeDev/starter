// Surface-level unit test for `registerExtensionContributions`.
//
// The SDK call is intentionally thin (it delegates straight to the
// handle the host passes in). The test pins that invariant so a
// future refactor doesn't accidentally route through a stale
// in-package state — that would create the "extension registered
// components into the SDK, not the host" bug class.

import { describe, expect, it, vi } from "vitest";

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "./register.js";

describe("registerExtensionContributions", () => {
  it("forwards contributions to the handle the host supplied", () => {
    const register = vi.fn();
    const handle: ExtensionRemoteHandle = {
      id: "com.acme.test",
      singletons: Object.freeze({ react: { fake: true } }),
      register,
    };
    const Comp = () => null;
    registerExtensionContributions(handle, { components: { Comp } });
    expect(register).toHaveBeenCalledTimes(1);
    expect(register).toHaveBeenCalledWith({ components: { Comp } });
  });
});
