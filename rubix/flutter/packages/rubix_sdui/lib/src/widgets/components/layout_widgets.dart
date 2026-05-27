/// Layout widgets — `page`, `row`, `col`.
///
/// Wave 1 subset for the proof slice (docs/PROOF.md). `grid`,
/// `tabs`, `section`, `divider`, `spacer`, `field_group` land next.
library;

import 'package:flutter/material.dart';

import '../../models/component.dart';
import '../sdui_renderer.dart';

/// Parses `raw['children']` into a list of [SduiComponent]s. Returns
/// an empty list when the field is missing or not a list — the
/// resolver guarantees the shape, this is defence in depth.
List<SduiComponent> _children(Map<String, Object?> raw) {
  final v = raw['children'];
  if (v is! List) return const [];
  return v
      .whereType<Map>()
      .map((m) => SduiComponent.fromJson(m.cast<String, Object?>()))
      .toList(growable: false);
}

class SduiPageWidget extends StatelessWidget {
  const SduiPageWidget({super.key, required this.component});
  final PageComponent component;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final title = component.raw['title'] as String?;
    final children = _children(component.raw);

    return ListView(
      padding: const EdgeInsets.all(16),
      children: [
        if (title != null && title.isNotEmpty) ...[
          Text(title, style: theme.textTheme.headlineSmall),
          const SizedBox(height: 16),
        ],
        for (final child in children) ...[
          buildComponent(context, child),
          const SizedBox(height: 16),
        ],
      ],
    );
  }
}

/// `row` — children laid out horizontally with their `span`
/// driving `Expanded.flex`. The renderer doesn't validate that
/// children are `col`s; the server enforces that (see
/// `rubix-tools/.../layout.rs`).
class SduiRowWidget extends StatelessWidget {
  const SduiRowWidget({super.key, required this.component});
  final RowComponent component;

  @override
  Widget build(BuildContext context) {
    final children = _children(component.raw);
    if (children.isEmpty) return const SizedBox.shrink();

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        for (var i = 0; i < children.length; i++) ...[
          if (i > 0) const SizedBox(width: 12),
          Expanded(
            flex: _span(children[i]),
            child: buildComponent(context, children[i]),
          ),
        ],
      ],
    );
  }

  int _span(SduiComponent c) {
    final v = c.raw['span'];
    if (v is int) return v.clamp(1, 12);
    if (v is num) return v.toInt().clamp(1, 12);
    return 12;
  }
}

/// `col` — children stacked vertically. `span` is read by the
/// parent `row`; the col itself just owns its vertical layout.
class SduiColWidget extends StatelessWidget {
  const SduiColWidget({super.key, required this.component});
  final ColComponent component;

  @override
  Widget build(BuildContext context) {
    final children = _children(component.raw);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var i = 0; i < children.length; i++) ...[
          if (i > 0) const SizedBox(height: 12),
          buildComponent(context, children[i]),
        ],
      ],
    );
  }
}
