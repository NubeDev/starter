// connection/scan.ts — LAN scanner.
//
// Given the device's IPv4 address (typically obtained via
// `Network.getIpAddressAsync()` from `expo-network`), derives the /24
// subnet and probes every host on a port for the rubix `/healthz`
// endpoint. Hits are reported via `onHit` as they come in so the UI
// can show partial results without waiting for the full sweep.
//
// Why /24 only: that's the home / small-office sweet spot (~250 hosts,
// ~3s at 32-way concurrency). /16 would be 65k hosts — a different
// product. Operators on bigger networks know their server URL.
//
// Why not multicast/mDNS: rubix-agent doesn't advertise via Bonjour
// today, and adding `react-native-zeroconf` would mean a native build
// step + an iOS Info.plist entitlement. The /healthz fetch sweep works
// against any rubix-agent unchanged, including ones behind a port
// forward on a different host.

const DEFAULT_PORT = 8088;
const DEFAULT_PATH = '/healthz';
const DEFAULT_TIMEOUT_MS = 1500;
const DEFAULT_CONCURRENCY = 32;

export interface ScanHit {
  readonly baseUrl: string;
  readonly ip: string;
  readonly port: number;
  readonly version?: string;
}

export interface ScanOptions {
  port?: number;
  path?: string;
  timeoutMs?: number;
  concurrency?: number;
  signal?: AbortSignal;
  onProgress?: (done: number, total: number) => void;
  onHit?: (hit: ScanHit) => void;
}

/**
 * Expand an IPv4 address into the 254 host addresses of its /24
 * (skipping `.0` and `.255`). Returns `null` for inputs that are not
 * IPv4 (e.g. an IPv6 address from `Network.getIpAddressAsync()` on a
 * cellular-only device, or the sentinel `0.0.0.0`).
 */
export function deriveSubnet24(localIp: string): readonly string[] | null {
  const m = /^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.\d{1,3}$/.exec(localIp);
  if (!m) return null;
  const [, a, b, c] = m;
  if (a === '0' || a === '127') return null;
  const out: string[] = [];
  for (let i = 1; i <= 254; i++) out.push(`${a}.${b}.${c}.${i}`);
  return out;
}

export async function scanLan(
  localIp: string,
  opts: ScanOptions = {},
): Promise<readonly ScanHit[]> {
  const subnet = deriveSubnet24(localIp);
  if (!subnet) {
    throw new Error(
      `scanLan: not a scannable IPv4 address (${localIp || '<empty>'})`,
    );
  }
  const hosts: readonly string[] = subnet;
  const port = opts.port ?? DEFAULT_PORT;
  const path = opts.path ?? DEFAULT_PATH;
  const timeoutMs = opts.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  const concurrency = Math.max(1, opts.concurrency ?? DEFAULT_CONCURRENCY);

  const hits: ScanHit[] = [];
  const total = hosts.length;
  let cursor = 0;
  let done = 0;

  async function worker(): Promise<void> {
    while (true) {
      if (opts.signal?.aborted) return;
      const idx = cursor++;
      if (idx >= total) return;
      const hit = await probe(hosts[idx], port, path, timeoutMs, opts.signal);
      done++;
      opts.onProgress?.(done, total);
      if (hit) {
        hits.push(hit);
        opts.onHit?.(hit);
      }
    }
  }

  const workers: Promise<void>[] = [];
  for (let i = 0; i < concurrency; i++) workers.push(worker());
  await Promise.all(workers);
  return hits;
}

async function probe(
  ip: string,
  port: number,
  path: string,
  timeoutMs: number,
  outerSignal: AbortSignal | undefined,
): Promise<ScanHit | null> {
  const baseUrl = `http://${ip}:${port}`;
  const ac = new AbortController();
  const timer = setTimeout(() => ac.abort(), timeoutMs);
  const onOuterAbort = (): void => ac.abort();
  outerSignal?.addEventListener('abort', onOuterAbort);
  try {
    const resp = await fetch(`${baseUrl}${path}`, { signal: ac.signal });
    if (!resp.ok) return null;
    let version: string | undefined;
    try {
      const body = (await resp.json()) as { version?: string } | null;
      version = body?.version;
    } catch {
      /* /healthz may return text — that's still a hit. */
    }
    return { baseUrl, ip, port, version };
  } catch {
    // ECONNREFUSED / timeout / network unreachable / DNS — all "no rubix here".
    return null;
  } finally {
    clearTimeout(timer);
    outerSignal?.removeEventListener('abort', onOuterAbort);
  }
}
