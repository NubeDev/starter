/**
 * Read the owner token seeded by global-setup.
 */

import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const ENV_FILE = path.resolve(__dirname, "../.env.e2e");

export function ownerToken(): string {
  const content = fs.readFileSync(ENV_FILE, "utf-8");
  const match = content.match(/^OWNER_TOKEN=(.+)$/m);
  if (!match) throw new Error("OWNER_TOKEN not found in .env.e2e — did global-setup run?");
  return match[1]!;
}
