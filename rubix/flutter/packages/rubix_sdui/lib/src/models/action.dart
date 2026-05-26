/// `SduiAction` — IR-level action reference embedded in interactive components.
///
/// Mirrors `Action` in `crates/starter-ui-ir/src/action.rs`.
///
/// When the user triggers the action, the renderer POSTs an
/// `ActionRequest { handler, args, context }` to
/// `/api/v1/ui/action`; the server returns an `SduiActionResponse`.
///
/// Pure Dart — no Flutter imports.
library;

class SduiOptimisticHint {
  const SduiOptimisticHint({
    required this.targetComponentId,
    required this.fields,
  });

  final String targetComponentId;
  final Map<String, Object?> fields;

  factory SduiOptimisticHint.fromJson(Map<String, Object?> map) =>
      SduiOptimisticHint(
        targetComponentId: map['target_component_id'] as String,
        fields: (map['fields'] as Map?)?.cast<String, Object?>() ?? const {},
      );

  Map<String, Object?> toJson() => {
        'target_component_id': targetComponentId,
        'fields': fields,
      };
}

class SduiAction {
  const SduiAction({
    required this.handler,
    this.args,
    this.optimistic,
  });

  final String handler;
  final Object? args;
  final SduiOptimisticHint? optimistic;

  factory SduiAction.fromJson(Map<String, Object?> map) => SduiAction(
        handler: map['handler'] as String,
        args: map['args'],
        optimistic: map['optimistic'] == null
            ? null
            : SduiOptimisticHint.fromJson(
                (map['optimistic'] as Map).cast<String, Object?>(),
              ),
      );

  Map<String, Object?> toJson() => {
        'handler': handler,
        if (args != null) 'args': args,
        if (optimistic != null) 'optimistic': optimistic!.toJson(),
      };
}
