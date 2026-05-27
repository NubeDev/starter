import 'dart:io';
import 'dart:math';

import 'package:shelf/shelf.dart';

/// Loads the shared-secret bearer token from [path], generating a new
/// 256-bit random one if the file does not yet exist.
///
/// The file is the single source of truth: the server reads it on
/// boot, the `Makefile` reads it again to inject the same value into
/// the Flutter web build via `--dart-define`. Mode 0600 so other
/// users on the box can't crib it.
String loadOrCreateToken(String path) {
  final file = File(path);
  if (file.existsSync()) {
    final token = file.readAsStringSync().trim();
    if (token.isNotEmpty) return token;
  }
  file.parent.createSync(recursive: true);
  final token = _generateToken();
  file.writeAsStringSync(token);
  // Best-effort lockdown; ignored on platforms that don't support it.
  try {
    Process.runSync('chmod', ['600', path]);
  } catch (_) {}
  return token;
}

String _generateToken() {
  final rng = Random.secure();
  final bytes = List<int>.generate(32, (_) => rng.nextInt(256));
  // URL-safe base64 without padding.
  const alphabet =
      'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_';
  final sb = StringBuffer();
  for (final b in bytes) {
    sb.write(alphabet[b % 64]);
  }
  return sb.toString();
}

/// Shelf middleware that requires `Authorization: Bearer <token>` on
/// every request except [openPaths] (typically `/healthz`).
///
/// Returns 401 (no/bad header) or 403 (wrong token).
Middleware requireBearer(String expected, {Set<String> openPaths = const {}}) {
  return (Handler inner) {
    return (Request req) async {
      if (openPaths.contains('/${req.url.path}')) {
        return inner(req);
      }
      final header = req.headers['authorization'];
      if (header == null || !header.toLowerCase().startsWith('bearer ')) {
        return Response.unauthorized(
          '{"error":"missing bearer token"}',
          headers: {'content-type': 'application/json'},
        );
      }
      final provided = header.substring(7).trim();
      if (!_constantTimeEq(provided, expected)) {
        return Response.forbidden(
          '{"error":"invalid token"}',
          headers: {'content-type': 'application/json'},
        );
      }
      return inner(req);
    };
  };
}

/// Length-independent comparison guards against trivial timing
/// side-channels; not bulletproof (Dart string compare semantics) but
/// raises the floor.
bool _constantTimeEq(String a, String b) {
  if (a.length != b.length) return false;
  var diff = 0;
  for (var i = 0; i < a.length; i++) {
    diff |= a.codeUnitAt(i) ^ b.codeUnitAt(i);
  }
  return diff == 0;
}
