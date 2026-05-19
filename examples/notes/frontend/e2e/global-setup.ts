/**
 * Playwright global setup — starts the Rust backend for E2E tests.
 *
 * Workflow:
 * 1. Builds `starter-notes` (skip if already built).
 * 2. Runs `notes migrate` against a fresh SQLite file in /tmp.
 * 3. Runs `notes claim --yes` to get an owner bearer token.
 * 4. Spawns `notes serve` and waits for `/health` to return 200.
 * 5. Writes `.env.e2e` with the bearer token so tests can read it.
 * 6. Returns a teardown function that kills the server process.
 */

import { execSync, spawn, type ChildProcess } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";
import type { FullConfig } from "@playwright/test";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const WORKSPACE_ROOT = path.resolve(__dirname, "../../../..");
const NOTES_BIN = path.join(WORKSPACE_ROOT, "target/debug/notes");
const FRONTEND_ROOT = path.resolve(__dirname, "..");
const DB_PATH = path.join(FRONTEND_ROOT, ".e2e-notes.db");
const DATABASE_URL = `sqlite:${DB_PATH}?mode=rwc`;
const ENV_FILE = path.join(FRONTEND_ROOT, ".env.e2e");
const HTTP_PORT = 8080;
const HTTP_BIND = `127.0.0.1:${HTTP_PORT}`;

let serverProc: ChildProcess | null = null;

export default async function globalSetup(_config: FullConfig): Promise<() => Promise<void>> {
  // Clean slate.
  if (fs.existsSync(DB_PATH)) fs.unlinkSync(DB_PATH);

  // Build the binary (no-op if already up to date).
  execSync("cargo build -p starter-notes", {
    cwd: WORKSPACE_ROOT,
    stdio: "pipe",
    env: { ...process.env },
  });

  // Migrate.
  execSync(`${NOTES_BIN} migrate --database-url "${DATABASE_URL}"`, {
    cwd: FRONTEND_ROOT,
    stdio: "pipe",
  });

  // Claim → bearer token.
  // `notes claim --yes` outputs the pending token. We must POST it to
  // /auth/claim to exchange it for the permanent owner_token.
  const pendingToken = execSync(
    `${NOTES_BIN} claim --yes --database-url "${DATABASE_URL}"`,
    { cwd: FRONTEND_ROOT, encoding: "utf-8" },
  ).trim().split("\n").pop()!.trim();
  if (!pendingToken || pendingToken.length < 10) {
    throw new Error(`unexpected claim output: ${pendingToken}`);
  }

  // Start the server before exchanging (need the HTTP endpoint).
  serverProc = spawn(NOTES_BIN, ["serve", "--database-url", DATABASE_URL, "--http-bind", HTTP_BIND], {
    cwd: FRONTEND_ROOT,
    env: { ...process.env, RUST_LOG: "info", EXTENSIONS_DIR: path.resolve(FRONTEND_ROOT, "../extensions") },
    stdio: ["ignore", "pipe", "pipe"],
  });

  // Wait for /health.
  await waitForHealth(`http://${HTTP_BIND}/health`, 10_000);

  // Exchange pending token for owner token.
  const claimRes = await fetch(`http://${HTTP_BIND}/auth/claim`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ token: pendingToken }),
  });
  if (!claimRes.ok) {
    throw new Error(`POST /auth/claim failed: ${claimRes.status} ${await claimRes.text()}`);
  }
  const claimBody = (await claimRes.json()) as { owner_token: string };
  const token = claimBody.owner_token;
  if (!token || token.length < 10) {
    throw new Error(`unexpected owner_token from /auth/claim`);
  }

  // Write the token so tests can import it.
  fs.writeFileSync(ENV_FILE, `OWNER_TOKEN=${token}\n`, "utf-8");

  return async () => {
    if (serverProc) {
      serverProc.kill("SIGTERM");
      serverProc = null;
    }
    if (fs.existsSync(DB_PATH)) fs.unlinkSync(DB_PATH);
    if (fs.existsSync(ENV_FILE)) fs.unlinkSync(ENV_FILE);
  };
}

async function waitForHealth(url: string, timeoutMs: number): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const res = await fetch(url);
      if (res.ok) return;
    } catch {
      // not ready yet
    }
    await new Promise((r) => setTimeout(r, 200));
  }
  throw new Error(`Backend did not become healthy at ${url} within ${timeoutMs}ms`);
}
