/// `SduiState` — immutable value class held by `SduiNotifier`.
///
/// Pure Dart — no Flutter imports.
library;

import '../models/component_tree.dart';
import '../models/resolve.dart';
import 'sdui_status.dart';

class SduiState {
  const SduiState({
    this.status = SduiStatus.idle,
    this.tree,
    this.subscriptions = const [],
    this.pageState = const {},
    this.liveValues = const {},
    this.liveSeries = const {},
    this.error,
  });

  final SduiStatus status;
  final ComponentTree? tree;
  final List<SduiSubject> subscriptions;

  /// Client-owned ephemeral state (`$page.*` bindings).
  final Map<String, Object?> pageState;

  /// Latest extracted slot value per widget id, fed by the SSE bridge.
  final Map<String, Object?> liveValues;

  /// Time-series points appended per widget id `(ts_ms, value)`.
  final Map<String, List<(int, double)>> liveSeries;

  final Object? error;

  SduiState copyWith({
    SduiStatus? status,
    ComponentTree? tree,
    List<SduiSubject>? subscriptions,
    Map<String, Object?>? pageState,
    Map<String, Object?>? liveValues,
    Map<String, List<(int, double)>>? liveSeries,
    Object? error,
    bool clearTree = false,
    bool clearError = false,
  }) {
    return SduiState(
      status: status ?? this.status,
      tree: clearTree ? null : (tree ?? this.tree),
      subscriptions: subscriptions ?? this.subscriptions,
      pageState: pageState ?? this.pageState,
      liveValues: liveValues ?? this.liveValues,
      liveSeries: liveSeries ?? this.liveSeries,
      error: clearError ? null : (error ?? this.error),
    );
  }
}
