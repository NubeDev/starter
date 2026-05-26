import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// A centered loading indicator — slim, low-key, no Material bounce.
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
