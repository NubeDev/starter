/// Sealed `SduiSideEffect` — out-of-band events the host app
/// listens to and reflects into Flutter side-effect APIs
/// (`ScaffoldMessenger`, `go_router`, `showDialog`, `url_launcher`).
///
/// `SduiNotifier` emits these from a `Stream<SduiSideEffect>`
/// rather than reaching into `BuildContext` itself — keeps the
/// notifier pure-Dart.
///
/// Pure Dart — no Flutter imports.
library;

sealed class SduiSideEffect {
  const SduiSideEffect();
}

final class SduiToast extends SduiSideEffect {
  const SduiToast({required this.message, this.intent});
  final String message;
  final String? intent;
}

final class SduiNavigate extends SduiSideEffect {
  const SduiNavigate(this.url);
  final String url;
}

final class SduiDownload extends SduiSideEffect {
  const SduiDownload({required this.url, this.filename});
  final String url;
  final String? filename;
}

final class SduiDialog extends SduiSideEffect {
  const SduiDialog({required this.title, required this.body});
  final String title;
  final String body;
}
