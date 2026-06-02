import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:lucide_icons/lucide_icons.dart';
import 'package:provision_app/core/api/bc_types.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/core/theme/theme_providers.dart';

/// "Device provisioned!" celebration — full-bleed radial glow, a spring-popped
/// check badge, the points/widgets/alarms counts as the payoff, and a
/// deterministic sparkle burst. Ported from the React `ProvisionedReveal.tsx`.
/// Reused for the scan-flow success moment.
class ProvisionedReveal extends ConsumerStatefulWidget {
  const ProvisionedReveal({
    required this.result,
    required this.onAddAnother,
    this.onPreview,
    super.key,
  });

  final ProvisionResult result;

  /// Absent when the device was commissioned as pending (no page to view).
  final VoidCallback? onPreview;
  final VoidCallback onAddAnother;

  @override
  ConsumerState<ProvisionedReveal> createState() => _ProvisionedRevealState();
}

class _ProvisionedRevealState extends ConsumerState<ProvisionedReveal>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller;

  @override
  void initState() {
    super.initState();
    _controller = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 900),
    )..forward();
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final look = ref.watch(lookProvider);
    final accent = look.accent;
    final result = widget.result;
    // No pageId → commissioned as pending (no widgets, nothing to view yet).
    final pending = result.pageId.isEmpty;

    return DecoratedBox(
      decoration: BoxDecoration(
        gradient: RadialGradient(
          center: const Alignment(0, -0.2),
          colors: [
            accent.withValues(alpha: 0.23),
            const Color(0xF0060810),
          ],
          stops: const [0, 0.72],
        ),
      ),
      child: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 24),
          child: AnimatedBuilder(
            animation: _controller,
            builder: (context, child) {
              final t = Curves.easeOutBack.transform(
                _controller.value.clamp(0.0, 1.0),
              );
              return Opacity(
                opacity: _controller.value.clamp(0.0, 1.0),
                child: Transform.scale(
                  scale: 0.85 + 0.15 * t,
                  child: child,
                ),
              );
            },
            child: Stack(
              alignment: Alignment.center,
              clipBehavior: Clip.none,
              children: [
                for (final s in _sparks)
                  Positioned(
                    top: -120,
                    child: _Spark(
                      progress: _controller,
                      dx: s.dx,
                      dy: s.dy,
                      size: s.size,
                      color: accent,
                    ),
                  ),
                Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Container(
                      width: 80,
                      height: 80,
                      decoration: BoxDecoration(
                        color: accent,
                        shape: BoxShape.circle,
                        boxShadow: [
                          BoxShadow(
                            color: accent,
                            blurRadius: 40,
                            spreadRadius: -4,
                          ),
                        ],
                      ),
                      child: const Icon(
                        LucideIcons.checkCircle2,
                        size: 40,
                        color: Color(0xFF002019),
                      ),
                    ),
                    const SizedBox(height: 24),
                    Text(
                      pending ? 'Commissioned · pending' : 'Device provisioned',
                      style: TextStyle(
                        color: accent,
                        fontSize: 13,
                        fontWeight: FontWeight.w900,
                        letterSpacing: 3.9,
                      ),
                    ),
                    const SizedBox(height: 4),
                    Text(
                      result.deviceId,
                      textAlign: TextAlign.center,
                      style: const TextStyle(
                        color: Colors.white,
                        fontSize: 20,
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                    const SizedBox(height: 24),
                    Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        _Stat(n: result.points, label: 'points'),
                        const SizedBox(width: 12),
                        _Stat(n: result.widgets, label: 'widgets'),
                        const SizedBox(width: 12),
                        _Stat(n: result.alarms, label: 'alarms'),
                      ],
                    ),
                    if (pending) ...[
                      const SizedBox(height: 16),
                      const SizedBox(
                        width: 260,
                        child: Text(
                          'Not on a dashboard yet — place it on a page '
                          'anytime from Devices.',
                          textAlign: TextAlign.center,
                          style: TextStyle(
                            color: RubixTokens.inkMuted,
                            fontSize: 12,
                          ),
                        ),
                      ),
                    ],
                    if (result.warnings.isNotEmpty) ...[
                      const SizedBox(height: 16),
                      SizedBox(
                        width: 260,
                        child: Text(
                          result.warnings.join(' · '),
                          textAlign: TextAlign.center,
                          style: const TextStyle(
                            color: RubixTokens.coral,
                            fontSize: 12,
                          ),
                        ),
                      ),
                    ],
                    if (widget.onPreview != null) ...[
                      const SizedBox(height: 32),
                      _AccentButton(
                        accent: accent,
                        label: 'View on dashboard',
                        onTap: widget.onPreview!,
                      ),
                      const SizedBox(height: 12),
                    ] else
                      const SizedBox(height: 32),
                    GestureDetector(
                      onTap: widget.onAddAnother,
                      child: Text(
                        'Add another device',
                        style: TextStyle(
                          color: Colors.white.withValues(alpha: 0.6),
                          fontSize: 14,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _AccentButton extends StatelessWidget {
  const _AccentButton({
    required this.accent,
    required this.label,
    required this.onTap,
  });

  final Color accent;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: onTap,
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 14),
        decoration: BoxDecoration(
          color: accent,
          borderRadius: BorderRadius.circular(RubixTokens.radiusLg),
          boxShadow: [
            BoxShadow(
              color: accent.withValues(alpha: 0.5),
              blurRadius: 34,
              offset: const Offset(0, 10),
              spreadRadius: -8,
            ),
          ],
        ),
        child: Text(
          label,
          style: const TextStyle(
            color: Color(0xFF002019),
            fontSize: 16,
            fontWeight: FontWeight.w700,
          ),
        ),
      ),
    );
  }
}

class _Stat extends StatelessWidget {
  const _Stat({required this.n, required this.label});

  final int n;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Container(
      constraints: const BoxConstraints(minWidth: 72),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      decoration: BoxDecoration(
        color: Colors.white.withValues(alpha: 0.06),
        borderRadius: BorderRadius.circular(RubixTokens.radiusMd),
        border: Border.all(color: Colors.white.withValues(alpha: 0.1)),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Text(
            '$n',
            style: const TextStyle(
              color: Colors.white,
              fontSize: 20,
              fontWeight: FontWeight.w800,
            ),
          ),
          Text(label, style: RubixText.label),
        ],
      ),
    );
  }
}

