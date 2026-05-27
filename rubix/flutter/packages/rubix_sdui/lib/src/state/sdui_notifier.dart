/// `SduiNotifier` — ChangeNotifier ViewModel for an SDUI page.
///
/// Owns a [SduiState], calls [SduiService] for resolve / action /
/// table fetches, and emits [SduiSideEffect]s for the widget layer
/// to consume.
///
/// Pure Dart (only `package:flutter/foundation.dart` for
/// `ChangeNotifier`).
library;

import 'dart:async';

import 'package:flutter/foundation.dart' show ChangeNotifier;

import '../client/sdui_service.dart';
import '../models/action.dart';
import '../models/resolve.dart';
import 'sdui_state.dart';
import 'sdui_status.dart';
import 'side_effect.dart';

class SduiNotifier extends ChangeNotifier {
  SduiNotifier({required SduiService service}) : _service = service;

  final SduiService _service;
  final StreamController<SduiSideEffect> _sideEffects =
      StreamController<SduiSideEffect>.broadcast();

  SduiState _state = const SduiState();
  SduiState get state => _state;

  /// Side-effect stream — toast / navigate / dialog / download.
  Stream<SduiSideEffect> get sideEffects => _sideEffects.stream;

  Future<void> load({
    required String pageRef,
    String? targetRef,
    Map<String, String> stack = const {},
    Map<String, Object?>? pageState,
  }) async {
    _setState(_state.copyWith(status: SduiStatus.loading, clearError: true));

    try {
      final result = await _service.resolve(
        ResolveRequest(
          pageRef: pageRef,
          targetRef: targetRef,
          stack: stack,
          pageState: pageState ?? const <String, Object?>{},
        ),
      );
      _setState(
        _state.copyWith(
          status: SduiStatus.loaded,
          tree: result.tree,
          subscriptions: result.subscriptions,
          clearError: true,
        ),
      );
    } catch (e) {
      // SduiVersionMismatchError, SduiServerError, and anything else
      // surface verbatim; the renderer handles version mismatch
      // specially and falls through to a generic banner otherwise.
      _setState(
        _state.copyWith(
          status: SduiStatus.error,
          error: e,
        ),
      );
    }
  }

  Future<void> dispatchAction(
    SduiAction action, {
    Map<String, Object?> context = const {},
  }) async {
    throw UnimplementedError(
      'SduiNotifier.dispatchAction — lands with the `button` widget. '
      'See packages/rubix_sdui/docs/PROOF.md.',
    );
  }

  /// Two-way bound control write (toggle, slider). Out of proof scope.
  Future<void> writeControl(String componentId, Object? value) async {
    throw UnimplementedError(
      'SduiNotifier.writeControl — open question in FLUTTER.md §5.',
    );
  }

  /// SSE bridge entrypoint. Out of proof scope (resolver returns
  /// empty subscriptions for the data-flow page).
  void pushSlotEvent({
    required String entityId,
    required String slot,
    required Object? value,
    required int ts,
  }) {
    // No-op until the SSE bridge lands.
  }

  void _setState(SduiState next) {
    _state = next;
    notifyListeners();
  }

  // ignore: unused_element
  void _emit(SduiSideEffect e) => _sideEffects.add(e);

  @override
  void dispose() {
    _sideEffects.close();
    super.dispose();
  }
}
