/// `CustomRendererRegistry` — host-side lookup table for the
/// `custom` IR variant.
///
/// A host app registers `renderer_id → builder` entries; the
/// `SduiCustomWidget` looks the id up and falls back to a
/// placeholder when nothing matches.
library;

import 'package:flutter/widgets.dart';

import '../../models/component.dart';

typedef CustomComponentBuilder = Widget Function(
  BuildContext context,
  CustomComponent component,
);

class CustomRendererRegistry extends InheritedWidget {
  const CustomRendererRegistry({
    super.key,
    required this.builders,
    required super.child,
  });

  final Map<String, CustomComponentBuilder> builders;

  Map<String, CustomComponentBuilder> get _map => builders;

  static Map<String, CustomComponentBuilder> of(BuildContext context) {
    final reg =
        context.dependOnInheritedWidgetOfExactType<CustomRendererRegistry>();
    return reg?._map ?? const {};
  }

  @override
  bool updateShouldNotify(CustomRendererRegistry oldWidget) =>
      !identical(oldWidget.builders, builders);
}
