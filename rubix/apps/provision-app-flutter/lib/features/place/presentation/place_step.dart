import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/ids.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/place/domain/placement.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';

/// Shared placement step: Site (or + create inline), Location (or + new),
/// Page (scoped to the chosen site, + new). The page picker only appears once a
/// site is chosen — a page belongs to a site. Ported from the React
/// `Place.tsx`.
class PlaceStep extends ConsumerStatefulWidget {
  const PlaceStep({required this.value, required this.onChanged, super.key});

  final Placement value;
  final ValueChanged<Placement> onChanged;

  @override
  ConsumerState<PlaceStep> createState() => _PlaceStepState();
}

class _PlaceStepState extends ConsumerState<PlaceStep> {
  List<SiteRow> _sites = const [];
  List<LocationRow> _locations = const [];
  List<PageRow> _pages = const [];

  void _set(Placement patch) => widget.onChanged(patch);

  Future<void> _loadSites() async {
    try {
      final list = await ref.read(bcApiProvider).sitesList();
      if (mounted) setState(() => _sites = list);
    } catch (_) {/* surfaced elsewhere */}
  }

  Future<void> _loadScoped(String siteId) async {
    final bc = ref.read(bcApiProvider);
    if (siteId.isEmpty) {
      if (mounted) {
        setState(() {
          _locations = const [];
          _pages = const [];
        });
      }
      return;
    }
    try {
      final locations = await bc.locationsList(siteId: siteId);
      if (mounted) setState(() => _locations = locations);
    } catch (_) {}
    try {
      final pages = await bc.pagesList(siteId: siteId);
      if (mounted) setState(() => _pages = pages);
    } catch (_) {}
  }

  Future<void> _createSite(String name) async {
    final nm = name.trim();
    if (nm.isEmpty) return;
    final id = mintId('site');
    final bc = ref.read(bcApiProvider);
    await bc.siteCreate(siteId: id, name: nm);
    // Refetch the authoritative list FIRST so the new option exists before we
    // select it (a picker won't hold a value not in its options).
    List<SiteRow> list;
    try {
      list = await bc.sitesList();
    } catch (_) {
      list = [SiteRow(siteId: id, name: nm)];
    }
    if (list.every((s) => s.siteId != id)) {
      list = [...list, SiteRow(siteId: id, name: nm)];
    }
    if (!mounted) return;
    setState(() => _sites = list);
    _set(Placement.empty.copyWith(siteId: id));
  }

  @override
  Widget build(BuildContext context) {
    // Re-fetch on any mutation, and reload scoped lists when the site changes.
    ref.watch(refreshProvider);
    final value = widget.value;
    final look = ref.watch(lookProvider);

    // Kick off loads after build; cheap + fresh-deduped.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _loadSites();
      _loadScoped(value.siteId);
    });

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Field(
          label: 'Site',
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              GlassPicker(
                value: value.siteId,
                placeholder: _sites.isNotEmpty
                    ? '+ Choose or create a site'
                    : '+ Create your first site',
                options: [
                  for (final s in _sites)
                    PickerOption(value: s.siteId, label: s.name),
                ],
                onChanged: (v) => _set(Placement.empty.copyWith(siteId: v)),
              ),
              if (value.siteId.isEmpty) ...[
                const SizedBox(height: 8),
                _CreateSiteRow(accent: look.accent, onCreate: _createSite),
              ],
            ],
          ),
        ),
        if (value.siteId.isNotEmpty) ...[
          const SizedBox(height: 16),
          Field(
            label: 'Location',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                GlassPicker(
                  value: value.locationId,
                  placeholder: '+ New location',
                  options: [
                    for (final l in _locations)
                      PickerOption(value: l.locationId, label: l.name),
                  ],
                  onChanged: (v) =>
                      _set(value.copyWith(locationId: v, newLocation: '')),
                ),
                if (value.locationId.isEmpty) ...[
                  const SizedBox(height: 8),
                  GlassTextField(
                    value: value.newLocation,
                    onChanged: (v) => _set(value.copyWith(newLocation: v)),
                    hint: 'New location name (e.g. Level 3 — North)',
                  ),
                ],
              ],
            ),
          ),
          const SizedBox(height: 16),
          Field(
            label: 'Dashboard page',
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                GlassPicker(
                  value: value.pageId,
                  placeholder: '+ New page for this site',
                  options: [
                    for (final p in _pages)
                      PickerOption(value: p.pageId, label: p.name),
                  ],
                  onChanged: (v) =>
                      _set(value.copyWith(pageId: v, newPage: '')),
                ),
                if (value.pageId.isEmpty) ...[
                  const SizedBox(height: 8),
                  GlassTextField(
                    value: value.newPage,
                    onChanged: (v) => _set(value.copyWith(newPage: v)),
                    hint: 'New page name (e.g. Floor 3 dashboard)',
                  ),
                ],
              ],
            ),
          ),
        ],
      ],
    );
  }
}

/// Inline "+ create site" control: a name field + a small accent square button.
class _CreateSiteRow extends StatefulWidget {
  const _CreateSiteRow({required this.accent, required this.onCreate});

  final Color accent;
  final Future<void> Function(String name) onCreate;

  @override
  State<_CreateSiteRow> createState() => _CreateSiteRowState();
}

class _CreateSiteRowState extends State<_CreateSiteRow> {
  final _controller = TextEditingController();
  bool _creating = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  Future<void> _create() async {
    if (_creating || _controller.text.trim().isEmpty) return;
    setState(() => _creating = true);
    try {
      await widget.onCreate(_controller.text);
      _controller.clear();
    } finally {
      if (mounted) setState(() => _creating = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final canCreate = _controller.text.trim().isNotEmpty && !_creating;
    return Row(
      children: [
        Expanded(
          child: GlassTextField(
            controller: _controller,
            onChanged: (_) => setState(() {}),
            onSubmitted: _create,
            hint: 'New site name (e.g. Building A)',
            semanticLabel: 'New site name',
          ),
        ),
        const SizedBox(width: 8),
        GestureDetector(
          onTap: canCreate ? _create : null,
          child: Opacity(
            opacity: canCreate ? 1 : 0.4,
            child: Container(
              width: 48,
              height: 48,
              decoration: BoxDecoration(
                color: widget.accent,
                borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
              ),
              child: _creating
                  ? const Padding(
                      padding: EdgeInsets.all(14),
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        color: RubixTokens.primaryOn,
                      ),
                    )
                  : const Icon(
                      LucideIcons.plus,
                      size: 20,
                      color: RubixTokens.primaryOn,
                    ),
            ),
          ),
        ),
      ],
    );
  }
}
