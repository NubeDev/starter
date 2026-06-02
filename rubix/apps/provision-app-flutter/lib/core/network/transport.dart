import 'package:dio/dio.dart';

import 'package:provision_app/core/network/auth_user.dart';
import 'package:provision_app/core/network/credential_store.dart';
import 'package:provision_app/core/network/ping_result.dart';
import 'package:provision_app/core/network/tenant.dart';
import 'package:provision_app/core/network/transport_exception.dart';

/// The single network layer the app talks to — a Dio port of the React app's
/// `webTransport.ts`. Talks to the rubix-agent REST API directly over the
/// network (LAN or internet), authenticating with a `sak_` Bearer token.
///
/// Why Bearer and not a session cookie: the agent's session cookie is
/// `SameSite=Lax`, so it is never sent cross-origin — and the whole point here
/// is a phone talking to a remote agent. `POST /auth/token` mints the cookie-
/// less `sak_…` Bearer (30-day TTL); every protected route accepts it with no
/// cookie and no CSRF. So we mint at login, persist it, and send it on every
/// call against the absolute agent URL.
class RubixTransport {
  RubixTransport(this._store, {Dio? dio}) : _dio = dio ?? Dio();

  final CredentialStore _store;
  final Dio _dio;

  String _baseUrl = '';
  String _token = '';

  // Coalesce concurrent identical reads onto one in-flight future; the epoch is
  // part of the key so a post-write read can't reuse a pre-write request.
  final Map<String, Future<dynamic>> _inFlight = {};
  int _readEpoch = 0;

  void _invalidateReads() {
    _readEpoch += 1;
    _inFlight.clear();
  }

  /// Load the persisted base URL + token at boot.
  Future<void> hydrate() async {
    _baseUrl = await _store.readBaseUrl();
    _token = await _store.readToken();
  }

  /// The agent base URL this device last connected to, or '' if none. Lets the
  /// Connect screen pre-fill the host the operator actually used (survives
  /// logout — logout drops the token, not the remembered host).
  String get savedBaseUrl => _baseUrl;

  String _url(String path) {
    final root = _baseUrl.replaceAll(RegExp(r'/+$'), '');
    return '$root$path';
  }

  /// No-auth liveness probe of `{baseUrl}/healthz`. Never throws — the failure
  /// is reported in [PingResult.ok] / [PingResult.message].
  Future<PingResult> ping(String nextBase) async {
    final root = nextBase.replaceAll(RegExp(r'/+$'), '');
    final target = root.isNotEmpty ? '$root/healthz' : '/healthz';
    final probe = Dio(
      BaseOptions(
        connectTimeout: const Duration(seconds: 4),
        receiveTimeout: const Duration(seconds: 4),
      ),
    );
    final started = DateTime.now();
    try {
      final res = await probe.getUri<dynamic>(Uri.parse(target));
      final latency = DateTime.now().difference(started).inMilliseconds;
      final code = res.statusCode ?? 0;
      if (code >= 200 && code < 300) {
        return PingResult(
          ok: true,
          latencyMs: latency,
          message: 'reachable in $latency ms',
        );
      }
      return PingResult(
        ok: false,
        message: 'agent answered HTTP $code (host reachable)',
      );
    } on DioException catch (e) {
      final reason = e.type == DioExceptionType.connectionTimeout ||
              e.type == DioExceptionType.receiveTimeout
          ? 'timed out'
          : (e.message ?? e.type.name);
      return PingResult(ok: false, message: 'cannot reach $target: $reason');
    }
  }

