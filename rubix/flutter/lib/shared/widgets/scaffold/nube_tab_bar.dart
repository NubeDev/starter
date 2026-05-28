import 'package:flutter/material.dart';
import 'package:rubix_flutter/core/theme/app_theme.dart';

/// Bottom tab bar styled per Nube iO design system.
class NubeTabBar extends StatelessWidget {
  final int currentIndex;
  final List<NubeTabItem> items;
  final ValueChanged<int> onTap;

  const NubeTabBar({
    Key? key,
    required this.currentIndex,
    required this.items,
    required this.onTap,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    return Container(
      decoration: BoxDecoration(
        color: t.surface.withOpacity(0.85),
        border: Border(top: BorderSide(color: t.border.withOpacity(0.18))),
        boxShadow: [
          BoxShadow(
            color: Colors.black.withOpacity(0.06),
            blurRadius: 12,
            offset: const Offset(0, -2),
          ),
        ],
      ),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      child: Row(
        mainAxisAlignment: MainAxisAlignment.spaceAround,
        children: List.generate(items.length, (i) {
          final selected = i == currentIndex;
          final item = items[i];
          return GestureDetector(
            onTap: () => onTap(i),
            behavior: HitTestBehavior.opaque,
            child: AnimatedContainer(
              duration: const Duration(milliseconds: 180),
              curve: Curves.easeOut,
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              decoration: BoxDecoration(
                color: selected ? t.surface2.withOpacity(0.18) : Colors.transparent,
                borderRadius: BorderRadius.circular(16),
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(item.icon, color: selected ? t.leaf : t.muted, size: 26),
                  const SizedBox(height: 2),
                  Text(
                    item.label,
                    style: Theme.of(context).textTheme.labelMedium?.copyWith(
                          color: selected ? t.leaf : t.muted,
                          fontWeight: selected ? FontWeight.w600 : FontWeight.w400,
                        ),
                  ),
                ],
              ),
            ),
          );
        }),
      ),
    );
  }
}

class NubeTabItem {
  final IconData icon;
  final String label;
  const NubeTabItem({required this.icon, required this.label});
}
