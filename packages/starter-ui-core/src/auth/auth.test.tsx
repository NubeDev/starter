// End-to-end test for <AuthProvider> + useAuth() + sessionStrategy,
// driven through createMockServer + createAuthWrapper. This is the
// canonical "does the testing surface actually work" check — if this
// breaks, every consumer test that copies the pattern breaks too.

import { afterEach, describe, expect, it } from "vitest";
import { act, cleanup, render, screen, waitFor } from "@testing-library/react";

afterEach(() => {
  cleanup();
});
import { StarterClient } from "@nube/starter-client-ts";

import { useAuth } from "./provider.js";
import { sessionStrategy, tokenStrategy } from "./strategy.js";
import { createMockServer } from "../testing/mock-server.js";
import { createAuthWrapper } from "../testing/wrapper.js";

const user = { subject: "u-1", email: "a@b", role: "admin" as const };

function Probe() {
  const auth = useAuth();
  return (
    <div>
      <span data-testid="status">{auth.status}</span>
      <span data-testid="user">{auth.user?.subject ?? "none"}</span>
      <button
        data-testid="login"
        onClick={() => {
          void auth.login({ kind: "credentials", email: "a@b", password: "pw" });
        }}
      >
        login
      </button>
      <button
        data-testid="logout"
        onClick={() => {
          void auth.logout();
        }}
      >
        logout
      </button>
    </div>
  );
}

describe("AuthProvider + sessionStrategy", () => {
  it("starts unauthenticated, transitions to authenticated on login, then logs out", async () => {
    const server = createMockServer({ validLogin: { email: "a@b", password: "pw" } });
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    const wrapper = createAuthWrapper({ client, strategy: sessionStrategy });

    render(<Probe />, { wrapper });

    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("unauthenticated"));

    // login() succeeds — but the mock server needs setUser before /auth/me
    // can return the user. Mirror what the real server does on login.
    server.setUser(user);
    await act(async () => {
      screen.getByTestId("login").click();
    });

    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("authenticated"));
    expect(screen.getByTestId("user").textContent).toBe("u-1");

    // sanity: requestCount went up — login + me round-tripped through the mock.
    expect(server.requestCount()).toBeGreaterThan(1);

    await act(async () => {
      screen.getByTestId("logout").click();
    });

    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("unauthenticated"));
    expect(screen.getByTestId("user").textContent).toBe("none");
  });

  it("stays unauthenticated when login is rejected", async () => {
    const server = createMockServer({ validLogin: { email: "a@b", password: "right" } });
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    const wrapper = createAuthWrapper({ client, strategy: sessionStrategy });

    function Bad() {
      const auth = useAuth();
      return (
        <div>
          <span data-testid="status">{auth.status}</span>
          <button
            data-testid="login"
            onClick={() => {
              auth.login({ kind: "credentials", email: "a@b", password: "wrong" }).catch(() => {});
            }}
          >
            login
          </button>
        </div>
      );
    }

    render(<Bad />, { wrapper });
    await waitFor(() => expect(screen.getByTestId("status").textContent).toBe("unauthenticated"));

    await act(async () => {
      screen.getByTestId("login").click();
    });

    // No transition to authenticated; status stays put.
    await new Promise((r) => setTimeout(r, 10));
    expect(screen.getByTestId("status").textContent).toBe("unauthenticated");
  });
});

describe("tokenStrategy", () => {
  it("attaches Bearer header on login and removes it on logout", async () => {
    const server = createMockServer();
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });

    // tokenStrategy validates the token by calling /auth/me, so seed the user.
    server.setUser(user);

    const strategy = tokenStrategy();
    const me = await strategy.login(client, { kind: "token", token: "sak_abc.def" });
    expect(me.subject).toBe("u-1");
    expect(client.headers["Authorization"]).toBe("Bearer sak_abc.def");

    await strategy.logout(client);
    expect(client.headers["Authorization"]).toBeUndefined();
  });

  it("rejects a token when /auth/me returns 401", async () => {
    const server = createMockServer();
    const client = new StarterClient({ baseUrl: "http://t", fetch: server.fetch });
    const strategy = tokenStrategy();

    await expect(strategy.login(client, { kind: "token", token: "bad" })).rejects.toBeDefined();
  });
});
