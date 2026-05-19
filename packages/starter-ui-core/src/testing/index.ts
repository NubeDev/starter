// Testing helpers. Two pieces:
//
// - `createMockServer()` — dependency-free fetch shim. Plug it into a
//   `StarterClient` via the `fetch` option to drive auth flows without
//   a real server.
// - `createAuthWrapper()` — wrapper component for RTL-style
//   `render(ui, { wrapper })`. Sets up `<QueryClientProvider>` +
//   `<AuthProvider>` so `useAuth()` works inside the unit under test.
//
// Neither helper pulls msw or @testing-library/react into ui-core. RTL
// is the consumer's choice; msw is overkill for the three endpoints we
// mock here. If a consumer needs richer mocks they can keep msw and
// pass `server.fetch` through their own bridge.

export { createMockServer } from "./mock-server.js";
export type { MockServer, MockServerState } from "./mock-server.js";
export { createAuthWrapper } from "./wrapper.js";
export type { AuthWrapperOptions } from "./wrapper.js";