/// A single sparkle: flies outward along its vector, fading in then out.
class _Spark extends StatelessWidget {
  const _Spark({
    required this.progress,
    required this.dx,
    required this.dy,
    required this.size,
    required this.color,
  });

  final Animation<double> progress;
  final double dx;
  final double dy;
  final double size;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return AnimatedBuilder(
      animation: progress,
      builder: (context, child) {
        final t = progress.value.clamp(0.0, 1.0);
        final eased = Curves.easeOut.transform(t);
        // opacity: 0 → 1 → 0
        final opacity = t < 0.5 ? t * 2 : (1 - t) * 2;
        return Transform.translate(
          offset: Offset(dx * eased, dy * eased),
          child: Opacity(
            opacity: opacity.clamp(0.0, 1.0),
            child: Container(
              width: size,
              height: size,
              decoration: BoxDecoration(
                color: color,
                shape: BoxShape.circle,
              ),
            ),
          ),
        );
      },
    );
  }
}

/// Pre-computed sparkle vectors — deterministic (no Random).
const _sparks = <_SparkVec>[
  _SparkVec(-120, -90, 10),
  _SparkVec(130, -70, 8),
  _SparkVec(-90, 60, 7),
  _SparkVec(110, 80, 11),
  _SparkVec(0, -140, 9),
  _SparkVec(-160, 10, 6),
  _SparkVec(160, 20, 8),
  _SparkVec(40, 130, 7),
  _SparkVec(-50, -120, 6),
  _SparkVec(70, -110, 9),
];

class _SparkVec {
  const _SparkVec(this.dx, this.dy, this.size);
  final double dx;
  final double dy;
  final double size;
}
