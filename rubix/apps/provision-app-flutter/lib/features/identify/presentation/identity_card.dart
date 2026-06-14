import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';
import 'package:provision_app/shared/widgets/glass.dart';

/// The decoded device card: model · network · serial, template icon, and a
/// preview of the template's points. Shown right after `bc_decode`. Ported from
/// the React `IdentityCard.tsx`.
class IdentityCard extends ConsumerWidget {
  const IdentityCard({required this.identity, super.key});

  final ScannedIdentity identity;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final look = ref.watch(lookProvider);
    final t = identity.template;

    return GlassSurface(
      borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
      boxShadow: GlassShadow.glass,
      padding: const EdgeInsets.all(20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Container(
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  color: look.accent.withValues(alpha: 0.13),
                  borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
                ),
                child: Icon(LucideIcons.cpu, size: 24, color: look.accent),
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
                      style: const TextStyle(
                        color: RubixTokens.ink,
                        fontSize: 18,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                    Text(
                      identity.model,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: const TextStyle(
                        color: RubixTokens.inkVariant,
                        fontSize: 14,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
          const SizedBox(height: 16),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _Meta(icon: LucideIcons.network, text: identity.network),
              _Meta(icon: LucideIcons.hash, text: identity.id),
            ],
          ),
          const SizedBox(height: 20),
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text('Points · ${t.points.length}', style: RubixText.label),
          ),
          for (final p in t.points)
            Padding(
              padding: const EdgeInsets.only(bottom: 6),
              child: Container(
                padding:
                    const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
                decoration: BoxDecoration(
                  color: Colors.white.withValues(alpha: 0.04),
                  borderRadius: BorderRadius.circular(RubixTokens.radiusSm),
                ),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Expanded(
                      child: Text(
                        p.name,
                        style: const TextStyle(
                          color: RubixTokens.ink,
                          fontSize: 14,
                        ),
                      ),
                    ),
                    Text(p.widget.toUpperCase(), style: RubixText.label),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _Meta extends StatelessWidget {
  const _Meta({required this.icon, required this.text});
  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.06),
        borderRadius: BorderRadius.circular(999),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 14, color: RubixTokens.inkVariant),
          const SizedBox(width: 6),
          Text(
            text,
            style: const TextStyle(
              color: RubixTokens.inkVariant,
              fontSize: 12,
              fontWeight: FontWeight.w600,
            ),
          ),
        ],
      ),
    );
  }
}
