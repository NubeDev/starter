import { describe, expect, it } from "vitest";

import { createMockServer } from "./mock-server.js";

const user = { subject: "u-1", email: "a@b", role: "admin" as const };

describe("createMockServer", () => {
  it("returns 401 on /auth/me when no user is set", async () => {
    const server = createMockServer();
    const res = await server.fetch("http://t/auth/me");
    expect(res.status).toBe(401);
    expect(res.headers.get("content-type")).toContain("problem+json");
  });

  it("returns the user on /auth/me when one is set", async () => {
    const server = createMockServer({ user });
    const res = await server.fetch("http://t/auth/me");
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual(user);
  });

  it("accepts valid credentials at /auth/login", async () => {
    const server = createMockServer({ validLogin: { email: "a@b", password: "pw" } });
    const res = await server.fetch("http://t/auth/login", {
      method: "POST",
      body: JSON.stringify({ email: "a@b", password: "pw" }),
    });
    expect(res.status).toBe(200);
  });

  it("rejects bad credentials at /auth/login", async () => {
    const server = createMockServer({ validLogin: { email: "a@b", password: "pw" } });
    const res = await server.fetch("http://t/auth/login", {
      method: "POST",
      body: JSON.stringify({ email: "a@b", password: "wrong" }),
    });
    expect(res.status).toBe(401);
  });

  it("clears user on /auth/logout", async () => {
    const server = createMockServer({ user });
    const res = await server.fetch("http://t/auth/logout", { method: "POST" });
    expect(res.status).toBe(204);
    expect(server.state.user).toBeNull();
  });

  it("404s unknown routes with a problem body", async () => {
    const server = createMockServer();
    const res = await server.fetch("http://t/nope");
    expect(res.status).toBe(404);
    expect(res.headers.get("content-type")).toContain("problem+json");
  });

  it("counts every request", async () => {
    const server = createMockServer();
    await server.fetch("http://t/auth/me");
    await server.fetch("http://t/auth/me");
    expect(server.requestCount()).toBe(2);
  });

  it("setUser flips the next /auth/me response", async () => {
    const server = createMockServer();
    expect((await server.fetch("http://t/auth/me")).status).toBe(401);
    server.setUser(user);
    expect((await server.fetch("http://t/auth/me")).status).toBe(200);
  });
});
