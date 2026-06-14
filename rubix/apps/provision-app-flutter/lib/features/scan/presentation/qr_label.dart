import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/shared/widgets/glass.dart';
import 'package:provision_app/shared/widgets/pressable.dart';
import 'package:provision_app/shared/widgets/toast.dart';
import 'package:qr_flutter/qr_flutter.dart';

/// A scannable QR sticker + a "Print" button. [value] is the QR *payload* (a
/// `rubix://add?…` string), NOT an image URL — encode it so the printed sticker
/// scans back through `bc_decode`. Shared by the device label sheet and the
/// Templates QR generator so every sticker looks identical. Ported from the
/// React `QrLabel.tsx`; printing is a platform-stubbed no-op (a browser print
/// window doesn't translate to Flutter).
class QrLabel extends ConsumerWidget {
  const QrLabel({
    required this.value,
    this.title,
    this.subtitle,
    this.caption,
    super.key,
  });

  final String value;
  final String? title;
  final String? subtitle;
  final String? caption;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = ref.watch(lookProvider);

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Column(
          children: [
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: Colors.white,
                borderRadius: BorderRadius.circular(RubixTokens.radius),
              ),
              child: QrImageView(
                data: value,
                size: 160,
                padding: EdgeInsets.zero,
                errorCorrectionLevel: QrErrorCorrectLevel.M,
                semanticsLabel: 'QR code for ${subtitle ?? title ?? value}',
              ),
            ),
            if (title != null) ...[
              const SizedBox(height: 8),
              Text(
                title!,
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: RubixTokens.ink,
                  fontSize: 18,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ],
            if (subtitle != null) ...[
              const SizedBox(height: 8),
              Text(
                subtitle!,
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: RubixTokens.inkVariant,
                  fontSize: 14,
                  fontFamily: 'monospace',
                ),
              ),
            ],
            if (caption != null) ...[
              const SizedBox(height: 8),
              Text(
                caption!,
                textAlign: TextAlign.center,
                style: const TextStyle(
                  color: RubixTokens.inkMuted,
                  fontSize: 12,
                  fontFamily: 'monospace',
                ),
              ),
            ],
          ],
        ),
        const SizedBox(height: 16),
        Pressable(
          onTap: () => ref.read(toastProvider.notifier).show(
                'Printing not supported on this platform yet',
              ),
          child: GlassSurface(
            borderRadius: BorderRadius.circular(RubixTokens.radius),
            padding: const EdgeInsets.symmetric(vertical: 12),
            child: Row(
              mainAxisAlignment: MainAxisAlignment.center,
              children: [
                Icon(LucideIcons.printer, size: 16, color: look.accent),
                const SizedBox(width: 8),
                const Text(
                  'Print sticker',
                  style: TextStyle(
                    color: RubixTokens.ink,
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
