/// Sealed `SduiActionResponse` — server response to `POST /api/v1/ui/action`.
///
/// Mirrors `starter_ui_ir::ActionResponse` (R5). Variants:
///
/// - `none`              → [NoneActionResponse]
/// - `patch`             → [PatchActionResponse]
/// - `full_render`       → [FullRenderActionResponse]
/// - `navigate`          → [NavigateActionResponse]
/// - `toast`             → [ToastActionResponse]
/// - `diagnostics`       → [DiagnosticsActionResponse]
/// - `download`          → [DownloadActionResponse]
/// - `stream`            → [StreamActionResponse]
/// - `dialog`            → [DialogActionResponse]            (starter shorthand)
/// - `toast_and_refresh` → [ToastAndRefreshActionResponse]   (starter shorthand)
///
/// Pure Dart — no Flutter imports.
library;

import 'diagnostic.dart';
import 'patch.dart';

sealed class SduiActionResponse {
  const SduiActionResponse();

  factory SduiActionResponse.fromJson(Map<String, Object?> map) {
    final type = map['type'] as String? ?? '';
    return switch (type) {
      'none' => const NoneActionResponse(),
      'patch' => PatchActionResponse.fromJson(map),
      'full_render' => const FullRenderActionResponse(),
      'navigate' => NavigateActionResponse.fromJson(map),
      'toast' => ToastActionResponse.fromJson(map),
      'diagnostics' => DiagnosticsActionResponse.fromJson(map),
      'download' => DownloadActionResponse.fromJson(map),
      'stream' => StreamActionResponse.fromJson(map),
      'dialog' => DialogActionResponse.fromJson(map),
      'toast_and_refresh' => ToastAndRefreshActionResponse.fromJson(map),
      _ => const NoneActionResponse(),
    };
  }
}

final class NoneActionResponse extends SduiActionResponse {
  const NoneActionResponse();
}

final class PatchActionResponse extends SduiActionResponse {
  const PatchActionResponse({required this.patches});
  final List<SduiPatch> patches;

  factory PatchActionResponse.fromJson(Map<String, Object?> map) =>
      PatchActionResponse(
        patches: ((map['patches'] as List?) ?? const [])
            .map((e) => SduiPatch.fromJson((e as Map).cast<String, Object?>()))
            .toList(),
      );
}

final class FullRenderActionResponse extends SduiActionResponse {
  const FullRenderActionResponse();
}

final class NavigateActionResponse extends SduiActionResponse {
  const NavigateActionResponse({required this.url});
  final String url;

  factory NavigateActionResponse.fromJson(Map<String, Object?> map) =>
      NavigateActionResponse(url: map['url'] as String? ?? '');
}

final class ToastActionResponse extends SduiActionResponse {
  const ToastActionResponse({required this.message, this.intent});
  final String message;
  final String? intent;

  factory ToastActionResponse.fromJson(Map<String, Object?> map) =>
      ToastActionResponse(
        message: map['message'] as String? ?? '',
        intent: map['intent'] as String?,
      );
}

final class DiagnosticsActionResponse extends SduiActionResponse {
  const DiagnosticsActionResponse({required this.items});
  final List<SduiDiagnostic> items;

  factory DiagnosticsActionResponse.fromJson(Map<String, Object?> map) =>
      DiagnosticsActionResponse(
        items: ((map['items'] as List?) ?? const [])
            .map((e) =>
                SduiDiagnostic.fromJson((e as Map).cast<String, Object?>()))
            .toList(),
      );
}

final class DownloadActionResponse extends SduiActionResponse {
  const DownloadActionResponse({required this.url, this.filename});
  final String url;
  final String? filename;

  factory DownloadActionResponse.fromJson(Map<String, Object?> map) =>
      DownloadActionResponse(
        url: map['url'] as String? ?? '',
        filename: map['filename'] as String?,
      );
}

final class StreamActionResponse extends SduiActionResponse {
  const StreamActionResponse({required this.subject, this.targetComponentId});
  final String subject;
  final String? targetComponentId;

  factory StreamActionResponse.fromJson(Map<String, Object?> map) =>
      StreamActionResponse(
        subject: map['subject'] as String? ?? '',
        targetComponentId: map['target_component_id'] as String?,
      );
}

final class DialogActionResponse extends SduiActionResponse {
  const DialogActionResponse({required this.title, required this.body});
  final String title;
  final String body;

  factory DialogActionResponse.fromJson(Map<String, Object?> map) =>
      DialogActionResponse(
        title: map['title'] as String? ?? '',
        body: map['body'] as String? ?? '',
      );
}

final class ToastAndRefreshActionResponse extends SduiActionResponse {
  const ToastAndRefreshActionResponse({required this.message, this.intent});
  final String message;
  final String? intent;

  factory ToastAndRefreshActionResponse.fromJson(Map<String, Object?> map) =>
      ToastAndRefreshActionResponse(
        message: map['message'] as String? ?? '',
        intent: map['intent'] as String?,
      );
}
