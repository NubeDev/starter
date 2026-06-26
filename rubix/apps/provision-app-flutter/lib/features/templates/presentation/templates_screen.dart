import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_api.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/api/refresh.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/features/templates/presentation/template_edit_sheet.dart';
import 'package:provision_app/features/templates/presentation/template_qr_sheet.dart';
import 'package:provision_app/shared/widgets/glass_card.dart';
import 'package:provision_app/shared/widgets/page_header.dart';
import 'package:provision_app/shared/widgets/refreshable.dart';
import 'package:provision_app/shared/widgets/pressable.dart';

/// List YAML templates; tap to view/edit the YAML and upsert, or tap the QR
/// button to mint an add-device sticker. Ported from the React `Templates`.
class TemplatesScreen extends ConsumerStatefulWidget {
  const TemplatesScreen({super.key});

  @override
  ConsumerState<TemplatesScreen> createState() => _TemplatesScreenState();
}

class _TemplatesScreenState extends ConsumerState<TemplatesScreen> {
  List<TemplateRow> _rows = const [];
  int _loadedFor = -1;

  Future<void> _load() async {
    try {
      final list = await ref.read(bcApiProvider).templatesList();
      if (mounted) setState(() => _rows = list);
    } catch (_) {
      /* ignore */
    }
  }

  @override
  Widget build(BuildContext context) {
    final refresh = ref.watch(refreshProvider);
    if (refresh != _loadedFor) {
      _loadedFor = refresh;
      _load();
    }

    return Refreshable(
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        padding: const EdgeInsets.fromLTRB(20, 80, 20, 128),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const PageHeader(eyebrow: 'Catalog', title: 'Templates'),
            for (final t in _rows) ...[
              _TemplateCard(template: t),
              const SizedBox(height: 8),
            ],
          ],
        ),
      ),
    );
  }
}

class _TemplateCard extends ConsumerWidget {
  const _TemplateCard({required this.template});
  final TemplateRow template;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = ref.watch(lookProvider);
    final t = template;

    return GlassCard(
      onTap: () => showTemplateEditSheet(context, ref, t),
      child: Row(
        children: [
          Container(
            width: 40,
            height: 40,
            decoration: BoxDecoration(
              color: const Color(0x0FFFFFFF),
              borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
            ),
            child: Icon(LucideIcons.fileCode2, size: 20, color: look.inkSoft),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  t.displayName,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(
                    color: look.ink,
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  '${t.category} · ${t.network} · v${t.version}',
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(color: look.inkMuted, fontSize: 12),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),
          Pressable(
            scale: 0.92,
            semanticLabel: 'Make a QR sticker for ${t.displayName}',
            onTap: () => showTemplateQrSheet(context, ref, t),
            child: Container(
              width: 36,
              height: 36,
              decoration: const BoxDecoration(
                color: Color(0x0FFFFFFF),
                shape: BoxShape.circle,
              ),
              child: Icon(LucideIcons.qrCode, size: 20, color: look.accent),
            ),
          ),
          const SizedBox(width: 8),
          Icon(LucideIcons.chevronRight, size: 20, color: look.inkMuted),
        ],
      ),
    );
  }
}
