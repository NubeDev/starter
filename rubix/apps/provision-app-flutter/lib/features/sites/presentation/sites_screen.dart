import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/ids.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/shared/widgets/async_list_view.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/glass_card.dart';
import 'package:provision_app/shared/widgets/page_header.dart';
import 'package:provision_app/shared/widgets/pressable.dart';
import 'package:provision_app/shared/widgets/toast.dart';

/// Site + location tree with inline create at both levels. Ported from the
/// React `Sites`.
class SitesScreen extends ConsumerStatefulWidget {
  const SitesScreen({super.key});

  @override
  ConsumerState<SitesScreen> createState() => _SitesScreenState();
}

class _SitesScreenState extends ConsumerState<SitesScreen> {
  final _newSite = TextEditingController();
  List<SiteRow> _sites = const [];
  bool _creating = false;
  int _loadedFor = -1;
  ListPhase _phase = ListPhase.loading;
  String? _error;

  Future<void> _load() async {
    setState(() {
      _phase = ListPhase.loading;
      _error = null;
    });
    try {
      final list = await ref.read(bcApiProvider).sitesList();
      if (!mounted) return;
      setState(() {
        _sites = list;
        _phase = list.isEmpty ? ListPhase.empty : ListPhase.data;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _phase = ListPhase.error;
        _error = e.toString();
      });
    }
  }

  Future<void> _reload() {
    _loadedFor = ref.read(refreshProvider);
    return _load();
  }

  @override
  void dispose() {
    _newSite.dispose();
    super.dispose();
  }

