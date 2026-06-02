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
          return _NubeTabBarItem(
            selected: selected,
            item: item,
            onTap: () => onTap(i),
          );
        }),
      ),
    );
  }
}

class _NubeTabBarItem extends StatefulWidget {
  const _NubeTabBarItem({
    required this.selected,
    required this.item,
    required this.onTap,
  });

  final bool selected;
  final NubeTabItem item;
  final VoidCallback onTap;

  @override
  State<_NubeTabBarItem> createState() => _NubeTabBarItemState();
}

class _NubeTabBarItemState extends State<_NubeTabBarItem> {
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context).nube;
    final fg = widget.selected ? t.leaf : t.muted;
    return GestureDetector(
      onTap: widget.onTap,
      behavior: HitTestBehavior.opaque,
      onTapDown: (_) => setState(() => _down = true),
      onTapUp: (_) => setState(() => _down = false),
      onTapCancel: () => setState(() => _down = false),
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 50),
        curve: Curves.easeOut,
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
        decoration: BoxDecoration(
          color: _down
              ? t.surface2.withOpacity(0.35)
              : (widget.selected ? t.surface2.withOpacity(0.18) : Colors.transparent),
          borderRadius: BorderRadius.circular(16),
        ),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(widget.item.icon, color: fg, size: 26),
            const SizedBox(height: 2),
            Text(
              widget.item.label,
              style: Theme.of(context).textTheme.labelMedium?.copyWith(
                    color: fg,
                    fontWeight:
                        widget.selected ? FontWeight.w600 : FontWeight.w400,
                  ),
            ),
          ],
        ),
      ),
    );
  }
}

class NubeTabItem {
  final IconData icon;
  final String label;
  const NubeTabItem({required this.icon, required this.label});
}
