import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:rubix_api/rubix_api.dart';
import 'package:rubix_flutter/core/network/network_providers.dart';

/// Generated OpenAPI client wired to the app's existing Dio instance.
///
/// Reuses `dioProvider`, so the AuthInterceptor and baseUrl-from-active-
/// connection wiring apply uniformly to generated calls.
///
/// Re-generate the underlying `rubix_api` package with `make api-client`
/// from `rubix/flutter/`. Hand-edits to `packages/rubix_api/` are
/// forbidden.
final apiClientProvider = Provider<RubixApi?>((ref) {
  final dio = ref.watch(dioProvider);
  if (dio == null) return null;
  // Pass an empty interceptor list so the generator's default auth
  // interceptors are NOT added — auth is already handled by the
  // AuthInterceptor attached to `dio` in `dioProvider`.
  return RubixApi(dio: dio, interceptors: const []);
});
