import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/ids.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/scan/domain/build_add_url.dart';
import 'package:provision_app/features/scan/presentation/qr_label.dart';
import 'package:provision_app/shared/widgets/bottom_sheet.dart';
import 'package:provision_app/shared/widgets/form_kit.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// Show the QR-sticker generator for a template — type/mint a device id and an
/// optional address, then render a scannable add-device QR. Ported from the
/// React Templates QR `BottomSheet`.
Future<void> showTemplateQrSheet(
  BuildContext context,
  WidgetRef ref,
  TemplateRow template,
) {
  return showGlassBottomSheet<void>(
    context: context,
    title: 'QR sticker · ${template.displayName}',
    builder: (_) => _TemplateQrBody(template: template),
  );
}

class _TemplateQrBody extends ConsumerStatefulWidget {
  const _TemplateQrBody({required this.template});
  final TemplateRow template;

  @override
  ConsumerState<_TemplateQrBody> createState() => _TemplateQrBodyState();
}

class _TemplateQrBodyState extends ConsumerState<_TemplateQrBody> {
  final _id = TextEditingController();
  final _addr = TextEditingController();

  String get _prefix {
    final t = widget.template.template;
    return t.substring(0, t.length.clamp(0, 3)).toUpperCase();
  }

  @override
  void dispose() {
    _id.dispose();
    _addr.dispose();
    super.dispose();
  }

  void _mint() {
    setState(() => _id.text = mintId(_prefix).replaceAll('_', '-'));
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final t = widget.template;
    final id = _id.text.trim();

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Field(
          label: 'Device ID / serial',
          child: Row(
            children: [
              Expanded(
                child: GlassTextField(
                  controller: _id,
                  hint: '$_prefix-0001',
                  semanticLabel: 'Device ID',
                  onChanged: (_) => setState(() {}),
                ),
              ),
              const SizedBox(width: 8),
              Pressable(
                scale: 0.94,
                semanticLabel: 'Mint an ID',
                onTap: _mint,
                child: GlassSurface(
                  borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
                  child: SizedBox(
                    width: 48,
                    height: 48,
                    child: Icon(
                      LucideIcons.wand2,
                      size: 20,
                      color: look.accent,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
        const SizedBox(height: 16),
        Field(
          label: '${addressLabel(t.network)} (optional)',
          child: GlassTextField(
            controller: _addr,
            hint: t.network == 'lora' ? '70B3D5499F2C18' : '192.168.15.42',
            semanticLabel: addressLabel(t.network),
            onChanged: (_) => setState(() {}),
          ),
        ),
        const SizedBox(height: 16),
        if (id.isNotEmpty)
          QrLabel(
            value: buildAddUrl(
              id: id,
              model: t.template,
              network: t.network,
              address: _addr.text.trim(),
            ),
            title: t.displayName,
            subtitle: id,
            caption: buildAddUrl(
              id: id,
              model: t.template,
              network: t.network,
              address: _addr.text.trim(),
            ),
          )
        else
          const Padding(
            padding: EdgeInsets.symmetric(vertical: 24),
            child: Row(
              children: [
                Icon(LucideIcons.qrCode, size: 16, color: RubixTokens.inkMuted),
                SizedBox(width: 8),
                Expanded(
                  child: Text(
                    'Enter or mint an ID to generate the QR.',
                    style: TextStyle(color: RubixTokens.inkMuted, fontSize: 14),
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }
}