  /// Sign in: mint a Bearer at `/auth/token`, persist it, then fetch the
  /// principal for the UI. [tenantId] selects which org to bind the token to —
  /// required when the account belongs to more than one org (the agent returns
  /// 409 `tenant_required` otherwise, surfaced as [TenantRequiredException]),
  /// and an Admin may pass `*` ([superAdminTenant]) to see every org at once.
  Future<AuthUser> login(
    String nextBase,
    String email,
    String password, {
    String? tenantId,
  }) async {
    _baseUrl = nextBase;
    await _store.writeBaseUrl(nextBase);

    // Mint the token with a status-aware request so a 409 `tenant_required`
    // (multi-org user, no tenant picked) becomes a typed exception the Connect
    // screen can turn into an org picker — rather than a generic login error.
    final Response<dynamic> res;
    try {
      res = await _dio.requestUri<dynamic>(
        Uri.parse(_url('/api/v1/auth/token')),
        data: {
          'email': email,
          'password': password,
          if (tenantId != null) 'tenant_id': tenantId,
        },
        options: Options(
          method: 'POST',
          headers: const {
            'accept': 'application/json',
            'content-type': 'application/json',
          },
          validateStatus: (_) => true,
        ),
      );
    } on DioException catch (e) {
      throw TransportException(_dioMessage(e));
    }

    final code = res.statusCode ?? 0;
    final data = res.data;
    if (code == 409 && data is Map && data['error'] == 'tenant_required') {
      final raw = (data['memberships'] as List?) ?? const [];
      throw TenantRequiredException(
        raw
            .cast<Map<String, dynamic>>()
            .map(TenantMembership.fromJson)
            .toList(growable: false),
      );
    }
    if (code < 200 || code >= 300) {
      throw TransportException(_errorMessage(data, code));
    }

    _token = (data is Map ? data['token'] as String? : null) ?? '';
    await _store.writeToken(_token);
    _invalidateReads();

    final user = await me();
    if (user == null) {
      throw const TransportException(
        'login succeeded but identity lookup failed',
      );
    }
    return user;
  }

  /// Current principal, or null if unauthenticated.
  Future<AuthUser?> me() async {
    if (_baseUrl.isEmpty || _token.isEmpty) return null;
    try {
      final out = await _request<Map<String, dynamic>>(
        '/api/v1/auth/me',
        method: 'GET',
      );
      return AuthUser.fromJson(out);
    } on TransportException {
      return null;
    }
  }

  /// Bearer tokens are stateless client-side; clearing the stored token signs
  /// this device out (it stays valid server-side until its TTL — no per-token
  /// revoke route is exposed).
  Future<void> logout() async {
    _token = '';
    await _store.writeToken('');
    _invalidateReads();
  }

  /// Invoke a tool by id at `/api/v1/tools/:id`. [fresh] skips read-dedup so a
  /// read after a write never observes a coalesced pre-write result.
  Future<T> dispatch<T>(
    String toolId,
    Object? params, {
    bool fresh = false,
  }) {
    final body = params ?? const <String, dynamic>{};
    final key = '$toolId::${body.hashCode}::e$_readEpoch';
    if (!fresh) {
      final existing = _inFlight[key];
      if (existing != null) return existing as Future<T>;
    }
    final future = _request<T>(
      '/api/v1/tools/$toolId',
      method: 'POST',
      body: body,
    );
    if (!fresh) {
      _inFlight[key] = future;
      // Drop from the in-flight map once settled, but only if it's still ours
      // (a later epoch may have cleared + replaced it).
      future.whenComplete(() {
        if (identical(_inFlight[key], future)) _inFlight.remove(key);
      });
    }
    return future;
  }

  Future<T> _request<T>(
    String path, {
    required String method,
    Object? body,
  }) async {
    try {
      final res = await _dio.requestUri<dynamic>(
        Uri.parse(_url(path)),
        data: body,
        options: Options(
          method: method,
          headers: {
            'accept': 'application/json',
            if (body != null) 'content-type': 'application/json',
            if (_token.isNotEmpty) 'authorization': 'Bearer $_token',
          },
          // We map non-2xx into TransportException ourselves so the 401
          // token-drop runs uniformly.
          validateStatus: (_) => true,
        ),
      );
      final code = res.statusCode ?? 0;
      if (code < 200 || code >= 300) {
        if (code == 401) {
          // Stale/expired token (30-day TTL) → drop it so the app falls back to
          // the Connect screen instead of looping on a dead token.
          _token = '';
          await _store.writeToken('');
        }
        throw TransportException(_errorMessage(res.data, code));
      }
      return res.data as T;
    } on DioException catch (e) {
      throw TransportException(_dioMessage(e));
    }
  }

  String _errorMessage(Object? data, int code) {
    if (data is Map && data['error'] != null) return data['error'].toString();
    return 'HTTP $code';
  }

  /// Turn a raw [DioException] into a short, operator-readable line — the raw
  /// `DioException [connection error]: The XMLHttpRequest onError…` is noise.
  String _dioMessage(DioException e) {
    return switch (e.type) {
      DioExceptionType.connectionTimeout ||
      DioExceptionType.receiveTimeout ||
      DioExceptionType.sendTimeout =>
        'Timed out reaching the agent — check the URL and that it is running.',
      DioExceptionType.connectionError =>
        'Cannot reach the agent — check the base URL and your network.',
      DioExceptionType.badCertificate =>
        'The agent\'s TLS certificate was rejected.',
      _ => e.message ?? 'Network error (${e.type.name}).',
    };
  }
}
