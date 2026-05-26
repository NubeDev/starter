/// `SduiProvider` — exposes `SduiNotifier` to the widget tree.
library;

import 'package:flutter/widgets.dart';

import '../state/sdui_notifier.dart';

class SduiProvider extends InheritedNotifier<SduiNotifier> {
  const SduiProvider({
    super.key,
    required SduiNotifier super.notifier,
    required super.child,
  });

  /// Returns the nearest [SduiNotifier], subscribing the caller to rebuilds.
  static SduiNotifier of(BuildContext context) {
    final p = context.dependOnInheritedWidgetOfExactType<SduiProvider>();
    assert(p != null, 'No SduiProvider found in context.');
    return p!.notifier!;
  }

  /// Returns the nearest [SduiNotifier] without subscribing.
  static SduiNotifier read(BuildContext context) {
    final p = context.getInheritedWidgetOfExactType<SduiProvider>();
    assert(p != null, 'No SduiProvider found in context.');
    return p!.notifier!;
  }
}
