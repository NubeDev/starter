// Vitest setup — wire `@testing-library/react`'s `cleanup()` into
// `afterEach` so renders from separate tests don't accumulate in the
// jsdom document.

import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

afterEach(() => {
  cleanup();
});
