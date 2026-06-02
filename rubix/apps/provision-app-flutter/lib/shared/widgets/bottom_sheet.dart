import 'package:flutter/material.dart';
import 'package:provision_app/core/theme/app_theme.dart';
import 'package:provision_app/shared/widgets/glass.dart';

/// Show the dominant modal recipe — a scrim + spring-up glass panel with a grab
/// handle, ported from the React `BottomSheet`. The [builder] content scrolls
/// inside a panel capped at 88% of the screen height.
Future<T?> showGlassBottomSheet<T>({
  required BuildContext context,
  String? title,
  required WidgetBuilder builder,
}) {
  return showModalBottomSheet<T>(
    context: context,
    isScrollControlled: true,
    barrierColor: Colors.black.withValues(alpha: 0.5),
    backgroundColor: Colors.transparent,
    builder: (ctx) => _GlassSheet(title: title, child: builder(ctx)),
  );
}

class _GlassSheet extends StatelessWidget {
  const _GlassSheet({required this.child, this.title});
  final Widget child;
  final String? title;

  @override
  Widget build(BuildContext context) {
    final media = MediaQuery.of(context);
    return Padding(
      // Lift the sheet above the keyboard when a field inside is focused.
      padding: EdgeInsets.only(bottom: media.viewInsets.bottom),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxHeight: media.size.height * 0.88),
        child: GlassSurface(
          strong: true,
          borderRadius: const BorderRadius.vertical(top: Radius.circular(32)),
          boxShadow: GlassShadow.glass,
          child: SingleChildScrollView(
            padding: EdgeInsets.fromLTRB(
              20,
              20,
              20,
              32 + media.padding.bottom,
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Center(
                  child: Container(
                    width: 40,
                    height: 6,
                    margin: const EdgeInsets.only(bottom: 16),
                    decoration: BoxDecoration(
                      color: Colors.white.withValues(alpha: 0.2),
                      borderRadius: BorderRadius.circular(999),
                    ),
                  ),
                ),
                if (title != null)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 12),
                    child: Text(title!.toUpperCase(), style: RubixText.label),
                  ),
                child,
              ],
            ),
          ),
        ),
      ),
    );
  }
}
