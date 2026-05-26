/// `/api/v1/ui/resolve` wire types + parsed result.
///
/// Mirrors `crates/starter-sdui-routes/src/routes/resolve.rs`.
///
/// Pure Dart — no Flutter imports.
library;

import 'component_tree.dart';
import 'ir_version.dart';

/// Request body for `POST /api/v1/ui/resolve`.
class ResolveRequest {
  const ResolveRequest({
    required this.pageRef,
    this.targetRef,
    this.stack = const {},
    this.pageState = const <String, Object?>{},
    this.user = const {},
    this.capabilities,
  });

  final String pageRef;
  final String? targetRef;
  final Map<String, String> stack;
  final Object pageState;
  final Map<String, Object?> user;
  final ClientCapabilities? capabilities;

  Map<String, Object?> toJson() => {
        'page_ref': pageRef,
        if (targetRef != null) 'target_ref': targetRef,
        'stack': stack,
        'page_state': pageState,
        'user': user,
        if (capabilities != null) 'capabilities': capabilities!.toJson(),
      };
}

/// Capability handshake — empty defaults pass everything through (R7).
class ClientCapabilities {
  const ClientCapabilities({
    this.irVersions = const [kSupportedIrVersion],
    this.customRenderers = const [],
  });

  final List<int> irVersions;
  final List<String> customRenderers;

  Map<String, Object?> toJson() => {
        'ir_versions': irVersions,
        'custom_renderers': customRenderers,
      };
}

/// One `(entity_id, slot)` pair the resolver touched. The renderer
/// subscribes to these to receive `slot_changed` events.
class SduiSubject {
  const SduiSubject({required this.entityId, required this.slot});

  final String entityId;
  final String slot;

  factory SduiSubject.fromJson(Map<String, Object?> map) => SduiSubject(
        entityId: map['entity_id'] as String? ?? '',
        slot: map['slot'] as String? ?? '',
      );

  Map<String, Object?> toJson() => {
        'entity_id': entityId,
        'slot': slot,
      };
}

/// Parsed result returned by `SduiService.resolve`.
class SduiResolveResult {
  const SduiResolveResult({
    required this.tree,
    required this.subscriptions,
  });

  final ComponentTree tree;
  final List<SduiSubject> subscriptions;
}

/// Thrown when the server emits an IR version newer than the client.
class SduiVersionMismatchError implements Exception {
  const SduiVersionMismatchError({
    required this.serverVersion,
    required this.supportedVersion,
  });

  final int serverVersion;
  final int supportedVersion;

  @override
  String toString() =>
      'SduiVersionMismatchError: server emitted IR v$serverVersion, '
      'client supports up to v$supportedVersion — please update the app.';
}

/// Wrapper around any error propagated from the HTTP transport.
class SduiServerError implements Exception {
  const SduiServerError(this.cause);
  final Object cause;

  @override
  String toString() => 'SduiServerError: $cause';
}
