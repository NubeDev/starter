import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';
import 'package:rubix_flutter/shared/widgets/skeleton_shimmer.dart';

/// A slim, low-key spinner — for inline / small-area loading.
///
/// For full-screen initial loads prefer [LoadingPanel], which renders a
/// skeleton silhouette of the destination screen instead of a bare
/// spinner (per `DESIGN.md` §9).
class LoadingIndicator extends StatelessWidget {
  const LoadingIndicator({super.key, this.size = 18});
  final double size;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Center(
      child: SizedBox(
        width: size,
        height: size,
        child: CircularProgressIndicator(
          strokeWidth: 2,
          color: t.leaf,
          backgroundColor: t.surface2,
        ),
      ),
    );
  }
}

/// Full-screen loading state — soft skeleton blocks shaped like the
/// destination layout. Replaces bare spinners on initial provider load.
class LoadingPanel extends StatelessWidget {
  const LoadingPanel({super.key, this.rowCount = 3});
  final int rowCount;

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      physics: const NeverScrollableScrollPhysics(),
      child: SkeletonScreen(rowCount: rowCount),
    );
  }
}
