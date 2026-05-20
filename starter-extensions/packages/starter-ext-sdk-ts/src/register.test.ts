// Surface-level unit test for `registerExtensionContributions`.
//
// The SDK call forwards `contributions` to the handle the host
// passes in, but the components themselves are wrapped in a
// `<HostBindingsProvider>` (Stage 3) so the prefs/i18n hooks see
// the host's singletons + the extension id without per-extension
// wiring. The test pins both invariants:
//
// - exactly one `handle.register` call;
// - the registered component for `"Comp"` is the SDK-supplied
//   wrapper (with a recognisable displayName) rather than the raw
//   component reference — proof that the wrapping actually happens.
//
// The deeper "the wrapper actually feeds bindings to the hooks"
// check lives in `extension-prefs-singleton.test.tsx`, which renders
// a wrapped panel and asserts the hooks resolve correctly.

import { describe, expect, it, vi } from "vitest";

import {
  registerExtensionContributions,
  type ExtensionRemoteHandle,
} from "./register.js";

describe("registerExtensionContributions", () => {
  it("registers exactly once with a HostBindings-wrapped component", () => {
    const register = vi.fn();
    const handle: ExtensionRemoteHandle = {
      id: "com.acme.test",
      singletons: Object.freeze({ react: { fake: true } }),
      register,
    };
    const Comp = () => null;
    registerExtensionContributions(handle, { components: { Comp } });
    expect(register).toHaveBeenCalledTimes(1);
    const passed = register.mock.calls[0]![0] as {
      components: Record<string, { displayName?: string }>;
    };
    expect(Object.keys(passed.components)).toEqual(["Comp"]);
    expect(passed.components["Comp"]).not.toBe(Comp);
    expect(passed.components["Comp"]!.displayName).toBe(
      "HostBindings(com.acme.test:Comp)",
    );
  });
});
