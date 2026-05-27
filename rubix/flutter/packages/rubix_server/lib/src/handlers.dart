import 'dart:async';
import 'dart:convert';

import 'package:rubix_data/rubix_data.dart';
import 'package:shelf/shelf.dart';
import 'package:shelf_router/shelf_router.dart';
import 'package:sqlite3/sqlite3.dart';

/// Wires up the REST surface exposed to the Flutter web client.
///
/// Routes intentionally mirror the method names on
/// [ConnectionsRepository] / [SettingsRepository] one-to-one so the
/// REST client in the Flutter app stays a thin pass-through.
Router buildRouter(Database db) {
  final router = Router();

  router.get('/healthz', (Request _) => _json({'ok': true}));

  // ---- connections --------------------------------------------------
  router.get('/api/connections', (Request _) {
    final rows = db.select('SELECT * FROM connections ORDER BY id ASC;');
    return _json(rows.map(_rowToConnectionJson).toList());
  });

  router.post('/api/connections', (Request req) async {
    final body = await _readJson(req);
    if (body == null) return _badRequest('body must be a JSON object');
    final label = _requireString(body, 'label', maxLen: 128);
    final baseUrl = _requireString(body, 'baseUrl', maxLen: 2048);
    if (label == null) return _badRequest('label is required (1..128 chars)');
    if (baseUrl == null) return _badRequest('baseUrl is required');
    db.execute(
      'INSERT INTO connections (label, base_url) VALUES (?, ?);',
      [label, baseUrl],
    );
    return _json({'id': db.lastInsertRowId});
  });

  router.patch('/api/connections/<id|[0-9]+>', (Request req, String id) async {
    final body = await _readJson(req);
    if (body == null) return _badRequest('body must be a JSON object');
    final sets = <String>[];
    final args = <Object?>[];
    if (body.containsKey('label')) {
      final v = _requireString(body, 'label', maxLen: 128);
      if (v == null) return _badRequest('label must be a non-empty string');
      sets.add('label = ?');
      args.add(v);
    }
    if (body.containsKey('baseUrl')) {
      final v = _requireString(body, 'baseUrl', maxLen: 2048);
      if (v == null) return _badRequest('baseUrl must be a non-empty string');
      sets.add('base_url = ?');
      args.add(v);
    }
    if (sets.isEmpty) return _json({'updated': false});
    args.add(int.parse(id));
    db.execute('UPDATE connections SET ${sets.join(', ')} WHERE id = ?;', args);
    return _json({'updated': db.updatedRows > 0});
  });

  router.delete('/api/connections/<id|[0-9]+>', (Request _, String id) {
    db.execute('DELETE FROM connections WHERE id = ?;', [int.parse(id)]);
    return _json({'deleted': db.updatedRows});
  });

  router.post('/api/connections/<id|[0-9]+>/mark-used',
      (Request _, String id) {
    db.execute(
      "UPDATE connections SET last_used_at = datetime('now') WHERE id = ?;",
      [int.parse(id)],
    );
    return _json({'updated': db.updatedRows > 0});
  });

  // ---- active connection -------------------------------------------
  router.get('/api/connections/active', (Request _) {
    final rows = db.select(
      'SELECT active_connection_id FROM connection_state WHERE id = 1;',
    );
    final id = rows.isEmpty ? null : rows.first['active_connection_id'] as int?;
    return _json({'activeId': id});
  });

  router.put('/api/connections/active', (Request req) async {
    final body = await _readJson(req);
    if (body == null) return _badRequest('body must be a JSON object');
    final raw = body['activeId'];
    if (raw != null && raw is! int) {
      return _badRequest('activeId must be int or null');
    }
    db.execute(
      'INSERT INTO connection_state (id, active_connection_id) VALUES (1, ?) '
      'ON CONFLICT(id) DO UPDATE SET active_connection_id = excluded.active_connection_id;',
      [raw],
    );
    return _json({'ok': true});
  });

  // ---- settings -----------------------------------------------------
  router.get('/api/settings/connections-pin', (Request _) {
    final rows = db.select(
      'SELECT connections_pin FROM app_settings WHERE id = 1;',
    );
    final pin = rows.isEmpty ? null : rows.first['connections_pin'] as String?;
    return _json({'pin': pin});
  });

  router.put('/api/settings/connections-pin', (Request req) async {
    final body = await _readJson(req);
    if (body == null) return _badRequest('body must be a JSON object');
    final raw = body['pin'];
    if (raw != null && raw is! String) {
      return _badRequest('pin must be string or null');
    }
    db.execute(
      'INSERT INTO app_settings (id, connections_pin) VALUES (1, ?) '
      'ON CONFLICT(id) DO UPDATE SET connections_pin = excluded.connections_pin;',
      [raw],
    );
    return _json({'ok': true});
  });

  return router;
}

Map<String, Object?> _rowToConnectionJson(Row r) {
  return ConnectionDto(
    id: r['id'] as int,
    label: r['label'] as String,
    baseUrl: r['base_url'] as String,
    createdAt: DateTime.parse(r['created_at'] as String),
    lastUsedAt: r['last_used_at'] == null
        ? null
        : DateTime.parse(r['last_used_at'] as String),
  ).toJson();
}

Response _json(Object? body, {int status = 200}) => Response(
      status,
      body: jsonEncode(body),
      headers: {'content-type': 'application/json'},
    );

Response _badRequest(String msg) => _json({'error': msg}, status: 400);

Future<Map<String, dynamic>?> _readJson(Request req) async {
  final body = await req.readAsString();
  if (body.isEmpty) return const {};
  try {
    final v = jsonDecode(body);
    if (v is Map<String, dynamic>) return v;
    return null;
  } on FormatException {
    return null;
  }
}

String? _requireString(Map<String, dynamic> body, String key,
    {required int maxLen}) {
  final v = body[key];
  if (v is! String) return null;
  final trimmed = v.trim();
  if (trimmed.isEmpty || trimmed.length > maxLen) return null;
  return trimmed;
}

/// CORS for the dev flow. Pass [allowedOrigins] explicitly — defaults
/// to nothing because echoing `*` while we also require a bearer
/// token would let any page on the user's machine probe the API.
///
/// Browsers will refuse to send `Authorization` to `*` anyway, so
/// allowed origins must be set to the exact Flutter web dev origin
/// (e.g. `http://localhost:3031`).
Middleware corsMiddleware(Set<String> allowedOrigins) {
  return (Handler inner) {
    return (Request req) async {
      final origin = req.headers['origin'];
      final allowOrigin =
          origin != null && allowedOrigins.contains(origin) ? origin : null;

      Map<String, String> headersFor(String? o) => {
            if (o != null) ...{
              'access-control-allow-origin': o,
              'vary': 'origin',
              'access-control-allow-credentials': 'true',
              'access-control-allow-methods':
                  'GET,POST,PUT,PATCH,DELETE,OPTIONS',
              'access-control-allow-headers': 'content-type,authorization',
            },
          };

      if (req.method == 'OPTIONS') {
        return Response.ok('', headers: headersFor(allowOrigin));
      }
      final response = await inner(req);
      if (allowOrigin == null) return response;
      return response.change(
        headers: {...response.headers, ...headersFor(allowOrigin)},
      );
    };
  };
}
