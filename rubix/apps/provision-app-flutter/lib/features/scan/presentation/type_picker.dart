import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/ids.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/scan/domain/build_add_url.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/primary_button.dart';

/// "Pick a device type" path — no barcode needed. Choose a template, we mint a
/// serial and synthesise the canonical `rubix://add?…` string, then run the
/// same `bc_decode` → place → provision flow. Ported from the React
/// `TypePicker.tsx`.
class TypePicker extends ConsumerStatefulWidget {
  const TypePicker({required this.onSynthesized, super.key});

  final ValueChanged<String> onSynthesized;

  @override
  ConsumerState<TypePicker> createState() => _TypePickerState();
}

class _TypePickerState extends ConsumerState<TypePicker> {
  List<TemplateRow> _templates = const [];
  String _chosen = '';
  int _loadedFor = -1;

  void _load() {
    ref.read(bcApiProvider).templatesList().then((rows) {
      if (mounted) setState(() => _templates = rows);
    }).catchError((_) {});
  }

  void _go() {
    final t = _templates.where((x) => x.template == _chosen).firstOrNull;
    if (t == null) return;
    final prefix = t.template
        .substring(0, t.template.length < 3 ? t.template.length : 3)
        .toUpperCase();
    final serial = mintId(prefix).replaceAll('_', '-');
    widget.onSynthesized(
      buildAddUrl(id: serial, model: t.template, network: t.network),
    );
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    // Re-fetch on mutations, mirroring the React `useRefreshKey()` effect dep.
    final refresh = ref.watch(refreshProvider);
    if (refresh != _loadedFor) {
      _loadedFor = refresh;
      _load();
    }

    return GlassSurface(
      borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Field(
            label: 'Device type',
            child: GlassPicker(
              value: _chosen,
              placeholder: 'Select a template',
              options: [
                for (final t in _templates)
                  PickerOption(value: t.template, label: t.displayName),
              ],
              onChanged: (v) => setState(() => _chosen = v),
            ),
          ),
          const SizedBox(height: 12),
          PrimaryButton(
            label: 'Use this type',
            accent: look.accent,
            onPressed: _chosen.isEmpty ? null : _go,
          ),
        ],
      ),
    );
  }
}
