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

/// Show the YAML editor for a template — loads the current YAML and upserts on
/// save. Ported from the React Templates edit `BottomSheet`.
Future<void> showTemplateEditSheet(
  BuildContext context,
  WidgetRef ref,
  TemplateRow template,
) {
  return showGlassBottomSheet<void>(
    context: context,
    title: 'Edit · ${template.template}',
    builder: (_) => _TemplateEditBody(template: template),
  );
}

class _TemplateEditBody extends ConsumerStatefulWidget {
  const _TemplateEditBody({required this.template});
  final TemplateRow template;

  @override
  ConsumerState<_TemplateEditBody> createState() => _TemplateEditBodyState();
}

class _TemplateEditBodyState extends ConsumerState<_TemplateEditBody> {
  final _yaml = TextEditingController();
  bool _loading = true;
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _yaml.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final toast = ref.read(toastProvider.notifier);
    try {
      final res = await ref
          .read(bcApiProvider)
          .templateYaml(widget.template.template);
      if (!mounted) return;
      setState(() {
        _yaml.text = res.isNotEmpty ? res.first.yaml : '';
        _loading = false;
      });
    } catch (e) {
      if (mounted) setState(() => _loading = false);
      toast.show(e.toString(), accent: RubixTokens.fault);
    }
  }

  Future<void> _save() async {
    if (_saving || _yaml.text.trim().isEmpty) return;
    setState(() => _saving = true);
    final look = ref.read(lookProvider);
    final toast = ref.read(toastProvider.notifier);
    try {
      await ref.read(bcApiProvider).templateUpsert(_yaml.text);
      toast.show('Template saved', accent: look.accent);
      if (mounted) Navigator.of(context).pop();
    } catch (e) {
      toast.show(e.toString(), accent: RubixTokens.fault);
      if (mounted) setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);

    if (_loading) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 32),
        child: Row(
          children: [
            SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(
                  strokeWidth: 2, color: look.inkMuted),
            ),
            const SizedBox(width: 8),
            Text(
              'Loading YAML…',
              style: TextStyle(color: look.inkMuted, fontSize: 14),
            ),
          ],
        ),
      );
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        SizedBox(
          height: 256,
          child: GlassTextField(
            controller: _yaml,
            maxLines: 1000,
            monospace: true,
            semanticLabel: 'Template YAML',
            onChanged: (_) => setState(() {}),
          ),
        ),
        const SizedBox(height: 16),
        PrimaryButton(
          label: _saving ? 'Saving…' : 'Save template',
          icon: _saving ? null : LucideIcons.save,
          accent: look.accent,
          loading: _saving,
          onPressed: _yaml.text.trim().isEmpty ? null : _save,
        ),
      ],
    );
  }
}
