/// `SduiNotifier` — ChangeNotifier ViewModel for an SDUI page.
///
/// Owns a `SduiState`, calls `SduiService` for resolve / action /
/// table fetches, and emits `SduiSideEffect`s for the widget layer
/// to consume.
///
/// **Scaffold only.** Method bodies throw `UnimplementedError`
/// pointing back to the relevant FLUTTER.md stage.
///
/// Pure Dart (only `package:flutter/foundation.dart` for
/// `ChangeNotifier`).
library;

import 'dart:async';

import 'package:flutter/foundation.dart' show ChangeNotifier;

import '../client/sdui_service.dart';
import '../models/action.dart';
import 'sdui_state.dart';
import 'sdui_status.dart';
import 'side_effect.dart';

class SduiNotifier extends ChangeNotifier {
  SduiNotifier({SduiService? service})
      : _service = service ?? const SduiService();

  final SduiService _service;
  final StreamController<SduiSideEffect> _sideEffects =
      StreamController<SduiSideEffect>.broadcast();

  SduiState _state = const SduiState();
  SduiState get state => _state;

  /// Side-effect stream — toast / navigate / dialog / download.
  Stream<SduiSideEffect> get sideEffects => _sideEffects.stream;

  // -------------------------------------------------------------------------
  // Public API — stage F5
  // -------------------------------------------------------------------------

  Future<void> load({
    required String pageRef,
    String? targetRef,
    Map<String, String> stack = const {},
    Map<String, Object?>? pageState,
  }) async {
    _setState(_state.copyWith(status: SduiStatus.loading, clearError: true));
    throw UnimplementedError(
      'SduiNotifier.load — stage F5. See FLUTTER.md.',
    );
  }

  Future<void> dispatchAction(
    SduiAction action, {
    Map<String, Object?> context = const {},
  }) async {
    throw UnimplementedError(
      'SduiNotifier.dispatchAction — stage F5. See FLUTTER.md.',
    );
  }

  /// Two-way bound control write (toggle, slider).
  ///
  /// Open question (FLUTTER.md §5 Q1): action-only path vs
  /// resurrecting `/api/v1/slots`. Until that's resolved, this
  /// stub throws.
  Future<void> writeControl(String componentId, Object? value) async {
    throw UnimplementedError(
      'SduiNotifier.writeControl — stage F5 + open question. See FLUTTER.md §5.',
    );
  }

  /// SSE bridge entrypoint — called by the host when a
  /// `slot_changed` event arrives.
  void pushSlotEvent({
    required String entityId,
    required String slot,
    required Object? value,
    required int ts,
  }) {
    // TODO(F5): port reference repo's pushSlotEvent — walks
    // subscription plan, updates liveValues + liveSeries.
  }

  // -------------------------------------------------------------------------
  // Internals
  // -------------------------------------------------------------------------

  void _setState(SduiState next) {
    _state = next;
    notifyListeners();
  }

  // ignore: unused_element
  void _emit(SduiSideEffect e) => _sideEffects.add(e);

  // Hint to the analyser that the service field will be read soon.
  // ignore: unused_element
  SduiService get _serviceRef => _service;

  @override
  void dispose() {
    _sideEffects.close();
    super.dispose();
  }
}
