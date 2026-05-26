/// Facade over the generated `rubix_api` Dio client for the
/// `/api/v1/ui/*` surface.
///
/// **Scaffold only.** Real wiring lands in stage F4 once
/// `rubix-agent` mounts `starter-sdui-routes` (stage F1) and
/// `make api-client` regenerates `rubix_api` with the `UiApi` /
/// `UiTableApi` classes. Until then each method throws
/// `UnimplementedError`.
///
/// Pure Dart — no Flutter imports.
library;

import '../models/action.dart';
import '../models/action_response.dart';
import '../models/resolve.dart';
import '../models/table_query.dart';

class SduiService {
  const SduiService();

  /// Resolves a page tree from `POST /api/v1/ui/resolve`.
  Future<SduiResolveResult> resolve(ResolveRequest request) async {
    throw UnimplementedError(
      'SduiService.resolve — blocked on stage F1 (rubix-agent must mount '
      'starter-sdui-routes and rubix_api must regenerate with UiApi). '
      'See rubix/docs/design/sdui/renderer/FLUTTER.md.',
    );
  }

  /// Dispatches an action to `POST /api/v1/ui/action`.
  Future<SduiActionResponse> dispatchAction(
    SduiAction action, {
    Map<String, Object?> context = const {},
  }) async {
    throw UnimplementedError(
      'SduiService.dispatchAction — blocked on stage F1. '
      'See rubix/docs/design/sdui/renderer/FLUTTER.md.',
    );
  }

  /// Pages a table source via `GET /api/v1/ui/table`.
  Future<TableResponse> queryTable(TableQuery query) async {
    throw UnimplementedError(
      'SduiService.queryTable — blocked on stage F1. '
      'See rubix/docs/design/sdui/renderer/FLUTTER.md.',
    );
  }
}
