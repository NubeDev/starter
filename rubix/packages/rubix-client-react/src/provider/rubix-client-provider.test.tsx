// Smoke tests for `RubixClientProvider` + `useRubixClient`. The
// interesting bits: the provider shares the same `RubixClient`
// reference to descendants, mounts a nested
// `StarterClientProvider` so starter-side hooks resolve, and
// `useRubixClient` throws loudly outside a provider.

import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";

import { StarterClient } from "@nube/starter-client-ts";
import { useStarterClient } from "@nube/starter-client-react";
import { RubixClient } from "@nube/rubix-client-ts";

import { RubixClientProvider, useRubixClient } from "./rubix-client-provider.js";

function makeClient(): RubixClient {
  return new RubixClient(new StarterClient({ baseUrl: "http://test.local" }));
}

function ShowClient() {
  const client = useRubixClient();
  return <div data-testid="base">{client.starter.baseUrl}</div>;
}

function ShowStarter() {
  const starter = useStarterClient();
  return <div data-testid="starter-base">{starter.baseUrl}</div>;
}

describe("RubixClientProvider", () => {
  it("provides the RubixClient to descendants", () => {
    const client = makeClient();
    render(
      <RubixClientProvider client={client}>
        <ShowClient />
      </RubixClientProvider>,
    );
    expect(screen.getByTestId("base").textContent).toBe("http://test.local");
  });

  it("also mounts a StarterClientProvider for the wrapped starter", () => {
    const client = makeClient();
    render(
      <RubixClientProvider client={client}>
        <ShowStarter />
      </RubixClientProvider>,
    );
    expect(screen.getByTestId("starter-base").textContent).toBe(
      "http://test.local",
    );
  });
});

describe("useRubixClient", () => {
  it("throws outside a provider", () => {
    // Swallow React's expected error log to keep test output clean.
    const spy = vi
      .spyOn(console, "error")
      .mockImplementation(() => undefined);
    expect(() => render(<ShowClient />)).toThrow(
      /useRubixClient\(\) called outside <RubixClientProvider>/,
    );
    spy.mockRestore();
  });
});
