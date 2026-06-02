import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/preview/widgets/widget_tile.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/page_header.dart';

/// The client view: pick a Site → one of its pages → render the widget tiles.
/// The payoff screen. [initialPageId] lets the scan-flow deep-link straight to
/// the page it just provisioned onto. Ported from the React `PagePreview`.
class PagePreviewScreen extends ConsumerStatefulWidget {
  const PagePreviewScreen({this.initialPageId, super.key});

  final String? initialPageId;

  @override
  ConsumerState<PagePreviewScreen> createState() => _PagePreviewScreenState();
}

class _PagePreviewScreenState extends ConsumerState<PagePreviewScreen> {
  List<SiteRow> _sites = const [];
  List<PageRow> _pages = const [];
  List<WidgetRow> _widgets = const [];
  String _siteId = '';
  late String _pageId = widget.initialPageId ?? '';
  int _lastRefresh = -1;
  String? _sitesError;

  Future<void> _loadSites() async {
    try {
      final list = await ref.read(bcApiProvider).sitesList();
      if (mounted) setState(() {
        _sites = list;
        _sitesError = null;
      });
    } catch (e) {
      // Surface instead of swallowing — a broken agent now shows a message
      // under the Site picker rather than a silently empty dropdown.
      if (mounted) setState(() => _sitesError = e.toString());
    }
  }

  /// When deep-linked to a page, discover its site so the pickers stay
  /// coherent.
  Future<void> _discoverSite() async {
    final target = widget.initialPageId;
    if (target == null || _siteId.isNotEmpty) return;
    final bc = ref.read(bcApiProvider);
    try {
      final sites = await bc.sitesList();
      for (final s in sites) {
        final pages = await bc.pagesList(siteId: s.siteId).catchError(
              (_) => <PageRow>[],
            );
        if (pages.any((p) => p.pageId == target)) {
          if (mounted) setState(() => _siteId = s.siteId);
          return;
        }
      }
    } catch (_) {}
  }

  Future<void> _loadPages() async {
    final siteId = _siteId;
    if (siteId.isEmpty) {
      if (mounted) setState(() => _pages = const []);
      return;
    }
    try {
      final pages = await ref.read(bcApiProvider).pagesList(siteId: siteId);
      if (mounted) setState(() => _pages = pages);
    } catch (_) {}
  }

  Future<void> _loadWidgets() async {
    final pageId = _pageId;
    if (pageId.isEmpty) {
      if (mounted) setState(() => _widgets = const []);
      return;
    }
    try {
      final widgets = await ref.read(bcApiProvider).widgetsByPage(pageId);
      if (mounted) setState(() => _widgets = widgets);
    } catch (_) {}
  }

  void _onSiteChanged(String v) {
    setState(() {
      _siteId = v;
      _pageId = '';
    });
    _loadPages();
    _loadWidgets();
  }

  void _onPageChanged(String v) {
    setState(() => _pageId = v);
    _loadWidgets();
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final refresh = ref.watch(refreshProvider);
    if (refresh != _lastRefresh) {
      _lastRefresh = refresh;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _loadSites();
        _discoverSite();
        _loadPages();
        _loadWidgets();
      });
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(20, 80, 20, 128),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const PageHeader(eyebrow: 'Client view', title: 'Page preview'),
          Field(
            label: 'Site',
            child: GlassPicker(
              value: _siteId,
              placeholder: 'Choose a site',
              options: [
                for (final s in _sites)
                  PickerOption(value: s.siteId, label: s.name),
              ],
              onChanged: _onSiteChanged,
            ),
          ),
          if (_sitesError != null) ...[
            const SizedBox(height: 8),
            Row(
              children: [
                const Icon(LucideIcons.alertTriangle,
                    size: 14, color: RubixTokens.fault),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    'Could not load sites: $_sitesError',
                    style: const TextStyle(
                        color: RubixTokens.fault, fontSize: 12),
                  ),
                ),
              ],
            ),
          ],
          if (_siteId.isNotEmpty) ...[
            const SizedBox(height: 16),
            Field(
              label: 'Dashboard page',
              child: GlassPicker(
                value: _pageId,
                placeholder: 'Choose a page',
                options: [
                  for (final p in _pages)
                    PickerOption(value: p.pageId, label: p.name),
                ],
                onChanged: _onPageChanged,
              ),
            ),
          ],
          const SizedBox(height: 24),
          if (_pageId.isEmpty)
            const _Empty(
              icon: LucideIcons.layoutDashboard,
              text: 'Pick a site and a page to see its sensor tiles.',
            )
          else if (_widgets.isEmpty)
            const _Empty(
              icon: LucideIcons.inbox,
              text: 'No widgets on this page yet. '
                  'Provision a device to populate it.',
            )
          else
            _TileGrid(widgets: _widgets, accent: look.accent),
        ],
      ),
    );
  }
}

/// Lays out the tiles in a 2-column grid where `gauge`/`line` widgets span the
/// full width. Full-span tiles get their own row; the rest pair up.
class _TileGrid extends StatelessWidget {
  const _TileGrid({required this.widgets, required this.accent});

  final List<WidgetRow> widgets;
  final Color accent;

  static bool _fullSpan(String w) => w == 'gauge' || w == 'line';

  @override
  Widget build(BuildContext context) {
    const gutter = 12.0;
    final rows = <Widget>[];
    var i = 0;
    while (i < widgets.length) {
      final w = widgets[i];
      if (_fullSpan(w.widget)) {
        rows.add(_tile(w, i));
        i += 1;
        continue;
      }
      // Pair this half-width tile with the next half-width tile (if any).
      final next =
          (i + 1 < widgets.length && !_fullSpan(widgets[i + 1].widget))
              ? i + 1
              : -1;
      rows.add(
        Row(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Expanded(child: _tile(w, i)),
            const SizedBox(width: gutter),
            Expanded(
              child: next >= 0
                  ? _tile(widgets[next], next)
                  : const SizedBox.shrink(),
            ),
          ],
        ),
      );
      i += next >= 0 ? 2 : 1;
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (var r = 0; r < rows.length; r++) ...[
          if (r > 0) const SizedBox(height: gutter),
          rows[r],
        ],
      ],
    );
  }

  Widget _tile(WidgetRow w, int index) {
    return TweenAnimationBuilder<double>(
      key: ValueKey(w.widgetId),
      tween: Tween(begin: 0, end: 1),
      duration: Duration(milliseconds: 300 + index * 50),
      curve: Curves.easeOut,
      builder: (context, t, child) => Opacity(
        opacity: t.clamp(0.0, 1.0),
        child: Transform.translate(
          offset: Offset(0, 16 * (1 - t)),
          child: child,
        ),
      ),
      child: WidgetTile(
        widget: w.widget,
        title: w.title ?? w.role ?? 'Point ${index + 1}',
        accent: accent,
        seed: index + 1,
      ),
    );
  }
}

/// Empty-state glass panel — centered icon + caption.
class _Empty extends StatelessWidget {
  const _Empty({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    return GlassSurface(
      borderRadius: BorderRadius.circular(RubixTokens.radius2xl),
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 48),
      child: Column(
        children: [
          Icon(icon, size: 32, color: RubixTokens.inkMuted),
          const SizedBox(height: 12),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 240),
            child: Text(
              text,
              textAlign: TextAlign.center,
              style: const TextStyle(
                fontSize: 14,
                color: RubixTokens.inkMuted,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
