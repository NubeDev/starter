/// Sentinel widgets — render placeholders for `dangling`,
/// `forbidden`, `custom` (when registry-miss), and unknown
/// variants. These are the only widgets that the F2 scaffold
/// ships in working form; everything else lands over F6.
library;

import 'package:flutter/material.dart';

import '../../models/component.dart';
import 'custom_registry.dart';

class SduiDanglingWidget extends StatelessWidget {
  const SduiDanglingWidget({super.key});

  @override
  Widget build(BuildContext context) => _placeholder(
        context,
        icon: Icons.link_off,
        label: 'dangling',
        tone: Theme.of(context).colorScheme.outline,
      );
}

class SduiForbiddenWidget extends StatelessWidget {
  const SduiForbiddenWidget({super.key});

  @override
  Widget build(BuildContext context) => _placeholder(
        context,
        icon: Icons.lock_outline,
        label: 'forbidden',
        tone: Theme.of(context).colorScheme.error,
      );
}

class SduiUnknownWidget extends StatelessWidget {
  const SduiUnknownWidget({super.key, required this.type});
  final String type;

  @override
  Widget build(BuildContext context) => _placeholder(
        context,
        icon: Icons.help_outline,
        label: 'unknown: $type',
        tone: Theme.of(context).colorScheme.tertiary,
      );
}

class SduiCustomWidget extends StatelessWidget {
  const SduiCustomWidget({super.key, required this.component});
  final CustomComponent component;

  @override
  Widget build(BuildContext context) {
    final id = component.rendererId;
    final builder = id == null ? null : CustomRendererRegistry.of(context)[id];
    if (builder == null) {
      return _placeholder(
        context,
        icon: Icons.extension_outlined,
        label: 'custom: ${id ?? '<no renderer_id>'}',
        tone: Theme.of(context).colorScheme.tertiary,
      );
    }
    return builder(context, component);
  }
}

Widget _placeholder(
  BuildContext context, {
  required IconData icon,
  required String label,
  required Color tone,
}) {
  return DecoratedBox(
    decoration: BoxDecoration(
      border: Border.all(color: tone.withValues(alpha: 0.4)),
      borderRadius: BorderRadius.circular(6),
    ),
    child: Padding(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 16, color: tone),
          const SizedBox(width: 8),
          Text(label, style: TextStyle(color: tone, fontSize: 12)),
        ],
      ),
    ),
  );
}