  Future<void> _addSite() async {
    final nm = _newSite.text.trim();
    if (nm.isEmpty || _creating) return;
    setState(() => _creating = true);
    final look = ref.read(lookProvider);
    final toast = ref.read(toastProvider.notifier);
    try {
      await ref
          .read(bcApiProvider)
          .siteCreate(siteId: mintId('site'), name: nm);
      _newSite.clear();
      toast.show('Site created', accent: look.accent);
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
    } finally {
      if (mounted) setState(() => _creating = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final refresh = ref.watch(refreshProvider);
    if (refresh != _loadedFor) {
      _loadedFor = refresh;
      _load();
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.fromLTRB(20, 80, 20, 128),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const PageHeader(eyebrow: 'Topology', title: 'Sites'),
          GlassSurface(
            borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
            padding: const EdgeInsets.fromLTRB(12, 6, 6, 6),
            child: Row(
              children: [
                const Icon(LucideIcons.building2,
                    size: 16, color: RubixTokens.inkMuted),
                const SizedBox(width: 8),
                Expanded(
                  child: TextField(
                    controller: _newSite,
                    onChanged: (_) => setState(() {}),
                    onSubmitted: (_) => _addSite(),
                    style: const TextStyle(
                        color: RubixTokens.ink, fontSize: 16),
                    cursorColor: RubixTokens.primary,
                    decoration: const InputDecoration(
                      isDense: true,
                      hintText: 'New site name',
                      hintStyle: TextStyle(color: RubixTokens.inkMuted),
                      border: InputBorder.none,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                _IconButton(
                  icon: _creating ? null : LucideIcons.plus,
                  loading: _creating,
                  background: look.accent,
                  foreground: RubixTokens.primaryOn,
                  enabled: _newSite.text.trim().isNotEmpty && !_creating,
                  semanticLabel: 'Create site',
                  onTap: _addSite,
                ),
              ],
            ),
          ),
          const SizedBox(height: 20),
          AsyncListView(
            phase: _phase,
            error: _error,
            emptyText: 'No sites yet. Add one above to get started.',
            emptyIcon: LucideIcons.building2,
            onRetry: _reload,
            child: Column(
              children: [
                for (final s in _sites) ...[
                  _SiteNode(site: s),
                  const SizedBox(height: 12),
                ],
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SiteNode extends ConsumerStatefulWidget {
  const _SiteNode({required this.site});
  final SiteRow site;

  @override
  ConsumerState<_SiteNode> createState() => _SiteNodeState();
}

class _SiteNodeState extends ConsumerState<_SiteNode> {
  final _newLoc = TextEditingController();
  List<LocationRow> _locations = const [];
  int _loadedFor = -1;

  Future<void> _load() async {
    try {
      final list = await ref
          .read(bcApiProvider)
          .locationsList(siteId: widget.site.siteId);
      if (mounted) setState(() => _locations = list);
    } catch (_) {/* ignore */}
  }

  @override
  void dispose() {
    _newLoc.dispose();
    super.dispose();
  }

  Future<void> _addLoc() async {
    final nm = _newLoc.text.trim();
    if (nm.isEmpty) return;
    final look = ref.read(lookProvider);
    final toast = ref.read(toastProvider.notifier);
    try {
      await ref.read(bcApiProvider).locationCreate(
            locationId: mintId('loc'),
            siteId: widget.site.siteId,
            name: nm,
          );
      _newLoc.clear();
      toast.show('Location added', accent: look.accent);
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
    }
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final refresh = ref.watch(refreshProvider);
    if (refresh != _loadedFor) {
      _loadedFor = refresh;
      _load();
    }

    return GlassCard(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              Icon(LucideIcons.building2, size: 20, color: look.accent),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  widget.site.name,
                  style: const TextStyle(
                    color: RubixTokens.ink,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            ],
          ),
          const SizedBox(height: 12),
          for (final l in _locations)
            Padding(
              padding: const EdgeInsets.only(left: 8, bottom: 6),
              child: Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: BoxDecoration(
                  color: const Color(0x0AFFFFFF),
                  borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
                ),
                child: Row(
                  children: [
                    const Icon(LucideIcons.mapPin,
                        size: 16, color: RubixTokens.inkMuted),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        l.name,
                        style: const TextStyle(
                            color: RubixTokens.ink, fontSize: 14),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          Padding(
            padding: const EdgeInsets.only(left: 8, top: 2),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _newLoc,
                    onChanged: (_) => setState(() {}),
                    onSubmitted: (_) => _addLoc(),
                    style: const TextStyle(
                        color: RubixTokens.ink, fontSize: 14),
                    cursorColor: RubixTokens.primary,
                    decoration: const InputDecoration(
                      isDense: true,
                      hintText: '+ New location',
                      hintStyle: TextStyle(color: RubixTokens.inkMuted),
                      border: InputBorder.none,
                    ),
                  ),
                ),
                const SizedBox(width: 8),
                _IconButton(
                  icon: LucideIcons.plus,
                  background: const Color(0x14FFFFFF),
                  foreground: RubixTokens.ink,
                  enabled: _newLoc.text.trim().isNotEmpty,
                  semanticLabel: 'Add location',
                  onTap: _addLoc,
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _IconButton extends StatelessWidget {
  const _IconButton({
    required this.background,
    required this.foreground,
    required this.enabled,
    required this.semanticLabel,
    required this.onTap,
    this.icon,
    this.loading = false,
  });

  final IconData? icon;
  final Color background;
  final Color foreground;
  final bool enabled;
  final bool loading;
  final String semanticLabel;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Pressable(
      scale: 0.94,
      semanticLabel: semanticLabel,
      onTap: enabled ? onTap : null,
      child: Opacity(
        opacity: enabled ? 1 : 0.4,
        child: Container(
          width: 40,
          height: 40,
          decoration: BoxDecoration(
            color: background,
            borderRadius: BorderRadius.circular(RubixTokens.radius),
          ),
          child: loading
              ? Padding(
                  padding: const EdgeInsets.all(10),
                  child: CircularProgressIndicator(
                      strokeWidth: 2, color: foreground),
                )
              : Icon(icon, size: 20, color: foreground),
        ),
      ),
    );
  }
}
