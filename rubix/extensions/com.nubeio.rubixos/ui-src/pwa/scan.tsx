// `scan.tsx` — camera scan via the native BarcodeDetector when available,
// always with a manual text-input fallback (paste rubix:// URL or serial).
import * as React from "react";
import { Camera, Keyboard } from "lucide-react";

// Minimal structural type for the experimental BarcodeDetector API.
interface DetectedBarcode {
  rawValue: string;
}
interface BarcodeDetectorLike {
  detect(source: CanvasImageSource): Promise<ReadonlyArray<DetectedBarcode>>;
}
type BarcodeDetectorCtor = new (opts?: { formats?: string[] }) => BarcodeDetectorLike;

export function Scan({ onScanned }: { onScanned: (barcode: string) => void }): React.ReactElement {
  const videoRef = React.useRef<HTMLVideoElement>(null);
  const [manual, setManual] = React.useState("");
  const [camError, setCamError] = React.useState<string | null>(null);
  const supported = typeof window !== "undefined" && "BarcodeDetector" in window;

  React.useEffect(() => {
    if (!supported) return;
    let stream: MediaStream | null = null;
    let raf = 0;
    let stopped = false;
    const Ctor = (window as unknown as { BarcodeDetector: BarcodeDetectorCtor }).BarcodeDetector;
    const detector = new Ctor({ formats: ["qr_code", "code_128"] });

    (async () => {
      try {
        stream = await navigator.mediaDevices.getUserMedia({ video: { facingMode: "environment" } });
        const video = videoRef.current;
        if (!video) return;
        video.srcObject = stream;
        await video.play();
        const tick = async () => {
          if (stopped || !video.videoWidth) {
            raf = requestAnimationFrame(tick);
            return;
          }
          try {
            const hits = await detector.detect(video);
            if (hits[0]?.rawValue) {
              onScanned(hits[0].rawValue);
              return;
            }
          } catch {
            /* transient decode errors are expected between frames */
          }
          raf = requestAnimationFrame(tick);
        };
        raf = requestAnimationFrame(tick);
      } catch (e) {
        setCamError(e instanceof Error ? e.message : String(e));
      }
    })();

    return () => {
      stopped = true;
      cancelAnimationFrame(raf);
      stream?.getTracks().forEach((t) => t.stop());
    };
  }, [supported, onScanned]);

  return (
    <div className="flex flex-col gap-4">
      {supported ? (
        <div className="relative overflow-hidden rounded-xl border border-border/60 bg-black">
          <video ref={videoRef} muted playsInline className="aspect-square w-full object-cover" />
          <div className="pointer-events-none absolute inset-6 rounded-lg border-2 border-primary/70" />
        </div>
      ) : (
        <div className="flex items-center gap-2 rounded-lg border border-border/60 bg-card p-3 text-sm text-muted-foreground">
          <Camera className="size-4" />
          Camera scanning unavailable on this device — enter the code below.
        </div>
      )}
      {camError ? (
        <div role="alert" className="rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
          {camError}
        </div>
      ) : null}

      <form
        className="flex flex-col gap-2"
        onSubmit={(e) => {
          e.preventDefault();
          if (manual.trim()) onScanned(manual.trim());
        }}
      >
        <label htmlFor="manual-bc" className="flex items-center gap-2 text-sm font-medium text-foreground">
          <Keyboard className="size-4" /> Or type / paste a code
        </label>
        <input
          id="manual-bc"
          value={manual}
          onChange={(e) => setManual(e.target.value)}
          placeholder="rubix://add?... or device serial"
          className="rounded-lg border border-border/60 bg-background px-3 py-3 text-base text-foreground outline-none focus:border-primary"
        />
        <button
          type="submit"
          disabled={!manual.trim()}
          className="rounded-lg bg-primary px-4 py-3 text-base font-semibold text-primary-foreground disabled:opacity-50"
        >
          Use this
        </button>
      </form>
    </div>
  );
}
