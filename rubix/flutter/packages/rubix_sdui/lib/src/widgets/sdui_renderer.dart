/// `SduiRenderer` — root dispatcher for the resolved component tree.
///
/// **Scaffold only.** The dispatch arms are filled in over stages
/// F6 Wave 1 (minimum useful set) and Wave 2 (full catalogue). For
/// now every variant degrades to `SduiUnknownWidget` so the
/// renderer compiles end-to-end.
library;

import 'package:flutter/material.dart';

import '../models/component.dart';
import '../models/component_tree.dart';
import '../models/resolve.dart';
import '../state/sdui_status.dart';
import 'components/display_widgets.dart';
import 'components/layout_widgets.dart';
import 'components/sentinel_widgets.dart';
import 'sdui_provider.dart';

class SduiRenderer extends StatelessWidget {
  const SduiRenderer({super.key});

  @override
  Widget build(BuildContext context) {
    final state = SduiProvider.of(context).state;

    return switch (state.status) {
      SduiStatus.idle => const SizedBox.shrink(),
      SduiStatus.loading => const Center(child: CircularProgressIndicator()),
      SduiStatus.error => _buildError(context, state.error),
      SduiStatus.loaded => _buildTree(context, state.tree!),
    };
  }

  Widget _buildTree(BuildContext context, ComponentTree tree) =>
      buildComponent(context, tree.root);

  Widget _buildError(BuildContext context, Object? error) {
    if (error is SduiVersionMismatchError) {
      return _VersionMismatchBanner(error: error);
    }
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(
          error?.toString() ?? 'Unknown error',
          textAlign: TextAlign.center,
        ),
      ),
    );
  }
}

/// Top-level dispatcher. Exposed so composite widgets (page, row,
/// form, wizard, ...) can recurse into their children.
///
/// All arms currently fall through to [SduiUnknownWidget]; per-variant
/// arms are added incrementally per FLUTTER.md F6.
Widget buildComponent(BuildContext context, SduiComponent component) {
  return switch (component) {
    PageComponent() => SduiPageWidget(component: component),
    RowComponent() => SduiRowWidget(component: component),
    ColComponent() => SduiColWidget(component: component),
    KpiComponent() => SduiKpiWidget(component: component),
    KpiGridComponent() => SduiKpiGridWidget(component: component),
    ChartComponent() => SduiChartWidget(component: component),
    DanglingComponent() => const SduiDanglingWidget(),
    ForbiddenComponent() => const SduiForbiddenWidget(),
    CustomComponent() => SduiCustomWidget(component: component),
    // TODO(F6 Wave 1 follow-up): grid, tabs, section, divider, spacer,
    //   text, heading, badge, markdown, toggle, slider,
    //   select, text_field, number_field, checkbox, segmented, form,
    //   card, button.
    // TODO(F6 Wave 2): everything else.
    _ => SduiUnknownWidget(type: component.type),
  };
}

class _VersionMismatchBanner extends StatelessWidget {
  const _VersionMismatchBanner({required this.error});
  final SduiVersionMismatchError error;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Material(
      color: scheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Icon(Icons.upgrade, color: scheme.onErrorContainer),
            const SizedBox(width: 12),
            Expanded(
              child: Text(
                'This page requires a newer version of the app '
                '(IR v${error.serverVersion}, app supports v${error.supportedVersion}). '
                'Please update.',
                style: TextStyle(color: scheme.onErrorContainer),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
