// connection/use-lan-scan.ts — React hook around `scanLan`.
//
// Resolves the device's IP via `expo-network` and runs a /24 sweep.
// Reports incremental hits + progress so the UI can render results as
// they arrive without blocking on the full sweep.

import { useCallback, useEffect, useRef, useState } from 'react';
import * as Network from 'expo-network';

import { scanLan, type ScanHit, type ScanOptions } from './scan';

export interface LanScanState {
  scanning: boolean;
  done: number;
  total: number;
  hits: ScanHit[];
  error: string | null;
  localIp: string | null;
  start: (
    opts?: Pick<ScanOptions, 'port' | 'path' | 'timeoutMs' | 'concurrency'>,
  ) => Promise<void>;
  cancel: () => void;
  reset: () => void;
}

export function useLanScan(): LanScanState {
  const [scanning, setScanning] = useState(false);
  const [done, setDone] = useState(0);
  const [total, setTotal] = useState(0);
  const [hits, setHits] = useState<ScanHit[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [localIp, setLocalIp] = useState<string | null>(null);
  const abortRef = useRef<AbortController | null>(null);

  useEffect(
    () => () => {
      abortRef.current?.abort();
    },
    [],
  );

  const start = useCallback(
    async (
      opts: Pick<ScanOptions, 'port' | 'path' | 'timeoutMs' | 'concurrency'> = {},
    ) => {
      setScanning(true);
      setError(null);
      setHits([]);
      setDone(0);
      setTotal(254);
      const ac = new AbortController();
      abortRef.current = ac;
      try {
        const ip = await Network.getIpAddressAsync();
        setLocalIp(ip);
        await scanLan(ip, {
          ...opts,
          signal: ac.signal,
          onProgress: (d, t) => {
            setDone(d);
            setTotal(t);
          },
          onHit: (hit) => setHits((prev) => [...prev, hit]),
        });
      } catch (e) {
        if (!ac.signal.aborted) {
          setError((e as Error)?.message ?? String(e));
        }
      } finally {
        setScanning(false);
        if (abortRef.current === ac) abortRef.current = null;
      }
    },
    [],
  );

  const cancel = useCallback(() => {
    abortRef.current?.abort();
  }, []);

  const reset = useCallback(() => {
    setHits([]);
    setDone(0);
    setTotal(0);
    setError(null);
  }, []);

  return { scanning, done, total, hits, error, localIp, start, cancel, reset };
}
