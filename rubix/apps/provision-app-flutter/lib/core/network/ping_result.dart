import 'package:flutter/foundation.dart';

/// Outcome of a pre-login connectivity probe against `{base}/healthz`. Ported
/// from the React transport's `PingResult`. Never thrown — the verdict is the
/// value, so the Connect screen can tell "host unreachable" apart from "bad
/// credentials".
@immutable
class PingResult {
  const PingResult({required this.ok, required this.message, this.latencyMs});
  final bool ok;
  final String message;
  final int? latencyMs;
}
