// `BlockShell` — the standard panel wrapper an extension renders at
// the root of its exposed component.
//
// Three responsibilities:
//
// 1. An error boundary so an extension crash never propagates into
//    the host shell. The boundary renders a small fallback that
//    surfaces the extension id (so an operator looking at the page
//    can identify *which* extension broke).
// 2. A `<Suspense/>` loading skeleton — extensions commonly fetch
//    on first render; the host doesn't want to render a blank gap
//    while that resolves.
// 3. A consistent outer shell (data attributes, className hooks)
//    so host CSS can target every extension uniformly. The extension
//    decides the inner content; the shell decides the border.

import * as React from "react";

import { useSlotContext } from "./slot-context.js";

export interface BlockShellProps {
  /** Override the default fallback rendered while the panel suspends. */
  loading?: React.ReactNode;
  /** Override the default fallback rendered when the panel throws. */
  errorFallback?: (err: unknown, extensionId: string) => React.ReactNode;
  /** Optional className appended to the shell root. */
  className?: string;
  children: React.ReactNode;
}

/**
 * Wrap an extension panel's exposed component:
 *
 * ```tsx
 * export default function Panel() {
 *   return (
 *     <BlockShell>
 *       <YourActualContent />
 *     </BlockShell>
 *   );
 * }
 * ```
 *
 * The shell reads the slot context for telemetry / error tagging.
 */
export function BlockShell(props: BlockShellProps): React.ReactElement {
  const slot = useSlotContext();
  return (
    <div
      className={
        props.className
          ? `starter-ext-block ${props.className}`
          : "starter-ext-block"
      }
      data-ext-id={slot.extensionId}
      data-ext-slot={slot.slotId}
    >
      <ExtensionErrorBoundary
        extensionId={slot.extensionId}
        fallback={props.errorFallback}
      >
        <React.Suspense
          fallback={props.loading ?? <DefaultLoading slotId={slot.slotId} />}
        >
          {props.children}
        </React.Suspense>
      </ExtensionErrorBoundary>
    </div>
  );
}

interface ErrorBoundaryProps {
  extensionId: string;
  fallback: BlockShellProps["errorFallback"];
  children: React.ReactNode;
}

interface ErrorBoundaryState {
  error: unknown;
}

/**
 * Internal React error boundary. Class component because hooks
 * cannot catch render-phase errors in v0.1 React.
 */
class ExtensionErrorBoundary extends React.Component<
  ErrorBoundaryProps,
  ErrorBoundaryState
> {
  override state: ErrorBoundaryState = { error: null };

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    return { error };
  }

  override componentDidCatch(error: unknown, info: React.ErrorInfo): void {
    // The host's tracing layer is intentionally not reached from here
    // — this package depends only on React. A consumer that wants
    // error reporting wraps `BlockShell` (or replaces this boundary)
    // at the host shell level.
    // eslint-disable-next-line no-console
    console.error(
      `[starter-ext] extension ${this.props.extensionId} crashed in render:`,
      error,
      info,
    );
  }

  override render(): React.ReactNode {
    if (this.state.error !== null) {
      const fb = this.props.fallback ?? defaultErrorFallback;
      return fb(this.state.error, this.props.extensionId);
    }
    return this.props.children;
  }
}

function defaultErrorFallback(err: unknown, extensionId: string): React.ReactElement {
  const msg = err instanceof Error ? err.message : String(err);
  return (
    <div role="alert" className="starter-ext-block__error">
      <strong>Extension failed:</strong> {extensionId}
      <div>{msg}</div>
    </div>
  );
}

function DefaultLoading(props: { slotId: string }): React.ReactElement {
  return (
    <div
      aria-busy="true"
      aria-live="polite"
      className="starter-ext-block__loading"
      data-slot={props.slotId}
    >
      Loading…
    </div>
  );
}
