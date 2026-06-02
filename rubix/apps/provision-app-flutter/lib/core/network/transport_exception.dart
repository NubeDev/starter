/// Error surfaced by [RubixTransport] when a request fails — either a non-2xx
/// agent response (message dug out of the `{error}` body) or a network fault.
/// Carries a clean message the UI can show directly (the React app threw a
/// plain `Error(msg)` here).
class TransportException implements Exception {
  const TransportException(this.message);
  final String message;

  @override
  String toString() => message;
}
