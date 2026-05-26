/// Root wrapper around the resolved SDUI component tree.
///
/// Pure Dart — no Flutter imports.
library;

import 'component.dart';

class ComponentTree {
  const ComponentTree({required this.irVersion, required this.root});

  /// Protocol version stamped by the server.
  final int irVersion;

  /// Root component — always a `PageComponent` for normal resolve output.
  final SduiComponent root;

  factory ComponentTree.fromJson(Map<String, Object?> map) => ComponentTree(
        irVersion: (map['ir_version'] as num?)?.toInt() ?? 1,
        root: SduiComponent.fromJson(
          (map['root'] as Map).cast<String, Object?>(),
        ),
      );

  Map<String, Object?> toJson() => {
        'ir_version': irVersion,
        'root': root.toJson(),
      };
}
