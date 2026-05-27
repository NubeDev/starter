/// HTTP facade for the `/api/v1/ui/*` surface.
///
/// Uses Dio directly while the generated `rubix_api` client catches
/// up (see `docs/PROOF.md` T1 and `PENDING.md` F1). The seam is the
/// `SduiService` class; swap the body of `resolve` for the generated
/// `UiApi` call when it's available — callers don't move.
///
/// Pure Dart — no Flutter imports.
library;

import 'package:dio/dio.dart';

import '../models/action.dart';
import '../models/action_response.dart';
import '../models/component_tree.dart';
import '../models/ir_version.dart';
import '../models/resolve.dart';
import '../models/table_query.dart';

class SduiService {
  SduiService({required Dio dio, String baseUrl = '/api/v1'})
      : _dio = dio,
        _baseUrl = _stripTrailingSlash(baseUrl);

  final Dio _dio;
  final String _baseUrl;

  /// Resolves a page tree from `POST /api/v1/ui/resolve`.
  ///
  /// Throws [SduiVersionMismatchError] if the server emits an
  /// `ir_version` newer than [kSupportedIrVersion]; wraps every
  /// other transport / parse failure in [SduiServerError].
  Future<SduiResolveResult> resolve(ResolveRequest request) async {
    final Response<Object?> response;
    try {
      response = await _dio.post<Object?>(
        '$_baseUrl/ui/resolve',
        data: request.toJson(),
        options: Options(contentType: 'application/json'),
      );
    } on DioException catch (e) {
      throw SduiServerError(e);
    }

    final data = response.data;
    if (data is! Map) {
      throw SduiServerError(
        'Expected JSON object from /ui/resolve, got ${data.runtimeType}',
      );
    }

    final map = data.cast<String, Object?>();
    final renderRaw = map['render'];
    if (renderRaw is! Map) {
      throw SduiServerError('Missing or invalid `render` in resolve response');
    }
    final tree =
        ComponentTree.fromJson(renderRaw.cast<String, Object?>());

    if (tree.irVersion > kSupportedIrVersion) {
      throw SduiVersionMismatchError(
        serverVersion: tree.irVersion,
        supportedVersion: kSupportedIrVersion,
      );
    }

    final subsRaw = map['subscriptions'];
    final subscriptions = <SduiSubject>[];
    if (subsRaw is List) {
      for (final item in subsRaw) {
        if (item is Map) {
          subscriptions.add(
            SduiSubject.fromJson(item.cast<String, Object?>()),
          );
        }
      }
    }

    return SduiResolveResult(tree: tree, subscriptions: subscriptions);
  }

  /// Dispatches an action to `POST /api/v1/ui/action`.
  ///
  /// Out of proof scope — see `docs/PROOF.md` "Out of scope".
  Future<SduiActionResponse> dispatchAction(
    SduiAction action, {
    Map<String, Object?> context = const {},
  }) async {
    throw UnimplementedError(
      'SduiService.dispatchAction — lands with the `button` widget. '
      'See packages/rubix_sdui/docs/PROOF.md.',
    );
  }

  /// Pages a table source via `GET /api/v1/ui/table`.
  ///
  /// Out of proof scope.
  Future<TableResponse> queryTable(TableQuery query) async {
    throw UnimplementedError(
      'SduiService.queryTable — lands with the `table` widget '
      '(F6 Wave 2). See packages/rubix_sdui/docs/PROOF.md.',
    );
  }

  static String _stripTrailingSlash(String s) =>
      s.endsWith('/') ? s.substring(0, s.length - 1) : s;
}
