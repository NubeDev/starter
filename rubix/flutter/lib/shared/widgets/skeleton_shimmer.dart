import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Soft pulsing skeleton placeholder — used in place of bare spinners on
/// initial load so the screen's silhouette is recognisable while data
/// arrives.
///
/// Drives an opacity tween between `surface2` (idle) and `surface2 + leaf`
/// tinted highlight. Calm, slow (1200 ms), no Material shimmer sweep.
class SkeletonBlock extends StatefulWidget {
  const SkeletonBlock({
    super.key,
    this.height = 14,
    this.width,
    this.radius = 6,
  });

  final double height;
  final double? width;
  final double radius;

  @override
  State<SkeletonBlock> createState() => _SkeletonBlockState();
}

class _SkeletonBlockState extends State<SkeletonBlock>
    with SingleTickerProviderStateMixin {
  late final AnimationController _c;

  @override
  void initState() {
    super.initState();
    _c = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 1200),
    )..repeat(reverse: true);
  }

  @override
  void dispose() {
    _c.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return AnimatedBuilder(
      animation: _c,
      builder: (_, __) {
        final a = 0.45 + 0.25 * _c.value;
        return Container(
          height: widget.height,
          width: widget.width ?? double.infinity,
          decoration: BoxDecoration(
            color: Color.alphaBlend(
              t.leaf.withValues(alpha: 0.04 * _c.value),
              t.surface2.withValues(alpha: a),
            ),
            borderRadius: BorderRadius.circular(widget.radius),
          ),
        );
      },
    );
  }
}

/// Hero + cards skeleton matching the redesigned screens — used by Home,
/// Dashboards, Connections, Settings while their primary providers are
/// loading.
class SkeletonScreen extends StatelessWidget {
  const SkeletonScreen({super.key, this.rowCount = 3});

  final int rowCount;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.fromLTRB(20, 12, 20, 32),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const SkeletonBlock(height: 18, width: 140, radius: 999),
          const SizedBox(height: 18),
          const SkeletonBlock(height: 38, width: 260),
          const SizedBox(height: 10),
          const SkeletonBlock(height: 38, width: 200),
          const SizedBox(height: 14),
          const SkeletonBlock(height: 14, width: 300),
          const SizedBox(height: 28),
          for (var i = 0; i < rowCount; i++) ...[
            _SkeletonCard(),
            const SizedBox(height: 14),
          ],
        ],
      ),
    );
  }
}

class _SkeletonCard extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      padding: const EdgeInsets.all(18),
      decoration: BoxDecoration(
        color: t.surface,
        borderRadius: BorderRadius.circular(16),
        border: Border.all(color: t.border),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: const [
          SkeletonBlock(height: 11, width: 90),
          SizedBox(height: 14),
          SkeletonBlock(height: 28, width: 160),
          SizedBox(height: 12),
          SkeletonBlock(height: 12, width: 220),
        ],
      ),
    );
  }
}
