import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Glass header with logo, avatar, and optional hero text.
class GlassHeader extends StatelessWidget {
  final String? heroText;
  final Widget? leading;
  final Widget? trailing;
  final EdgeInsetsGeometry padding;
  final double borderRadius;
  final double height;

  const GlassHeader({
    Key? key,
    this.heroText,
    this.leading,
    this.trailing,
    this.padding = const EdgeInsets.symmetric(horizontal: 24, vertical: 20),
    this.borderRadius = 24,
    this.height = 120,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      height: height,
      padding: padding,
      decoration: BoxDecoration(
        color: t.surface.withOpacity(0.7),
        borderRadius: BorderRadius.circular(borderRadius),
        border: Border.all(color: t.border.withOpacity(0.18)),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.08),
            blurRadius: 24,
            offset: const Offset(0, 8),
          ),
        ],
        backgroundBlendMode: BlendMode.luminosity,
      ),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.center,
        children: [
          leading ?? const SizedBox(width: 48, height: 48),
          const SizedBox(width: 16),
          if (heroText != null)
            Expanded(
              child: Text.rich(
                TextSpan(
                  text: heroText!.split('Lina.')[0],
                  children: [
                    TextSpan(
                      text: 'Lina.',
                      style: accentItalicTextStyle(context, fontSize: 32),
                    ),
                  ],
                  style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                        fontWeight: FontWeight.w700,
                        fontSize: 32,
                      ),
                ),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
            ),
          if (trailing != null) ...[
            const SizedBox(width: 16),
            trailing!,
          ],
        ],
      ),
    );
  }
}
