// `/auth/*` client methods. Cookie-based; the client never handles
// raw tokens — the browser handles the session cookie automatically
// when `credentials: "include"` is set.

import { StarterClient } from "../client/client.js";

export interface LoginRequest {
  email: string;
  password: string;
}

export interface MeResponse {
  subject: string;
  email: string;
  role: "reader" | "writer" | "admin";
}

declare module "../client/client.js" {
  interface StarterClient {
    login(request: LoginRequest): Promise<void>;
    logout(): Promise<void>;
    me(): Promise<MeResponse>;
  }
}

StarterClient.prototype.login = async function login(this: StarterClient, request: LoginRequest) {
  await this.fetch(`${this.baseUrl}/auth/login`, {
    method: "POST",
    credentials: "include",
    headers: { ...this.headers, "content-type": "application/json" },
    body: JSON.stringify(request),
  });
};

StarterClient.prototype.logout = async function logout(this: StarterClient) {
  await this.fetch(`${this.baseUrl}/auth/logout`, {
    method: "POST",
    credentials: "include",
    headers: this.headers,
  });
};

StarterClient.prototype.me = async function me(this: StarterClient): Promise<MeResponse> {
  const res = await this.fetch(`${this.baseUrl}/auth/me`, {
    credentials: "include",
    headers: this.headers,
  });
  return (await res.json()) as MeResponse;
};
