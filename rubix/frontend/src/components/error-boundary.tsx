// `ErrorBoundary` — catches errors thrown by descendant queries /
// mutations (when `throwOnError` is set) and renders a localised
// failure surface keyed off `RubixError.code`.
//
// When the thrown error is a `RubixError` carrying a catalogue code
// (e.g. `rubix.system.disk.full`), the matching message from the
// `react-intl` catalogue is rendered with the diagnostic params.
// Otherwise we fall back to a generic `errors.unknown` string.
//
// The boundary is intentionally a class component — only class
// components can implement React's `componentDidCatch` /
// `getDerivedStateFromError` lifecycle. A `key` prop reset is
// exposed via `resetKey` so consumers can recover the subtree
// without remounting the whole app.

import { Component, type ReactNode } from 'react'
import { useIntl, type IntlShape } from 'react-intl'
import { RubixError } from '@nube/rubix-client-ts'

interface Props {
  children: ReactNode
  /** Bumping this value tears down the failed subtree and rebuilds. */
  resetKey?: unknown
}

interface State {
  error: Error | null
}

class ErrorBoundaryInner extends Component<Props & { intl: IntlShape }, State> {
  state: State = { error: null }

  static getDerivedStateFromError(error: Error): State {
    return { error }
  }

  componentDidUpdate(prev: Props & { intl: IntlShape }) {
    if (prev.resetKey !== this.props.resetKey && this.state.error) {
      this.setState({ error: null })
    }
  }

  render() {
    const { error } = this.state
    if (!error) return this.props.children

    const { intl } = this.props
    const code = error instanceof RubixError ? error.code : undefined
    // RubixError carries optional `problem.detail` and code params via
    // the Diagnostic envelope; pass-through whatever we have so the
    // catalogue placeholders resolve.
    const params =
      error instanceof RubixError && error.problem
        ? (error.problem as unknown as Record<string, unknown>)
        : {}

    const message = code
      ? intl.formatMessage(
          { id: code, defaultMessage: error.message },
          params as Record<string, string | number>,
        )
      : intl.formatMessage(
          { id: 'errors.unknown', defaultMessage: error.message },
        )

    return (
      <div
        role="alert"
        className="glass mx-auto my-12 max-w-2xl rounded-2xl border border-red-500/30 p-6"
      >
        <div className="text-[11px] font-semibold uppercase tracking-[0.22em] text-red-400">
          {intl.formatMessage({ id: 'errors.title', defaultMessage: 'Something went wrong' })}
        </div>
        <p className="mt-3 text-sm text-[color:var(--color-text)]">{message}</p>
        {code ? (
          <p className="mt-2 font-mono text-[10px] text-[color:var(--color-subtle)]">{code}</p>
        ) : null}
      </div>
    )
  }
}

export function ErrorBoundary({ children, resetKey }: Props) {
  const intl = useIntl()
  return (
    <ErrorBoundaryInner intl={intl} resetKey={resetKey}>
      {children}
    </ErrorBoundaryInner>
  )
}
