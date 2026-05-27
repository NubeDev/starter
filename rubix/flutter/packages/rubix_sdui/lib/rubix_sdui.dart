/// rubix_sdui — public API barrel.
///
/// Import this file to use the Flutter SDUI renderer:
///
/// ```dart
/// import 'package:rubix_sdui/rubix_sdui.dart';
/// ```
///
/// Layering rule (enforced by review, not by code):
///   - `src/models/`  — pure Dart, zero Flutter imports.
///   - `src/client/`  — pure Dart, wraps the generated `rubix_api` Dio client.
///   - `src/state/`   — pure Dart, ChangeNotifier-based.
///   - `src/widgets/` — Flutter widgets.
///
/// See ../../../docs/design/sdui/renderer/FLUTTER.md for the
/// implementation plan.
library;

// models
export 'src/models/action.dart';
export 'src/models/action_response.dart';
export 'src/models/binding.dart';
export 'src/models/component.dart';
export 'src/models/component_tree.dart';
export 'src/models/diagnostic.dart';
export 'src/models/ir_version.dart';
export 'src/models/patch.dart';
export 'src/models/resolve.dart';
export 'src/models/table_query.dart';

// client
export 'src/client/sdui_service.dart';

// state
export 'src/state/sdui_notifier.dart';
export 'src/state/sdui_state.dart';
export 'src/state/sdui_status.dart';
export 'src/state/side_effect.dart';

// widgets
export 'src/widgets/components/custom_registry.dart';
export 'src/widgets/sdui_provider.dart';
export 'src/widgets/sdui_renderer.dart';
export 'src/widgets/sdui_theme.dart';
