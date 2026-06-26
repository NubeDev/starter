import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/shared/widgets/bottom_sheet.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/primary_button.dart';
import 'package:provision_app/shared/widgets/toast.dart';

/// Show the place-on-page sheet for a pending (or pageless) device. Pick an
/// existing page scoped to the device's site, or type a new page name →
/// `bc.assignPage`, which generates the widgets and flips status to
/// provisioned. Closes + refreshes (assignPage bumps the shared refresh signal)
/// on success. Ported from the React `PlaceOnPageSheet`.
Future<void> showPlaceOnPageSheet(
  BuildContext context,
  WidgetRef ref,
  DeviceRow device,
) {
  return showGlassBottomSheet<void>(
    context: context,
    title: 'Place on page',
    builder: (_) => _PlaceOnPageBody(device: device),
  );
}

class _PlaceOnPageBody extends ConsumerStatefulWidget {
  const _PlaceOnPageBody({required this.device});
  final DeviceRow device;

  @override
  ConsumerState<_PlaceOnPageBody> createState() => _PlaceOnPageBodyState();
}

class _PlaceOnPageBodyState extends ConsumerState<_PlaceOnPageBody> {
  final _newPage = TextEditingController();
  List<PageRow> _pages = const [];
  String _pageId = '';
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _newPage.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    try {
      final list = await ref
          .read(bcApiProvider)
          .pagesList(siteId: widget.device.siteId);
      if (!mounted) return;
      setState(() {
        _pages = list;
        _pageId = '';
        _newPage.clear();
      });
    } catch (_) {
      if (mounted) setState(() => _pages = const []);
    }
  }

  bool get _ready => _pageId.isNotEmpty || _newPage.text.trim().isNotEmpty;

  Future<void> _submit() async {
    if (!_ready || _busy) return;
    setState(() => _busy = true);
    final look = ref.read(lookProvider);
    final toast = ref.read(toastProvider.notifier);
    try {
      final r = await ref.read(bcApiProvider).assignPage(
            deviceId: widget.device.deviceId,
            pageId: _pageId.isNotEmpty ? _pageId : null,
            newPage: _pageId.isEmpty ? _newPage.text.trim() : null,
          );
      final tiles = '${r.widgets} tile${r.widgets == 1 ? '' : 's'}';
      toast.show('Placed — $tiles', accent: look.accent);
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final device = widget.device;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Row(
          children: [
            Icon(LucideIcons.layoutDashboard, size: 16, color: look.accent),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                device.name ?? device.deviceId,
                style: TextStyle(
                  color: look.inkSoft,
                  fontSize: 14,
                ),
              ),
            ),
          ],
        ),
        const SizedBox(height: 16),
        Field(
          label: 'Dashboard page',
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              GlassPicker(
                value: _pageId,
                placeholder: _pages.isNotEmpty
                    ? '+ New page for this site'
                    : '+ Create a page',
                options: [
                  for (final p in _pages)
                    PickerOption(value: p.pageId, label: p.name),
                ],
                onChanged: (v) => setState(() {
                  _pageId = v;
                  _newPage.clear();
                }),
              ),
              if (_pageId.isEmpty) ...[
                const SizedBox(height: 8),
                GlassTextField(
                  controller: _newPage,
                  hint: 'New page name (e.g. Floor 3 dashboard)',
                  semanticLabel: 'New page name',
                  onChanged: (_) => setState(() {}),
                  onSubmitted: _submit,
                ),
              ],
            ],
          ),
        ),
        const SizedBox(height: 16),
        PrimaryButton(
          label: _busy ? 'Placing…' : 'Place device',
          accent: look.accent,
          loading: _busy,
          onPressed: _ready ? _submit : null,
        ),
      ],
    );
  }
}
